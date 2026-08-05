use super::{AuthSessionV1, AuthUserV1, Scope};
use crate::sync::v1::AppId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct TokenResponseV1 {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_expires_in: Option<u64>,
    pub user: AuthUserV1,
    pub session: AuthSessionV1,
    pub scopes: Vec<Scope>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct RefreshRequestV1 {
    pub refresh_token: String,
    pub app_id: AppId,
    pub device_id: String,
}
