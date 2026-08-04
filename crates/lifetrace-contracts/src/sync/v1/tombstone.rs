//! Tombstone DTO.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ids::{DeviceId, EntityId, ServerVersion};
use crate::registry::EntityType;
use crate::time::UtcTimestamp;

/// Server-generated record of a deleted entity.
///
/// Re-creating a deleted entity requires a NEW entity id. Restoring a soft
/// delete must explicitly submit based on the tombstone's latest
/// `server_version`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct TombstoneV1 {
    pub entity_type: EntityType,
    pub entity_id: EntityId,
    pub deleted_at: UtcTimestamp,
    pub server_version: ServerVersion,
    pub deleted_by_device: Option<DeviceId>,
}
