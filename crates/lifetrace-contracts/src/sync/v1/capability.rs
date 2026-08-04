//! Capabilities protocol v1.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::sync::v1::client::AppId;
use crate::time::UtcTimestamp;

/// Minimum required client version for one app id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct MinimumClientVersion {
    pub app_id: AppId,
    pub client_version: String,
}

/// `GET /api/v1/sync/capabilities` response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct CapabilitiesResponseV1 {
    pub protocol_version: u32,
    pub supported_protocol_versions: Vec<u32>,
    pub schema_version: u32,
    pub minimum_schema_version: u32,
    pub minimum_client_versions: Vec<MinimumClientVersion>,
    pub maximum_push_batch_size: u32,
    pub maximum_pull_batch_size: u32,
    pub maximum_request_bytes: u64,
    pub maximum_snapshot_page_size: u32,
    pub maximum_atomic_group_size: u32,
    pub tombstone_retention_days: u32,
    pub supported_entity_types: Vec<String>,
    pub server_time: UtcTimestamp,
}

impl CapabilitiesResponseV1 {
    /// Documented v1 defaults (server implementations may tune limits, but
    /// must always return them explicitly).
    pub fn default_v1(server_time: UtcTimestamp) -> Self {
        Self {
            protocol_version: crate::PROTOCOL_VERSION,
            supported_protocol_versions: vec![1],
            schema_version: crate::SCHEMA_VERSION,
            minimum_schema_version: crate::MINIMUM_SCHEMA_VERSION,
            minimum_client_versions: vec![MinimumClientVersion {
                app_id: AppId::new(AppId::DESKTOP),
                client_version: "0.2.1".to_owned(),
            }],
            maximum_push_batch_size: 500,
            maximum_pull_batch_size: 200,
            maximum_request_bytes: 4 * 1024 * 1024,
            maximum_snapshot_page_size: 200,
            maximum_atomic_group_size: 50,
            tombstone_retention_days: 90,
            supported_entity_types: crate::registry::REGISTRY
                .iter()
                .map(|descriptor| descriptor.entity_type.to_owned())
                .collect(),
            server_time,
        }
    }
}
