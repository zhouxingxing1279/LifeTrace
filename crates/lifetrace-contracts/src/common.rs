//! Common entity metadata shared by all public domain DTOs.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ids::{DeviceId, EntityId, ServerVersion, UserId};
use crate::time::UtcTimestamp;

/// Metadata shared by every syncable entity payload.
///
/// `local_version` is the local revision counter (EPIC-01 `version` column).
/// It is NOT a server authority; `server_version` is assigned by the server
/// and `base_server_version` (on changes) is what the client knew when it
/// made the change. Offline edits must never fabricate `server_version`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct EntityMeta {
    pub id: EntityId,
    pub user_id: UserId,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
    pub deleted_at: Option<UtcTimestamp>,
    /// Local revision number (never the server-authoritative version).
    pub local_version: u64,
    /// Server-authoritative version; `None` until the server first accepts it.
    pub server_version: Option<ServerVersion>,
    /// Device that last modified the entity locally (audit only).
    pub modified_by_device: Option<DeviceId>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::EntityId;

    #[test]
    fn entity_meta_uses_camel_case_wire_fields() {
        let stamp: UtcTimestamp = "2026-08-04T15:30:00Z".parse().unwrap();
        let meta = EntityMeta {
            id: EntityId::new("tx-1"),
            user_id: UserId::new("local-user"),
            created_at: stamp,
            updated_at: stamp,
            deleted_at: None,
            local_version: 3,
            server_version: None,
            modified_by_device: None,
        };
        let json = serde_json::to_value(&meta).unwrap();
        assert_eq!(json["id"], "tx-1");
        assert_eq!(json["userId"], "local-user");
        assert_eq!(json["localVersion"], 3);
        assert!(json.get("serverVersion").unwrap().is_null());
        assert!(json.get("modifiedByDevice").unwrap().is_null());
        assert!(json.get("createdAt").is_some());
        assert!(json.get("updatedAt").is_some());
        assert!(json.get("deletedAt").is_some());
    }
}
