use serde::{Deserialize, Serialize};

/// Strong local ownership identifier. It is not a cloud user id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalProfileId(pub String);

impl LocalProfileId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Strong cloud-account identifier obtained from the authenticated session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CloudUserId(pub String);

impl CloudUserId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncPhase {
    Disabled,
    LocalOnly,
    AuthRequired,
    Idle,
    InitializingSnapshot,
    Pushing,
    Pulling,
    UpToDate,
    Offline,
    Backoff,
    Conflict,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub phase: SyncPhase,
    pub pending_count: u64,
    pub conflict_count: u64,
    pub last_success_at: Option<String>,
    pub next_retry_at: Option<String>,
    pub last_error_code: Option<String>,
    pub last_error_message: Option<String>,
}

impl Default for SyncStatus {
    fn default() -> Self {
        Self {
            phase: SyncPhase::LocalOnly,
            pending_count: 0,
            conflict_count: 0,
            last_success_at: None,
            next_retry_at: None,
            last_error_code: None,
            last_error_message: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerContext {
    pub profile_id: LocalProfileId,
    pub cloud_user_id: Option<CloudUserId>,
    pub app_id: String,
    pub device_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOrigin {
    LocalWrite,
    RemoteSync,
    Migration,
    Import,
    ConflictResolution,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncScope {
    pub key: String,
    pub entity_types: Option<Vec<String>>,
}

impl Default for SyncScope {
    fn default() -> Self {
        Self {
            key: "all".to_owned(),
            entity_types: None,
        }
    }
}
