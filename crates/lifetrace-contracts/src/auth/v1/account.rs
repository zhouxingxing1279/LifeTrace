use super::Scope;
use crate::sync::v1::AppId;
use crate::{UserId, UtcTimestamp};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AuthUserV1 {
    pub id: UserId,
    pub email: String,
    pub display_name: Option<String>,
    pub state: String,
    pub email_verified_at: Option<UtcTimestamp>,
    pub created_at: UtcTimestamp,
    pub password_changed_at: Option<UtcTimestamp>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct RegisterRequestV1 {
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
    pub invite_token: Option<String>,
    pub app_id: AppId,
    pub device_id: String,
    pub device_name: String,
    pub platform: String,
    pub client_version: Option<String>,
    #[serde(default)]
    pub requested_scopes: Vec<Scope>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AuthCapabilitiesV1 {
    pub registration_mode: String,
    pub password_min_length: u32,
    pub password_max_bytes: u32,
    pub access_token_ttl_seconds: u64,
    pub refresh_idle_ttl_seconds: u64,
    pub refresh_absolute_ttl_seconds: u64,
    pub web_session_enabled: bool,
    pub supported_apps: Vec<AppId>,
}
