//! Identity DTOs (server managed; EPIC-04 implements registration/auth).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::common::EntityMeta;
use crate::time::UtcTimestamp;

/// `identity.user`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct User {
    pub meta: EntityMeta,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub status: String,
}

/// `identity.device`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct Device {
    pub meta: EntityMeta,
    pub device_name: String,
    pub platform: String,
    pub app_id: Option<String>,
    pub status: String,
    pub last_seen_at: Option<UtcTimestamp>,
}
