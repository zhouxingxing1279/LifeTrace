use super::{AppInstallationId, AuthSessionId, Scope};
use crate::sync::v1::AppId;
use crate::UtcTimestamp;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AuthSessionV1 {
    pub id: AuthSessionId,
    pub app_id: AppId,
    pub device_id: AppInstallationId,
    pub session_type: String,
    pub status: String,
    pub scopes: Vec<Scope>,
    pub public_device: bool,
    pub created_at: UtcTimestamp,
    pub last_seen_at: UtcTimestamp,
    pub idle_expires_at: UtcTimestamp,
    pub absolute_expires_at: UtcTimestamp,
    pub revoked_at: Option<UtcTimestamp>,
    pub current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SessionListV1 {
    pub sessions: Vec<AuthSessionV1>,
}
