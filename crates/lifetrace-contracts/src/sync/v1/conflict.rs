//! Conflict DTO.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ids::{ChangeId, ConflictId, EntityId, ServerVersion};
use crate::json_value::JsonValue;
use crate::registry::EntityType;

/// Why a conflict was returned.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConflictReason(String);

impl ConflictReason {
    pub const BASE_VERSION_MISMATCH: &'static str = "base_version_mismatch";
    pub const CLIENT_MODIFIED_SERVER_DELETED: &'static str = "client_modified_server_deleted";
    pub const CLIENT_DELETED_SERVER_MODIFIED: &'static str = "client_deleted_server_modified";
    pub const BOTH_DELETED: &'static str = "both_deleted";

    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ConflictReason {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for ConflictReason {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ConflictReason {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Ok(Self(raw))
    }
}

impl JsonSchema for ConflictReason {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("ConflictReason")
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        let mut schema = String::json_schema(generator);
        if let Some(object) = schema.as_object_mut() {
            object.insert(
                "description".to_owned(),
                serde_json::Value::String(format!(
                    "Conflict reason. Known values: {}. Unknown values are preserved.",
                    [
                        Self::BASE_VERSION_MISMATCH,
                        Self::CLIENT_MODIFIED_SERVER_DELETED,
                        Self::CLIENT_DELETED_SERVER_MODIFIED,
                        Self::BOTH_DELETED,
                    ]
                    .join(", ")
                )),
            );
        }
        schema
    }
}

impl ts_rs::TS for ConflictReason {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn decl() -> String {
        "type ConflictReason = string;".to_owned()
    }

    fn decl_concrete() -> String {
        Self::decl()
    }

    fn name() -> String {
        "ConflictReason".to_owned()
    }

    fn visit_dependencies(_visitor: &mut impl ts_rs::TypeVisitor) {}

    fn inline() -> String {
        "string".to_owned()
    }

    fn inline_flattened() -> String {
        Self::inline()
    }

    fn output_path() -> Option<std::path::PathBuf> {
        None
    }
}

/// Explicit conflict returned by the server. Resolution is client-side:
/// `keep_server`, `keep_local` (new changeId, based on the latest server
/// version) or `manual_merge`. No automatic last-write-wins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ConflictV1 {
    pub conflict_id: ConflictId,
    pub change_id: ChangeId,
    pub entity_type: EntityType,
    pub entity_id: EntityId,
    pub client_base_server_version: ServerVersion,
    pub current_server_version: ServerVersion,
    /// Current server entity (or tombstone payload) for client resolution.
    pub server_entity: Option<JsonValue>,
    pub server_deleted: bool,
    pub reason: ConflictReason,
}
