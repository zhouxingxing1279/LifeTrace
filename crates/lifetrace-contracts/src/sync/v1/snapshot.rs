//! Snapshot protocol v1.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ids::{Cursor, EntityId, RequestId, ServerVersion, SnapshotId};
use crate::json_value::JsonValue;
use crate::registry::EntityType;
use crate::sync::v1::client::SyncClientInfo;
use crate::time::UtcTimestamp;

/// `POST /api/v1/sync/snapshot` request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SnapshotRequestV1 {
    pub request_id: RequestId,
    pub client: SyncClientInfo,
    /// Omitted on the first page; the server assigns one and expects it on
    /// subsequent pages so all pages share one consistent view.
    pub snapshot_id: Option<SnapshotId>,
    pub page_token: Option<String>,
    pub entity_types: Option<Vec<EntityType>>,
    pub page_size: u32,
}

/// One current entity in a snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct EntitySnapshotV1 {
    pub entity_type: EntityType,
    pub entity_id: EntityId,
    pub server_version: ServerVersion,
    pub payload: JsonValue,
}

/// `POST /api/v1/sync/snapshot` response.
///
/// All pages of one snapshot correspond to one consistent view. After the
/// client finishes (completed == true), it sets its cursor to
/// `snapshot_cursor` and continues with Pull from there; concurrent changes
/// are then received without gaps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SnapshotResponseV1 {
    pub request_id: RequestId,
    pub snapshot_id: SnapshotId,
    pub snapshot_cursor: Cursor,
    pub items: Vec<EntitySnapshotV1>,
    pub next_page_token: Option<String>,
    pub completed: bool,
    pub server_time: UtcTimestamp,
}
