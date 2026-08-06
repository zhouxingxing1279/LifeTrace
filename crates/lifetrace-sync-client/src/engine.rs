use std::sync::Arc;
use std::time::Duration;

use lifetrace_contracts::sync::v1::SyncClientInfo;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::pull::run_pull;
use crate::push::run_push;
use crate::snapshot::run_snapshot;
use crate::{
    FailureClass, LocalProfileId, RetryPolicy, SyncError, SyncPhase, SyncScope, SyncStatus,
    SyncStore, SyncTransport,
};

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub push_batch_size: usize,
    pub pull_page_size: u32,
    pub snapshot_page_size: u32,
    pub lease_seconds: u64,
    pub retry_policy: RetryPolicy,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            push_batch_size: 100,
            pull_page_size: 100,
            snapshot_page_size: 200,
            lease_seconds: 120,
            retry_policy: RetryPolicy::default(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncRunReport {
    pub pushed: usize,
    pub pulled: usize,
    pub confirmed_by_pull: usize,
    pub conflicts: usize,
    pub snapshot_initialized: bool,
}

pub struct SyncEngine<T, S> {
    transport: Arc<T>,
    store: Arc<S>,
    client: SyncClientInfo,
    config: EngineConfig,
    run_lock: Mutex<()>,
}

impl<T: SyncTransport + 'static, S: SyncStore + 'static> SyncEngine<T, S> {
    pub fn new(
        transport: Arc<T>,
        store: Arc<S>,
        client: SyncClientInfo,
        config: EngineConfig,
    ) -> Self {
        Self {
            transport,
            store,
            client,
            config,
            run_lock: Mutex::new(()),
        }
    }

    pub async fn run_once(
        &self,
        profile: &LocalProfileId,
        scope: &SyncScope,
        initialize_snapshot: bool,
    ) -> Result<SyncRunReport, SyncError> {
        let _guard = self.run_lock.lock().await;
        if !self.store.profile_is_cloud_bound(profile).await? {
            self.update_status(profile, SyncPhase::LocalOnly, None)
                .await?;
            return Ok(SyncRunReport::default());
        }
        let mut snapshot_initialized = initialize_snapshot;
        if initialize_snapshot {
            self.initialize_snapshot(profile, scope).await?;
        }
        self.update_status(profile, SyncPhase::Pushing, None)
            .await?;
        let lease_owner = format!("{}:{}", self.client.device_id, profile.as_str());
        let leased = self
            .store
            .lease_pending(
                profile,
                &lease_owner,
                self.config.push_batch_size,
                self.config.lease_seconds,
            )
            .await?;
        let pushed = match run_push(
            self.transport.as_ref(),
            self.store.as_ref(),
            &self.client,
            &leased,
        )
        .await
        {
            Ok(value) => value,
            Err(error) => {
                self.apply_error_status(profile, &error).await?;
                return Err(error);
            }
        };
        self.update_status(profile, SyncPhase::Pulling, None)
            .await?;
        let pulled = match run_pull(
            self.transport.as_ref(),
            self.store.as_ref(),
            profile,
            &self.client,
            scope,
            self.config.pull_page_size,
        )
        .await
        {
            Ok(value) => value,
            Err(error) if requires_snapshot(&error) => {
                self.initialize_snapshot(profile, scope).await?;
                snapshot_initialized = true;
                run_pull(
                    self.transport.as_ref(),
                    self.store.as_ref(),
                    profile,
                    &self.client,
                    scope,
                    self.config.pull_page_size,
                )
                .await?
            }
            Err(error) => {
                self.apply_error_status(profile, &error).await?;
                return Err(error);
            }
        };
        let phase = if pulled.conflicts > 0 {
            SyncPhase::Conflict
        } else {
            SyncPhase::UpToDate
        };
        self.update_status(profile, phase, None).await?;
        Ok(SyncRunReport {
            pushed,
            pulled: pulled.applied,
            confirmed_by_pull: pulled.confirmed_local,
            conflicts: pulled.conflicts,
            snapshot_initialized,
        })
    }

    async fn initialize_snapshot(
        &self,
        profile: &LocalProfileId,
        scope: &SyncScope,
    ) -> Result<(), SyncError> {
        self.update_status(profile, SyncPhase::InitializingSnapshot, None)
            .await?;
        if let Err(error) = run_snapshot(
            self.transport.as_ref(),
            self.store.as_ref(),
            profile,
            &self.client,
            scope,
            self.config.snapshot_page_size,
        )
        .await
        {
            self.apply_error_status(profile, &error).await?;
            return Err(error);
        }
        Ok(())
    }

    async fn update_status(
        &self,
        profile: &LocalProfileId,
        phase: SyncPhase,
        error: Option<&SyncError>,
    ) -> Result<(), SyncError> {
        let (pending_count, conflict_count) = self.store.counts(profile).await?;
        let last_success_at = matches!(phase, SyncPhase::UpToDate | SyncPhase::Conflict)
            .then(|| chrono::Utc::now().to_rfc3339());
        let next_retry_at = error.and_then(|value| self.retry_at(value));
        self.store
            .set_status(
                profile,
                SyncStatus {
                    phase,
                    pending_count,
                    conflict_count,
                    last_success_at,
                    next_retry_at,
                    last_error_code: error.map(|value| value.code.clone()),
                    last_error_message: error.map(|value| value.message.clone()),
                },
            )
            .await
    }

    fn retry_at(&self, error: &SyncError) -> Option<String> {
        let delay = match error.class {
            FailureClass::RateLimited {
                retry_after_seconds,
            } => Duration::from_secs(retry_after_seconds.unwrap_or(2)),
            FailureClass::Offline | FailureClass::Transient => self.config.retry_policy.delay(1, 0),
            _ => return None,
        };
        Some((chrono::Utc::now() + chrono::Duration::from_std(delay).ok()?).to_rfc3339())
    }

    async fn apply_error_status(
        &self,
        profile: &LocalProfileId,
        error: &SyncError,
    ) -> Result<(), SyncError> {
        let phase = match error.class {
            FailureClass::AuthRequired => SyncPhase::AuthRequired,
            FailureClass::Offline => SyncPhase::Offline,
            FailureClass::RateLimited { .. } | FailureClass::Transient => SyncPhase::Backoff,
            FailureClass::PermissionDenied
            | FailureClass::PayloadTooLarge
            | FailureClass::UpgradeRequired
            | FailureClass::Permanent => SyncPhase::Error,
        };
        self.update_status(profile, phase, Some(error)).await
    }
}

fn requires_snapshot(error: &SyncError) -> bool {
    error.code.contains("CURSOR_EXPIRED") || error.code.contains("SNAPSHOT_REQUIRED")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_expiry_requires_snapshot_recovery() {
        let error = SyncError::new(
            "LIFETRACE_CURSOR_EXPIRED",
            "expired",
            FailureClass::Permanent,
        );
        assert!(requires_snapshot(&error));
    }
}
