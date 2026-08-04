//! Generic cross-entity link DTO.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::common::EntityMeta;
use crate::json_value::JsonValue;
use crate::registry::EntityRef;

/// `entity.link`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct EntityLink {
    pub meta: EntityMeta,
    pub source: EntityRef,
    pub target: EntityRef,
    /// Stable, extensible relation type (for example `created_from`,
    /// `references`, `attachment`, `summary_of`, `belongs_to`, `evidence_for`).
    pub relation_type: String,
    pub metadata: Option<JsonValue>,
}
