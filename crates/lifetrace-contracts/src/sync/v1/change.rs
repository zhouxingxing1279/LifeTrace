//! Sync change v1.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ids::{AtomicGroupId, ChangeId, EntityId, ServerVersion};
use crate::json_value::JsonValue;
use crate::registry::{EntityRef, EntityType};
use crate::time::UtcTimestamp;

/// Change operation. Wire enum is forward compatible (unknown values are
/// preserved; v1 only defines `upsert` and `delete`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChangeOperation(String);

impl ChangeOperation {
    pub const UPSERT: &'static str = "upsert";
    pub const DELETE: &'static str = "delete";

    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ChangeOperation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for ChangeOperation {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ChangeOperation {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Ok(Self(raw))
    }
}

impl JsonSchema for ChangeOperation {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("ChangeOperation")
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        let mut schema = String::json_schema(generator);
        if let Some(object) = schema.as_object_mut() {
            object.insert(
                "description".to_owned(),
                serde_json::Value::String(
                    "Change operation. Known values: upsert, delete. Unknown values are preserved."
                        .to_owned(),
                ),
            );
        }
        schema
    }
}

impl ts_rs::TS for ChangeOperation {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn decl() -> String {
        "type ChangeOperation = string;".to_owned()
    }

    fn decl_concrete() -> String {
        Self::decl()
    }

    fn name() -> String {
        "ChangeOperation".to_owned()
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

/// One client change. `upsert` payloads are complete entity snapshots (no
/// JSON patch). `delete` carries no payload by default; the server creates a
/// tombstone. `client_modified_at` is audit-only and never used for ordering
/// or conflict resolution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct SyncChangeV1 {
    pub change_id: ChangeId,
    pub entity_type: EntityType,
    pub entity_id: EntityId,
    pub operation: ChangeOperation,
    /// Server version the client knew when making this change (`"0"` for new
    /// entities). Never fabricated offline.
    pub base_server_version: ServerVersion,
    pub entity_schema_version: u32,
    /// Audit only. Never used for global ordering or conflict resolution.
    pub client_modified_at: UtcTimestamp,
    /// Full entity snapshot for `upsert`; empty for `delete`.
    pub payload: Option<JsonValue>,
    /// Changes sharing an atomic group must be in one request and succeed or
    /// fail together.
    pub atomic_group_id: Option<AtomicGroupId>,
    /// Entities this change depends on (must exist before applying).
    pub dependencies: Vec<EntityRef>,
}
