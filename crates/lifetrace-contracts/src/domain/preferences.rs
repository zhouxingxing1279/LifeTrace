//! User preference DTO.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::common::EntityMeta;
use crate::json_value::JsonValue;

/// `user.preference`
///
/// Only syncable preferences belong here. Secrets (API keys, refresh tokens,
/// passwords) are `secret_local_only` and MUST NOT be sent in a sync payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct UserPreference {
    pub meta: EntityMeta,
    pub preference_key: String,
    pub value: JsonValue,
}
