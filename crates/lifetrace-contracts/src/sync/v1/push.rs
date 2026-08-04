//! Push protocol v1.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{ErrorCode, FieldError};
use crate::ids::{ChangeId, ConflictId, Cursor, EntityId, RequestId, ServerVersion};
use crate::json_value::JsonValue;
use crate::registry::EntityType;
use crate::sync::v1::change::SyncChangeV1;
use crate::sync::v1::client::SyncClientInfo;
use crate::sync::v1::conflict::ConflictReason;
use crate::time::UtcTimestamp;

/// `POST /api/v1/sync/push` request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct PushRequestV1 {
    /// Tracing only; never replaces `changeId` for idempotency.
    pub request_id: RequestId,
    pub client: SyncClientInfo,
    pub changes: Vec<SyncChangeV1>,
}

/// Per-change push result. `status` is one of `accepted`, `duplicate`,
/// `conflict`, `rejected`. Business conflicts are returned per change and do
/// not fail the whole HTTP request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "status", rename_all = "snake_case")]
#[ts(tag = "status", rename_all = "snake_case")]
pub enum PushChangeResultV1 {
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Accepted {
        change_id: ChangeId,
        entity_type: EntityType,
        entity_id: EntityId,
        server_version: ServerVersion,
        cursor: Cursor,
        server_modified_at: UtcTimestamp,
    },
    /// Same changeId with identical payload was already applied; first result
    /// is returned and nothing is written twice.
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Duplicate {
        change_id: ChangeId,
        entity_type: EntityType,
        entity_id: EntityId,
        server_version: ServerVersion,
        cursor: Cursor,
        server_modified_at: UtcTimestamp,
    },
    /// `baseServerVersion` did not match the current server version, or the
    /// entity state changed in a conflicting way. Contains the current server
    /// entity/tombstone for client resolution.
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Conflict {
        conflict_id: ConflictId,
        change_id: ChangeId,
        entity_type: EntityType,
        entity_id: EntityId,
        client_base_server_version: ServerVersion,
        current_server_version: ServerVersion,
        server_entity: Option<JsonValue>,
        server_deleted: bool,
        reason: ConflictReason,
    },
    /// The change was rejected with a stable error code.
    #[serde(rename_all = "camelCase")]
    #[ts(rename_all = "camelCase")]
    Rejected {
        change_id: ChangeId,
        entity_type: EntityType,
        entity_id: EntityId,
        code: ErrorCode,
        message: String,
        #[serde(default)]
        field_errors: Vec<FieldError>,
    },
}

/// `POST /api/v1/sync/push` response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct PushResponseV1 {
    pub request_id: RequestId,
    pub server_time: UtcTimestamp,
    pub results: Vec<PushChangeResultV1>,
    pub latest_cursor: Cursor,
}
