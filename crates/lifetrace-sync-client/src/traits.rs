use async_trait::async_trait;
use lifetrace_contracts::registry::EntityType;
use lifetrace_contracts::sync::v1::{
    CapabilitiesResponseV1, PullRequestV1, PullResponseV1, PushRequestV1, PushResponseV1,
    SnapshotRequestV1, SnapshotResponseV1, SyncChangeV1,
};
use lifetrace_contracts::{Cursor, EntityId};

use crate::{
    ConflictResolution, LocalProfileId, PersistedConflict, SyncError, SyncScope, SyncStatus,
};

#[derive(Debug, Clone)]
pub struct LeasedChange {
    pub change: SyncChangeV1,
    pub local_payload_json: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApplyPageResult {
    pub applied: usize,
    pub confirmed_local: usize,
    pub conflicts: usize,
}

#[async_trait]
pub trait SyncTransport: Send + Sync {
    async fn capabilities(&self) -> Result<CapabilitiesResponseV1, SyncError>;
    async fn push(&self, request: PushRequestV1) -> Result<PushResponseV1, SyncError>;
    async fn pull(&self, request: PullRequestV1) -> Result<PullResponseV1, SyncError>;
    async fn snapshot(&self, request: SnapshotRequestV1) -> Result<SnapshotResponseV1, SyncError>;
}

#[async_trait]
pub trait SyncStore: Send + Sync {
    async fn profile_is_cloud_bound(&self, profile: &LocalProfileId) -> Result<bool, SyncError>;
    async fn lease_pending(
        &self,
        profile: &LocalProfileId,
        owner: &str,
        limit: usize,
        lease_seconds: u64,
    ) -> Result<Vec<LeasedChange>, SyncError>;
    async fn release_lease(
        &self,
        change_ids: &[String],
        error: Option<&SyncError>,
    ) -> Result<(), SyncError>;
    async fn mark_confirmed(
        &self,
        change_id: &str,
        server_version: &str,
        cursor: &str,
    ) -> Result<(), SyncError>;
    async fn mark_blocked(
        &self,
        change_id: &str,
        code: &str,
        message: &str,
    ) -> Result<(), SyncError>;
    async fn mark_dead_letter(
        &self,
        change_id: &str,
        code: &str,
        message: &str,
    ) -> Result<(), SyncError>;
    async fn persist_conflict(&self, conflict: PersistedConflict) -> Result<(), SyncError>;
    async fn cursor(
        &self,
        profile: &LocalProfileId,
        scope: &SyncScope,
    ) -> Result<Option<Cursor>, SyncError>;
    /// Applies one pull page and persists `next_cursor` in the same local
    /// transaction. Implementations must roll back the page when any entity
    /// fails, so the cursor cannot advance independently.
    async fn apply_pull_page(
        &self,
        profile: &LocalProfileId,
        scope: &SyncScope,
        response: &PullResponseV1,
    ) -> Result<ApplyPageResult, SyncError>;
    async fn begin_snapshot(
        &self,
        profile: &LocalProfileId,
        scope: &SyncScope,
    ) -> Result<(), SyncError>;
    async fn stage_snapshot_page(
        &self,
        profile: &LocalProfileId,
        scope: &SyncScope,
        response: &SnapshotResponseV1,
    ) -> Result<(), SyncError>;
    async fn finalize_snapshot(
        &self,
        profile: &LocalProfileId,
        scope: &SyncScope,
        snapshot_cursor: &Cursor,
    ) -> Result<(), SyncError>;
    async fn snapshot_resume(
        &self,
        profile: &LocalProfileId,
        scope: &SyncScope,
    ) -> Result<(Option<String>, Option<String>), SyncError>;
    async fn counts(&self, profile: &LocalProfileId) -> Result<(u64, u64), SyncError>;
    async fn set_status(
        &self,
        profile: &LocalProfileId,
        status: SyncStatus,
    ) -> Result<(), SyncError>;
    async fn list_conflicts(
        &self,
        profile: &LocalProfileId,
    ) -> Result<Vec<PersistedConflict>, SyncError>;
    async fn resolve_conflict(
        &self,
        profile: &LocalProfileId,
        conflict_id: &str,
        resolution: ConflictResolution,
    ) -> Result<(), SyncError>;
    async fn entity_has_pending_change(
        &self,
        profile: &LocalProfileId,
        entity_type: &EntityType,
        entity_id: &EntityId,
    ) -> Result<bool, SyncError>;
}

pub trait Clock: Send + Sync {
    fn now_rfc3339(&self) -> String;
}
