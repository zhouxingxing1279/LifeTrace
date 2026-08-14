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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum EntityOwnership {
    UserOwned,
    ServerManaged,
    SharedCatalog,
    DeviceLocal,
    SecretLocalOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SyncMode {
    Bidirectional,
    ServerToClient,
    ClientToServer,
    NotSynced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ConflictMode {
    Optimistic,
    ServerAuthoritative,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityDescriptor {
    pub entity_type: &'static str,
    pub schema_version: u32,
    pub ownership: EntityOwnership,
    pub sync_mode: SyncMode,
    pub conflict_mode: ConflictMode,
    pub contains_file_references: bool,
}

const fn user_owned(entity_type: &'static str, contains_file_references: bool) -> EntityDescriptor {
    EntityDescriptor {
        entity_type,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references,
    }
}

const fn server_managed(entity_type: &'static str) -> EntityDescriptor {
    EntityDescriptor {
        entity_type,
        schema_version: 1,
        ownership: EntityOwnership::ServerManaged,
        sync_mode: SyncMode::ServerToClient,
        conflict_mode: ConflictMode::ServerAuthoritative,
        contains_file_references: false,
    }
}

const fn shared_catalog(entity_type: &'static str) -> EntityDescriptor {
    EntityDescriptor {
        entity_type,
        schema_version: 1,
        ownership: EntityOwnership::SharedCatalog,
        sync_mode: SyncMode::ServerToClient,
        conflict_mode: ConflictMode::ServerAuthoritative,
        contains_file_references: false,
    }
}

/// Complete static registry. Finance entities introduced by the Android
/// bookkeeping client deliberately use the same generic LifeTrace sync store;
/// registering them here is enough for push/pull/snapshot persistence and
/// optimistic conflict handling.
pub const REGISTRY: &[EntityDescriptor] = &[
    server_managed(EntityType::IDENTITY_USER),
    server_managed(EntityType::IDENTITY_DEVICE),
    user_owned(EntityType::FINANCE_LEDGER, false),
    user_owned(EntityType::FINANCE_ACCOUNT, false),
    user_owned(EntityType::FINANCE_CATEGORY, false),
    user_owned(EntityType::FINANCE_TRANSACTION, false),
    user_owned(EntityType::FINANCE_RECURRING_TRANSACTION, false),
    user_owned(EntityType::FINANCE_TAG, false),
    user_owned(EntityType::FINANCE_TRANSACTION_TAG, false),
    user_owned(EntityType::FINANCE_BUDGET, false),
    user_owned(EntityType::FINANCE_TRANSACTION_ATTACHMENT, true),
    user_owned(EntityType::FINANCE_TRANSACTION_EVIDENCE, false),
    user_owned(EntityType::HABIT_ACTIVITY, false),
    user_owned(EntityType::HABIT_LOG, false),
    user_owned(EntityType::REVIEW_DAILY, false),
    user_owned(EntityType::NOTE_FOLDER, false),
    user_owned(EntityType::NOTE_NOTE, true),
    user_owned(EntityType::NOTE_TAG, false),
    user_owned(EntityType::NOTE_TAG_RELATION, false),
    user_owned(EntityType::NOTE_RELATION, false),
    user_owned(EntityType::NOTE_REVISION, false),
    shared_catalog(EntityType::ENGLISH_ARTICLE),
    user_owned(EntityType::ENGLISH_LEARNING_RECORD, false),
    user_owned(EntityType::ENGLISH_HIGHLIGHT, false),
    user_owned(EntityType::ENGLISH_NOTE, false),
    user_owned(EntityType::ENGLISH_VOCABULARY, false),
    user_owned(EntityType::ENGLISH_VOCABULARY_OCCURRENCE, false),
    user_owned(EntityType::ENGLISH_VOCABULARY_REVIEW_STATE, false),
    user_owned(EntityType::WORKOUT_IMPORT, false),
    user_owned(EntityType::WORKOUT_WORKOUT, false),
    user_owned(EntityType::WORKOUT_EXERCISE, false),
    user_owned(EntityType::WORKOUT_SET, false),
    user_owned(EntityType::WORKOUT_TRAINING_NOTE, false),
    user_owned(EntityType::EXECUTION_GOAL, false),
    user_owned(EntityType::EXECUTION_PROJECT, false),
    user_owned(EntityType::EXECUTION_RECURRENCE_RULE, false),
    user_owned(EntityType::EXECUTION_TASK, false),
    user_owned(EntityType::EXECUTION_TASK_DEPENDENCY, false),
    user_owned(EntityType::EXECUTION_TASK_OCCURRENCE, false),
    user_owned(EntityType::EXECUTION_WAITING_ITEM, false),
    user_owned(EntityType::EXECUTION_CALENDAR_EVENT, false),
    user_owned(EntityType::EXECUTION_CALENDAR_OCCURRENCE, false),
    user_owned(EntityType::EXECUTION_MEMO, false),
    user_owned(EntityType::EXECUTION_MEMO_TAG, false),
    user_owned(EntityType::EXECUTION_MEMO_TAG_RELATION, false),
    user_owned(EntityType::EXECUTION_REMINDER, false),
    user_owned(EntityType::EXECUTION_COMPLETION_RESULT, false),
    user_owned(EntityType::EXECUTION_ENTITY_LINK, false),
    user_owned(EntityType::FILE_METADATA, true),
    user_owned(EntityType::ENTITY_LINK, false),
    user_owned(EntityType::USER_PREFERENCE, false),
];

pub fn describe(entity_type: &str) -> Option<&'static EntityDescriptor> {
    REGISTRY.iter().find(|descriptor| descriptor.entity_type == entity_type)
}

pub fn is_syncable(entity_type: &str) -> bool {
    describe(entity_type).is_some_and(|descriptor| descriptor.sync_mode != SyncMode::NotSynced)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntityType(String);

impl EntityType {
    pub const IDENTITY_USER: &'static str = "identity.user";
    pub const IDENTITY_DEVICE: &'static str = "identity.device";
    pub const FINANCE_LEDGER: &'static str = "finance.ledger";
    pub const FINANCE_ACCOUNT: &'static str = "finance.account";
    pub const FINANCE_CATEGORY: &'static str = "finance.category";
    pub const FINANCE_TRANSACTION: &'static str = "finance.transaction";
    pub const FINANCE_RECURRING_TRANSACTION: &'static str = "finance.recurring_transaction";
    pub const FINANCE_TAG: &'static str = "finance.tag";
    pub const FINANCE_TRANSACTION_TAG: &'static str = "finance.transaction_tag";
    pub const FINANCE_BUDGET: &'static str = "finance.budget";
    pub const FINANCE_TRANSACTION_ATTACHMENT: &'static str = "finance.transaction_attachment";
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
    pub const EXECUTION_GOAL: &'static str = "execution.goal";
    pub const EXECUTION_PROJECT: &'static str = "execution.project";
    pub const EXECUTION_RECURRENCE_RULE: &'static str = "execution.recurrence_rule";
    pub const EXECUTION_TASK: &'static str = "execution.task";
    pub const EXECUTION_TASK_DEPENDENCY: &'static str = "execution.task_dependency";
    pub const EXECUTION_TASK_OCCURRENCE: &'static str = "execution.task_occurrence";
    pub const EXECUTION_WAITING_ITEM: &'static str = "execution.waiting_item";
    pub const EXECUTION_CALENDAR_EVENT: &'static str = "execution.calendar_event";
    pub const EXECUTION_CALENDAR_OCCURRENCE: &'static str = "execution.calendar_occurrence";
    pub const EXECUTION_MEMO: &'static str = "execution.memo";
    pub const EXECUTION_MEMO_TAG: &'static str = "execution.memo_tag";
    pub const EXECUTION_MEMO_TAG_RELATION: &'static str = "execution.memo_tag_relation";
    pub const EXECUTION_REMINDER: &'static str = "execution.reminder";
    pub const EXECUTION_COMPLETION_RESULT: &'static str = "execution.completion_result";
    pub const EXECUTION_ENTITY_LINK: &'static str = "execution.entity_link";
    pub const FILE_METADATA: &'static str = "file.metadata";
    pub const ENTITY_LINK: &'static str = "entity.link";
    pub const USER_PREFERENCE: &'static str = "user.preference";

    pub fn new(value: impl Into<String>) -> Self { Self(value.into()) }
    pub fn as_str(&self) -> &str { &self.0 }

    pub const fn known() -> &'static [&'static str] {
        &[
            Self::IDENTITY_USER,
            Self::IDENTITY_DEVICE,
            Self::FINANCE_LEDGER,
            Self::FINANCE_ACCOUNT,
            Self::FINANCE_CATEGORY,
            Self::FINANCE_TRANSACTION,
            Self::FINANCE_RECURRING_TRANSACTION,
            Self::FINANCE_TAG,
            Self::FINANCE_TRANSACTION_TAG,
            Self::FINANCE_BUDGET,
            Self::FINANCE_TRANSACTION_ATTACHMENT,
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
            Self::EXECUTION_GOAL,
            Self::EXECUTION_PROJECT,
            Self::EXECUTION_RECURRENCE_RULE,
            Self::EXECUTION_TASK,
            Self::EXECUTION_TASK_DEPENDENCY,
            Self::EXECUTION_TASK_OCCURRENCE,
            Self::EXECUTION_WAITING_ITEM,
            Self::EXECUTION_CALENDAR_EVENT,
            Self::EXECUTION_CALENDAR_OCCURRENCE,
            Self::EXECUTION_MEMO,
            Self::EXECUTION_MEMO_TAG,
            Self::EXECUTION_MEMO_TAG_RELATION,
            Self::EXECUTION_REMINDER,
            Self::EXECUTION_COMPLETION_RESULT,
            Self::EXECUTION_ENTITY_LINK,
            Self::FILE_METADATA,
            Self::ENTITY_LINK,
            Self::USER_PREFERENCE,
        ]
    }
}

impl fmt::Display for EntityType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result { formatter.write_str(&self.0) }
}

impl Serialize for EntityType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> { serializer.serialize_str(&self.0) }
}

impl<'de> Deserialize<'de> for EntityType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self(String::deserialize(deserializer)?))
    }
}

impl JsonSchema for EntityType {
    fn schema_name() -> Cow<'static, str> { Cow::Borrowed("EntityType") }
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
    fn decl() -> String { "type EntityType = string;".to_owned() }
    fn decl_concrete() -> String { Self::decl() }
    fn name() -> String { "EntityType".to_owned() }
    fn visit_dependencies(_visitor: &mut impl TypeVisitor) {}
    fn inline() -> String { "string".to_owned() }
    fn inline_flattened() -> String { Self::inline() }
    fn output_path() -> Option<PathBuf> { None }
}

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
    fn registry_covers_every_known_entity_type_with_unique_names() {
        let known = EntityType::known();
        assert_eq!(REGISTRY.len(), known.len());
        for expected in known {
            assert_eq!(describe(expected).unwrap().schema_version, 1);
        }
        let mut names: Vec<&str> = REGISTRY.iter().map(|item| item.entity_type).collect();
        names.sort_unstable();
        let mut unique = names.clone();
        unique.dedup();
        assert_eq!(names, unique);
    }

    #[test]
    fn finance_bookkeeping_entities_are_syncable() {
        for entity in [
            EntityType::FINANCE_LEDGER,
            EntityType::FINANCE_ACCOUNT,
            EntityType::FINANCE_CATEGORY,
            EntityType::FINANCE_TRANSACTION,
            EntityType::FINANCE_RECURRING_TRANSACTION,
            EntityType::FINANCE_TAG,
            EntityType::FINANCE_TRANSACTION_TAG,
            EntityType::FINANCE_BUDGET,
            EntityType::FINANCE_TRANSACTION_ATTACHMENT,
            EntityType::FINANCE_TRANSACTION_EVIDENCE,
        ] {
            let descriptor = describe(entity).unwrap();
            assert_eq!(descriptor.ownership, EntityOwnership::UserOwned);
            assert_eq!(descriptor.sync_mode, SyncMode::Bidirectional);
            assert_eq!(descriptor.conflict_mode, ConflictMode::Optimistic);
        }
        assert!(describe(EntityType::FINANCE_TRANSACTION_ATTACHMENT)
            .unwrap()
            .contains_file_references);
    }

    #[test]
    fn unknown_entity_types_are_not_syncable() {
        assert!(!is_syncable("secret.credential"));
        assert!(is_syncable(EntityType::FINANCE_TRANSACTION));
        assert!(is_syncable(EntityType::EXECUTION_GOAL));
    }
}
