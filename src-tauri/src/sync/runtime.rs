use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use lifetrace_contracts::auth::v1::AuthUserV1;
use lifetrace_contracts::sync::v1::{AppId, ClientPlatform, SyncClientInfo};
use lifetrace_contracts::{DeviceId, PROTOCOL_VERSION, SCHEMA_VERSION};
use lifetrace_sync_client::{
    ConflictResolution, EngineConfig, LocalProfileId, SyncEngine, SyncPhase, SyncScope, SyncStatus,
    SyncStore,
};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex as AsyncMutex, Notify, RwLock};

use crate::database::{self, profile};

use super::outbox;
use super::store::SqliteSyncStore;
use super::transport::{AuthContext, HttpSyncTransport};

#[derive(Clone)]
pub struct SyncDesktopState {
    pub data_dir: PathBuf,
    pub auth: Arc<RwLock<AuthContext>>,
    run_gate: Arc<AsyncMutex<()>>,
    wake: Arc<Notify>,
}

impl SyncDesktopState {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            auth: Arc::new(RwLock::new(AuthContext::default())),
            run_gate: Arc::new(AsyncMutex::new(())),
            wake: Arc::new(Notify::new()),
        }
    }

    pub fn signal_local_change(&self) {
        self.wake.notify_one();
    }

    pub async fn scheduler(self) {
        let mut maintenance = tokio::time::interval(Duration::from_secs(30));
        maintenance.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut last_periodic = tokio::time::Instant::now() - Duration::from_secs(300);
        loop {
            let local_change = tokio::select! {
                _ = maintenance.tick() => false,
                _ = self.wake.notified() => true,
            };
            if local_change {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            let authenticated = {
                let auth = self.auth.read().await;
                auth.access_token.is_some() && auth.cloud_user_id.is_some()
            };
            if !authenticated {
                continue;
            }
            let retry_due = status(&self)
                .ok()
                .and_then(|value| value.next_retry_at)
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(&value).ok())
                .is_some_and(|value| value.with_timezone(&chrono::Utc) <= chrono::Utc::now());
            let periodic_due = last_periodic.elapsed() >= Duration::from_secs(300);
            if local_change || retry_due || periodic_due {
                let _ = run_now(&self, false).await;
                last_periodic = tokio::time::Instant::now();
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionBindingResult {
    pub profile_id: String,
    pub cloud_user_id: String,
    pub binding_required: bool,
    pub already_bound: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatusView {
    pub profile_id: String,
    pub cloud_user_id: Option<String>,
    pub cloud_binding_state: String,
    pub phase: String,
    pub pending_count: u64,
    pub conflict_count: u64,
    pub last_success_at: Option<String>,
    pub next_retry_at: Option<String>,
    pub last_error_code: Option<String>,
    pub last_error_message: Option<String>,
}

fn open_database(data_dir: &Path) -> Result<Connection, String> {
    std::fs::create_dir_all(data_dir).map_err(|error| error.to_string())?;
    let mut connection = database::connection::open(&data_dir.join("lifetrace.db"))
        .map_err(|error| error.to_string())?;
    let context = database::migration_runner::MigrationContext::new(data_dir.to_path_buf());
    database::migration_runner::run(&mut connection, &context, &database::migrations::all())
        .map_err(|error| error.to_string())?;
    profile::ensure_active_profile(&connection)?;
    Ok(connection)
}

async fn authenticated_cloud_user(state: &SyncDesktopState) -> Result<String, String> {
    state
        .auth
        .read()
        .await
        .cloud_user_id
        .clone()
        .ok_or_else(|| "请先登录并完成服务器身份校验".to_owned())
}

async fn verify_server_identity(origin: &str, access_token: &str) -> Result<AuthUserV1, String> {
    let response = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| error.to_string())?
        .get(format!("{}/api/v1/auth/me", origin.trim_end_matches('/')))
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|error| format!("无法验证云端身份: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "云端身份验证失败: HTTP {}",
            response.status().as_u16()
        ));
    }
    response
        .json::<AuthUserV1>()
        .await
        .map_err(|error| format!("云端身份响应无效: {error}"))
}

pub async fn set_session(
    state: &SyncDesktopState,
    origin: String,
    access_token: String,
    device_id: String,
) -> Result<SessionBindingResult, String> {
    let clean_origin = origin.trim_end_matches('/').to_owned();
    let verified_user = verify_server_identity(&clean_origin, &access_token).await?;
    let cloud_user_id = verified_user.id.as_str().to_owned();
    {
        let mut auth = state.auth.write().await;
        auth.origin = clean_origin;
        auth.access_token = Some(access_token);
        auth.cloud_user_id = Some(cloud_user_id.clone());
        auth.device_id = device_id;
    }
    let connection = open_database(&state.data_dir)?;
    let profile_id = profile::active_profile_id(&connection)?;
    let bound: Option<String> = connection
        .query_row(
            "SELECT cloud_user_id FROM local_profiles WHERE id=?1",
            [&profile_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .flatten();
    let already_bound = bound.as_deref() == Some(cloud_user_id.as_str());
    let binding_required = !already_bound;
    if binding_required {
        profile::mark_pending_choice(&connection, &profile_id)?;
    }
    state.wake.notify_one();
    Ok(SessionBindingResult {
        profile_id,
        cloud_user_id,
        binding_required,
        already_bound,
    })
}

pub async fn bind_current_profile(state: &SyncDesktopState) -> Result<String, String> {
    let cloud_user_id = authenticated_cloud_user(state).await?;
    let connection = open_database(&state.data_dir)?;
    let profile_id = profile::active_profile_id(&connection)?;
    profile::bind_cloud_user(&connection, &profile_id, &cloud_user_id)?;
    outbox::enqueue_existing_profile(&connection, &profile_id)?;
    state.wake.notify_one();
    Ok(profile_id)
}

pub async fn create_cloud_profile(
    state: &SyncDesktopState,
    display_name: &str,
) -> Result<String, String> {
    let cloud_user_id = authenticated_cloud_user(state).await?;
    let connection = open_database(&state.data_dir)?;
    let created = profile::create(&connection, display_name)?;
    profile::bind_cloud_user(&connection, &created.id, &cloud_user_id)?;
    profile::set_active(&connection, &created.id)?;
    state.wake.notify_one();
    Ok(created.id)
}

pub fn list_profiles(state: &SyncDesktopState) -> Result<Vec<profile::LocalProfile>, String> {
    profile::list(&open_database(&state.data_dir)?)
}

pub fn set_active_profile(state: &SyncDesktopState, profile_id: &str) -> Result<(), String> {
    profile::set_active(&open_database(&state.data_dir)?, profile_id)?;
    state.wake.notify_one();
    Ok(())
}

pub fn status(state: &SyncDesktopState) -> Result<SyncStatusView, String> {
    let connection = open_database(&state.data_dir)?;
    let profile_id = profile::active_profile_id(&connection)?;
    let (cloud_user_id, binding_state): (Option<String>, String) = connection
        .query_row(
            "SELECT cloud_user_id,cloud_binding_state FROM local_profiles WHERE id=?1",
            [&profile_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| error.to_string())?;
    let row: Option<(
        String,
        i64,
        i64,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    )> = connection
        .query_row(
            "SELECT phase,pending_count,conflict_count,last_success_at,next_retry_at,last_error_code,last_error_message
             FROM sync_state WHERE profile_id=?1 AND scope_key='all'",
            [&profile_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?;
    let actual_pending: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sync_outbox WHERE profile_id=?1 AND status IN ('pending','leased','blocked')",
            [&profile_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let actual_conflicts: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sync_conflicts WHERE profile_id=?1 AND status='unresolved'",
            [&profile_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    let (phase, _, _, last_success, next_retry, error_code, error_message) = row.unwrap_or((
        if cloud_user_id.is_some() {
            "idle".to_owned()
        } else {
            "local_only".to_owned()
        },
        0,
        0,
        None,
        None,
        None,
        None,
    ));
    Ok(SyncStatusView {
        profile_id,
        cloud_user_id,
        cloud_binding_state: binding_state,
        phase,
        pending_count: actual_pending.max(0) as u64,
        conflict_count: actual_conflicts.max(0) as u64,
        last_success_at: last_success,
        next_retry_at: next_retry,
        last_error_code: error_code,
        last_error_message: error_message,
    })
}

pub async fn run_now(
    state: &SyncDesktopState,
    force_snapshot: bool,
) -> Result<lifetrace_sync_client::SyncRunReport, String> {
    let _run_guard = state.run_gate.lock().await;
    let auth = state.auth.read().await.clone();
    let cloud_user_id = auth
        .cloud_user_id
        .clone()
        .ok_or_else(|| "请先登录云账号".to_owned())?;
    if auth.access_token.is_none() {
        return Err("云账号会话需要恢复".to_owned());
    }
    let connection = open_database(&state.data_dir)?;
    let profile_id = profile::active_profile_id(&connection)?;
    let bound: Option<String> = connection
        .query_row(
            "SELECT cloud_user_id FROM local_profiles WHERE id=?1 AND cloud_binding_state='bound'",
            [&profile_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .flatten();
    if bound.as_deref() != Some(cloud_user_id.as_str()) {
        return Err("当前本地资料尚未明确绑定到该云账号".to_owned());
    }
    let database = Arc::new(Mutex::new(connection));
    let store = Arc::new(SqliteSyncStore::new(database, auth.device_id.clone()));
    let transport = Arc::new(HttpSyncTransport::new(state.auth.clone())?);
    let client = SyncClientInfo {
        app_id: AppId::new(AppId::DESKTOP),
        client_version: env!("CARGO_PKG_VERSION").to_owned(),
        platform: ClientPlatform::new(ClientPlatform::WINDOWS),
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        device_id: DeviceId::new(auth.device_id),
    };
    let initialize_snapshot = if force_snapshot {
        true
    } else {
        store
            .cursor(&LocalProfileId::new(&profile_id), &SyncScope::default())
            .await
            .map_err(|error| error.to_string())?
            .is_none()
    };
    let engine = SyncEngine::new(transport, store, client, EngineConfig::default());
    engine
        .run_once(
            &LocalProfileId::new(profile_id),
            &SyncScope::default(),
            initialize_snapshot,
        )
        .await
        .map_err(|error| error.to_string())
}

pub async fn conflicts(
    state: &SyncDesktopState,
) -> Result<Vec<lifetrace_sync_client::PersistedConflict>, String> {
    let auth = state.auth.read().await.clone();
    let connection = open_database(&state.data_dir)?;
    let profile_id = profile::active_profile_id(&connection)?;
    let store = SqliteSyncStore::new(Arc::new(Mutex::new(connection)), auth.device_id);
    store
        .list_conflicts(&LocalProfileId::new(profile_id))
        .await
        .map_err(|error| error.to_string())
}

pub async fn resolve_conflict(
    state: &SyncDesktopState,
    conflict_id: &str,
    resolution: ConflictResolution,
) -> Result<(), String> {
    let auth = state.auth.read().await.clone();
    let connection = open_database(&state.data_dir)?;
    let profile_id = profile::active_profile_id(&connection)?;
    let store = SqliteSyncStore::new(Arc::new(Mutex::new(connection)), auth.device_id);
    store
        .resolve_conflict(&LocalProfileId::new(profile_id), conflict_id, resolution)
        .await
        .map_err(|error| error.to_string())?;
    state.wake.notify_one();
    Ok(())
}

pub async fn mark_logged_out(state: &SyncDesktopState) -> Result<(), String> {
    {
        let mut auth = state.auth.write().await;
        auth.access_token = None;
        auth.cloud_user_id = None;
    }
    let connection = open_database(&state.data_dir)?;
    let profile_id = profile::active_profile_id(&connection)?;
    let status = SyncStatus {
        phase: SyncPhase::AuthRequired,
        ..SyncStatus::default()
    };
    let store = SqliteSyncStore::new(Arc::new(Mutex::new(connection)), "");
    store
        .set_status(&LocalProfileId::new(profile_id), status)
        .await
        .map_err(|error| error.to_string())
}
