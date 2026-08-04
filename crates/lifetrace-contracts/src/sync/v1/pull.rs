//! Pull protocol v1.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ids::{Cursor, DeviceId, EntityId, RequestId, ServerVersion};
use crate::json_value::JsonValue;
use crate::registry::EntityType;
use crate::sync::v1::change::ChangeOperation;
use crate::sync::v1::client::SyncClientInfo;
use crate::sync::v1::tombstone::TombstoneV1;
use crate::time::UtcTimestamp;

/// `POST /api/v1/sync/pull` request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct PullRequestV1 {
    pub request_id: RequestId,
    pub client: SyncClientInfo,
    /// Opaque server cursor. `None` means "start from the beginning" (may be
    /// answered with `LIFETRACE_SNAPSHOT_REQUIRED` when history is pruned).
    pub after_cursor: Option<Cursor>,
    pub limit: u32,
    /// Optional entity type filter.
    pub entity_types: Option<Vec<EntityType>>,
}

/// A change as served by pull, in strict server cursor order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ServerChangeV1 {
    pub cursor: Cursor,
    pub entity_type: EntityType,
    pub entity_id: EntityId,
    pub operation: ChangeOperation,
    pub server_version: ServerVersion,
    pub server_modified_at: UtcTimestamp,
    /// Present for `upsert`; empty for `delete`.
    pub payload: Option<JsonValue>,
    /// Present for `delete` (and for conflicts involving deletion).
    pub tombstone: Option<TombstoneV1>,
    pub origin_device_id: Option<DeviceId>,
}

/// `POST /api/v1/sync/pull` response.
///
/// Changes are ordered strictly by server cursor. Clients MUST apply them in
/// order and MUST NOT re-sort by `updatedAt`; only persist `next_cursor` after
/// the whole batch succeeded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct PullResponseV1 {
    pub request_id: RequestId,
    pub server_time: UtcTimestamp,
    pub changes: Vec<ServerChangeV1>,
    pub next_cursor: Cursor,
    pub has_more: bool,
}
