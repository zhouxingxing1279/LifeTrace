//! Entity type registry.
//!
//! `EntityType` is the stable, forward-compatible entity type name
//! (`finance.transaction`, `note.note`, ...). Unknown entity types are
//! preserved as strings so a newer client can never break a whole batch.

use std::borrow::Cow;
use std::fmt;
use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ts_rs::{TypeVisitor, TS};

use crate::ids::EntityId;

/// Stable entity type name used in sync changes and cross-entity links.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntityType(String);

impl EntityType {
    pub const IDENTITY_USER: &'static str = "identity.user";
    pub const IDENTITY_DEVICE: &'static str = "identity.device";
    pub const FINANCE_ACCOUNT: &'static str = "finance.account";
    pub const FINANCE_CATEGORY: &'static str = "finance.category";
    pub const FINANCE_TRANSACTION: &'static str = "finance.transaction";
    pub const FINANCE_TRANSACTION_EVIDENCE: &'static str = "finance.transaction_evidence";
    pub const HABIT_ACTIVITY: &'static str = "habit.activity";
    pub const HABIT_LOG: &'static str = "habit.log";
    pub const REVIEW_DAILY: &'static str = "review.daily";
    pub const NOTE_FOLDER: &'static str = "note.folder";
    pub const NOTE_NOTE: &'static str = "note.note";
    pub const NOTE_TAG: &'static str = "note.tag";
    pub const NOTE_TAG_RELATION: &'static str = "note.tag_relation";
    pub const NOTE_RELATION: &'static str = "note.relation";
    pub const NOTE_REVISION: &'static str = "note.revision";
    pub const ENGLISH_ARTICLE: &'static str = "english.article";
    pub const ENGLISH_LEARNING_RECORD: &'static str = "english.learning_record";
    pub const ENGLISH_HIGHLIGHT: &'static str = "english.highlight";
    pub const ENGLISH_NOTE: &'static str = "english.note";
    pub const ENGLISH_VOCABULARY: &'static str = "english.vocabulary";
    pub const ENGLISH_VOCABULARY_OCCURRENCE: &'static str = "english.vocabulary_occurrence";
    pub const ENGLISH_VOCABULARY_REVIEW_STATE: &'static str = "english.vocabulary_review_state";
    pub const WORKOUT_IMPORT: &'static str = "workout.import";
    pub const WORKOUT_WORKOUT: &'static str = "workout.workout";
    pub const WORKOUT_EXERCISE: &'static str = "workout.exercise";
    pub const WORKOUT_SET: &'static str = "workout.set";
    pub const WORKOUT_TRAINING_NOTE: &'static str = "workout.training_note";
    pub const FILE_METADATA: &'static str = "file.metadata";
    pub const ENTITY_LINK: &'static str = "entity.link";
    pub const USER_PREFERENCE: &'static str = "user.preference";

    /// Wrap any entity type string. Unknown values are preserved.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// All known entity type names (registry in phase 3 adds descriptors).
    pub const fn known() -> &'static [&'static str] {
        &[
            Self::IDENTITY_USER,
            Self::IDENTITY_DEVICE,
            Self::FINANCE_ACCOUNT,
            Self::FINANCE_CATEGORY,
            Self::FINANCE_TRANSACTION,
            Self::FINANCE_TRANSACTION_EVIDENCE,
            Self::HABIT_ACTIVITY,
            Self::HABIT_LOG,
            Self::REVIEW_DAILY,
            Self::NOTE_FOLDER,
            Self::NOTE_NOTE,
            Self::NOTE_TAG,
            Self::NOTE_TAG_RELATION,
            Self::NOTE_RELATION,
            Self::NOTE_REVISION,
            Self::ENGLISH_ARTICLE,
            Self::ENGLISH_LEARNING_RECORD,
            Self::ENGLISH_HIGHLIGHT,
            Self::ENGLISH_NOTE,
            Self::ENGLISH_VOCABULARY,
            Self::ENGLISH_VOCABULARY_OCCURRENCE,
            Self::ENGLISH_VOCABULARY_REVIEW_STATE,
            Self::WORKOUT_IMPORT,
            Self::WORKOUT_WORKOUT,
            Self::WORKOUT_EXERCISE,
            Self::WORKOUT_SET,
            Self::WORKOUT_TRAINING_NOTE,
            Self::FILE_METADATA,
            Self::ENTITY_LINK,
            Self::USER_PREFERENCE,
        ]
    }
}

impl fmt::Display for EntityType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for EntityType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for EntityType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Ok(Self(raw))
    }
}

impl JsonSchema for EntityType {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("EntityType")
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        let mut schema = String::json_schema(generator);
        if let Some(object) = schema.as_object_mut() {
            object.insert(
                "description".to_owned(),
                serde_json::Value::String(format!(
                    "Registered entity type. Known values: {}. Unknown values are preserved.",
                    EntityType::known().join(", ")
                )),
            );
        }
        schema
    }
}

impl TS for EntityType {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn decl() -> String {
        "type EntityType = string;".to_owned()
    }

    fn decl_concrete() -> String {
        Self::decl()
    }

    fn name() -> String {
        "EntityType".to_owned()
    }

    fn visit_dependencies(_visitor: &mut impl TypeVisitor) {}

    fn inline() -> String {
        "string".to_owned()
    }

    fn inline_flattened() -> String {
        Self::inline()
    }

    fn output_path() -> Option<PathBuf> {
        None
    }
}

/// A typed reference to another syncable entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct EntityRef {
    pub entity_type: EntityType,
    pub entity_id: EntityId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_type_round_trips_known_and_unknown() {
        let known = EntityType::new(EntityType::FINANCE_TRANSACTION);
        let json = serde_json::to_string(&known).unwrap();
        assert_eq!(json, "\"finance.transaction\"");
        let back: EntityType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, known);

        let unknown: EntityType = serde_json::from_str("\"future.thing\"").unwrap();
        assert_eq!(unknown.as_str(), "future.thing");
    }

    #[test]
    fn entity_ref_uses_camel_case() {
        let reference = EntityRef {
            entity_type: EntityType::new(EntityType::FINANCE_TRANSACTION),
            entity_id: EntityId::new("tx-1"),
        };
        let value = serde_json::to_value(&reference).unwrap();
        assert_eq!(value["entityType"], "finance.transaction");
        assert_eq!(value["entityId"], "tx-1");
    }
}
