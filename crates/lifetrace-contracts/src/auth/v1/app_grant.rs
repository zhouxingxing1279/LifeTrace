use super::{AppGrantId, Scope};
use crate::sync::v1::AppId;
use crate::UtcTimestamp;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AppGrantV1 {
    pub id: AppGrantId,
    pub app_id: AppId,
    pub scopes: Vec<Scope>,
    pub status: String,
    pub granted_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
    pub revoked_at: Option<UtcTimestamp>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct AppGrantListV1 {
    pub grants: Vec<AppGrantV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UpdateAppGrantRequestV1 {
    pub scopes: Vec<Scope>,
}
