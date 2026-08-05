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

/// Entity ownership class.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, JsonSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum EntityOwnership {
    /// Owned and edited by the user; bidirectional sync with conflict checks.
    UserOwned,
    /// Managed by the server (for example identity.user, identity.device).
    ServerManaged,
    /// Global catalog shared by all users (for example english.article).
    SharedCatalog,
    /// Only meaningful on the device that created it; never synced.
    DeviceLocal,
    /// Credentials and secrets; MUST NEVER enter a sync payload.
    SecretLocalOnly,
}

/// Entity sync mode.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, JsonSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum SyncMode {
    Bidirectional,
    ServerToClient,
    ClientToServer,
    NotSynced,
}

/// Entity conflict handling mode.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, JsonSchema, TS,
)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ConflictMode {
    /// Client `baseServerVersion` must equal the current server version;
    /// otherwise an explicit conflict is returned. No automatic last-write-wins.
    Optimistic,
    /// The server is authoritative; client writes are rejected or ignored.
    ServerAuthoritative,
    /// No conflict semantics (device-local / not synced data).
    None,
}

/// Static registry entry for one entity type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityDescriptor {
    pub entity_type: &'static str,
    pub schema_version: u32,
    pub ownership: EntityOwnership,
    pub sync_mode: SyncMode,
    pub conflict_mode: ConflictMode,
    /// Whether the entity payload references files (binary content is synced
    /// separately by EPIC-12; only metadata/IDs cross the wire).
    pub contains_file_references: bool,
}

/// Complete static entity type registry.
///
/// Device-local and secret-local-only data (photos, AI/translation settings,
/// certificates, import uploads, ...) are intentionally NOT registered here:
/// unknown entity types are rejected by the sync protocol with
/// `LIFETRACE_UNKNOWN_ENTITY_TYPE`, giving defense in depth against secrets
/// entering a sync payload.
pub const REGISTRY: &[EntityDescriptor] = &[
    EntityDescriptor {
        entity_type: EntityType::IDENTITY_USER,
        schema_version: 1,
        ownership: EntityOwnership::ServerManaged,
        sync_mode: SyncMode::ServerToClient,
        conflict_mode: ConflictMode::ServerAuthoritative,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::IDENTITY_DEVICE,
        schema_version: 1,
        ownership: EntityOwnership::ServerManaged,
        sync_mode: SyncMode::ServerToClient,
        conflict_mode: ConflictMode::ServerAuthoritative,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::FINANCE_ACCOUNT,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::FINANCE_CATEGORY,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::FINANCE_TRANSACTION,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::FINANCE_TRANSACTION_EVIDENCE,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::HABIT_ACTIVITY,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::HABIT_LOG,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::REVIEW_DAILY,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::NOTE_FOLDER,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::NOTE_NOTE,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: true,
    },
    EntityDescriptor {
        entity_type: EntityType::NOTE_TAG,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::NOTE_TAG_RELATION,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::NOTE_RELATION,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::NOTE_REVISION,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::ENGLISH_ARTICLE,
        schema_version: 1,
        ownership: EntityOwnership::SharedCatalog,
        sync_mode: SyncMode::ServerToClient,
        conflict_mode: ConflictMode::ServerAuthoritative,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::ENGLISH_LEARNING_RECORD,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::ENGLISH_HIGHLIGHT,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::ENGLISH_NOTE,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::ENGLISH_VOCABULARY,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::ENGLISH_VOCABULARY_OCCURRENCE,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::ENGLISH_VOCABULARY_REVIEW_STATE,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::WORKOUT_IMPORT,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::WORKOUT_WORKOUT,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::WORKOUT_EXERCISE,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::WORKOUT_SET,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::WORKOUT_TRAINING_NOTE,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::FILE_METADATA,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: true,
    },
    EntityDescriptor {
        entity_type: EntityType::ENTITY_LINK,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: false,
    },
    EntityDescriptor {
        entity_type: EntityType::USER_PREFERENCE,
        schema_version: 1,
        ownership: EntityOwnership::UserOwned,
        sync_mode: SyncMode::Bidirectional,
        conflict_mode: ConflictMode::Optimistic,
        contains_file_references: false,
    },
];

/// Look up a registered descriptor by entity type name.
pub fn describe(entity_type: &str) -> Option<&'static EntityDescriptor> {
    REGISTRY
        .iter()
        .find(|descriptor| descriptor.entity_type == entity_type)
}

/// Whether an entity type is registered and allowed to sync.
pub fn is_syncable(entity_type: &str) -> bool {
    describe(entity_type).is_some_and(|descriptor| descriptor.sync_mode != SyncMode::NotSynced)
}

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

    #[test]
    fn registry_covers_every_known_entity_type_with_unique_names() {
        let known = EntityType::known();
        assert_eq!(REGISTRY.len(), known.len());
        for expected in known {
            let descriptor = describe(expected)
                .unwrap_or_else(|| panic!("entity type {expected} must be registered"));
            assert_eq!(descriptor.entity_type, *expected);
            assert_eq!(descriptor.schema_version, 1);
        }
        let mut names: Vec<&str> = REGISTRY.iter().map(|item| item.entity_type).collect();
        names.sort_unstable();
        let mut unique = names.clone();
        unique.dedup();
        assert_eq!(names, unique, "entity type names must be unique");
    }

    #[test]
    fn every_entity_has_ownership_sync_and_conflict_modes() {
        for descriptor in REGISTRY {
            assert!(matches!(
                descriptor.ownership,
                EntityOwnership::UserOwned
                    | EntityOwnership::ServerManaged
                    | EntityOwnership::SharedCatalog
                    | EntityOwnership::DeviceLocal
                    | EntityOwnership::SecretLocalOnly
            ));
            assert!(matches!(
                descriptor.sync_mode,
                SyncMode::Bidirectional
                    | SyncMode::ServerToClient
                    | SyncMode::ClientToServer
                    | SyncMode::NotSynced
            ));
            assert!(matches!(
                descriptor.conflict_mode,
                ConflictMode::Optimistic | ConflictMode::ServerAuthoritative | ConflictMode::None
            ));
        }
    }

    #[test]
    fn user_owned_entities_use_optimistic_conflict_never_lww() {
        for descriptor in REGISTRY {
            if descriptor.ownership == EntityOwnership::UserOwned {
                assert_eq!(descriptor.conflict_mode, ConflictMode::Optimistic);
                assert_eq!(descriptor.sync_mode, SyncMode::Bidirectional);
            }
        }
    }

    #[test]
    fn unknown_entity_types_are_not_syncable() {
        assert!(!is_syncable("secret.credential"));
        assert!(!is_syncable(""));
        assert!(is_syncable(EntityType::FINANCE_TRANSACTION));
        assert!(is_syncable(EntityType::ENGLISH_ARTICLE));
    }

    #[test]
    fn file_reference_flags_are_set_where_expected() {
        assert!(
            describe(EntityType::NOTE_NOTE)
                .unwrap()
                .contains_file_references
        );
        assert!(
            describe(EntityType::FILE_METADATA)
                .unwrap()
                .contains_file_references
        );
        assert!(
            !describe(EntityType::FINANCE_TRANSACTION)
                .unwrap()
                .contains_file_references
        );
    }

    #[test]
    fn all_five_ownership_classes_are_representable() {
        let classes = [
            EntityOwnership::UserOwned,
            EntityOwnership::ServerManaged,
            EntityOwnership::SharedCatalog,
            EntityOwnership::DeviceLocal,
            EntityOwnership::SecretLocalOnly,
        ];
        let wire = classes
            .iter()
            .map(|value| serde_json::to_string(value).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            wire,
            vec![
                "\"user_owned\"",
                "\"server_managed\"",
                "\"shared_catalog\"",
                "\"device_local\"",
                "\"secret_local_only\"",
            ]
        );
    }
}
