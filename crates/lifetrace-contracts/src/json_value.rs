//! A serde_json::Value wrapper that also implements `JsonSchema` and `TS`,
//! used for opaque payload fields (entity payloads, error details, metadata).

use std::borrow::Cow;
use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::{TypeVisitor, TS};

/// Opaque JSON value. Serializes transparently; generated TypeScript type is
/// `unknown` because the shape is validated by the entity schema at runtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JsonValue(pub serde_json::Value);

impl From<serde_json::Value> for JsonValue {
    fn from(value: serde_json::Value) -> Self {
        Self(value)
    }
}

impl From<JsonValue> for serde_json::Value {
    fn from(value: JsonValue) -> Self {
        value.0
    }
}

impl TS for JsonValue {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn decl() -> String {
        "type JsonValue = unknown;".to_owned()
    }

    fn decl_concrete() -> String {
        Self::decl()
    }

    fn name() -> String {
        "JsonValue".to_owned()
    }

    fn visit_dependencies(_visitor: &mut impl TypeVisitor) {}

    fn inline() -> String {
        "unknown".to_owned()
    }

    fn inline_flattened() -> String {
        Self::inline()
    }

    fn output_path() -> Option<PathBuf> {
        None
    }
}

impl JsonSchema for JsonValue {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("JsonValue")
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        <serde_json::Value as JsonSchema>::json_schema(generator)
    }
}
