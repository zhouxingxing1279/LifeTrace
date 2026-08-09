//! Typed domain payload dispatch.
//!
//! Change payloads travel as opaque JSON on the wire; `EntityPayload` is a
//! Rust-side convenience for validation, adapters and the reference
//! implementation. It is dispatched through the entity registry.

use crate::domain::english::*;
use crate::domain::files::FileMetadata;
use crate::domain::finance::*;
use crate::domain::habits::*;
use crate::domain::links::EntityLink;
use crate::domain::notes::*;
use crate::domain::preferences::UserPreference;
use crate::domain::reviews::DailyReview;
use crate::domain::user::{Device, User};
use crate::domain::workouts::*;
use crate::ids::EntityId;
use crate::json_value::JsonValue;
use crate::registry::EntityType;

/// All supported domain payloads, keyed by registered entity type.
///
/// This is a stable Rust-side protocol dispatch API. Boxing selected variants
/// would impose a source-breaking constructor change across every adapter for
/// no wire-format benefit, so its deliberately heterogeneous size is accepted.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum EntityPayload {
    User(User),
    Device(Device),
    FinanceAccount(FinanceAccount),
    TransactionCategory(TransactionCategory),
    Transaction(Transaction),
    TransactionEvidence(TransactionEvidence),
    Activity(Activity),
    ActivityLog(ActivityLog),
    DailyReview(DailyReview),
    NoteFolder(NoteFolder),
    Note(Note),
    NoteTag(NoteTag),
    NoteTagRelation(NoteTagRelation),
    NoteRelation(NoteRelation),
    NoteRevision(NoteRevision),
    EnglishArticle(EnglishArticle),
    EnglishLearningRecord(EnglishLearningRecord),
    EnglishHighlight(EnglishHighlight),
    EnglishNote(EnglishNote),
    EnglishVocabulary(EnglishVocabulary),
    VocabularyOccurrence(VocabularyOccurrence),
    VocabularyReviewState(VocabularyReviewState),
    WorkoutImport(WorkoutImport),
    Workout(Workout),
    WorkoutExercise(WorkoutExercise),
    WorkoutSet(WorkoutSet),
    TrainingNote(TrainingNote),
    FileMetadata(FileMetadata),
    EntityLink(EntityLink),
    UserPreference(UserPreference),
    RegisteredJson {
        entity_type: &'static str,
        entity_id: EntityId,
        payload: JsonValue,
    },
}

impl EntityPayload {
    /// The registered entity type of this payload.
    pub fn entity_type(&self) -> EntityType {
        EntityType::new(match self {
            EntityPayload::User(_) => EntityType::IDENTITY_USER,
            EntityPayload::Device(_) => EntityType::IDENTITY_DEVICE,
            EntityPayload::FinanceAccount(_) => EntityType::FINANCE_ACCOUNT,
            EntityPayload::TransactionCategory(_) => EntityType::FINANCE_CATEGORY,
            EntityPayload::Transaction(_) => EntityType::FINANCE_TRANSACTION,
            EntityPayload::TransactionEvidence(_) => EntityType::FINANCE_TRANSACTION_EVIDENCE,
            EntityPayload::Activity(_) => EntityType::HABIT_ACTIVITY,
            EntityPayload::ActivityLog(_) => EntityType::HABIT_LOG,
            EntityPayload::DailyReview(_) => EntityType::REVIEW_DAILY,
            EntityPayload::NoteFolder(_) => EntityType::NOTE_FOLDER,
            EntityPayload::Note(_) => EntityType::NOTE_NOTE,
            EntityPayload::NoteTag(_) => EntityType::NOTE_TAG,
            EntityPayload::NoteTagRelation(_) => EntityType::NOTE_TAG_RELATION,
            EntityPayload::NoteRelation(_) => EntityType::NOTE_RELATION,
            EntityPayload::NoteRevision(_) => EntityType::NOTE_REVISION,
            EntityPayload::EnglishArticle(_) => EntityType::ENGLISH_ARTICLE,
            EntityPayload::EnglishLearningRecord(_) => EntityType::ENGLISH_LEARNING_RECORD,
            EntityPayload::EnglishHighlight(_) => EntityType::ENGLISH_HIGHLIGHT,
            EntityPayload::EnglishNote(_) => EntityType::ENGLISH_NOTE,
            EntityPayload::EnglishVocabulary(_) => EntityType::ENGLISH_VOCABULARY,
            EntityPayload::VocabularyOccurrence(_) => EntityType::ENGLISH_VOCABULARY_OCCURRENCE,
            EntityPayload::VocabularyReviewState(_) => EntityType::ENGLISH_VOCABULARY_REVIEW_STATE,
            EntityPayload::WorkoutImport(_) => EntityType::WORKOUT_IMPORT,
            EntityPayload::Workout(_) => EntityType::WORKOUT_WORKOUT,
            EntityPayload::WorkoutExercise(_) => EntityType::WORKOUT_EXERCISE,
            EntityPayload::WorkoutSet(_) => EntityType::WORKOUT_SET,
            EntityPayload::TrainingNote(_) => EntityType::WORKOUT_TRAINING_NOTE,
            EntityPayload::FileMetadata(_) => EntityType::FILE_METADATA,
            EntityPayload::EntityLink(_) => EntityType::ENTITY_LINK,
            EntityPayload::UserPreference(_) => EntityType::USER_PREFERENCE,
            EntityPayload::RegisteredJson { entity_type, .. } => *entity_type,
        })
    }

    /// The entity id embedded in the payload meta.
    pub fn entity_id(&self) -> &EntityId {
        match self {
            EntityPayload::User(value) => &value.meta.id,
            EntityPayload::Device(value) => &value.meta.id,
            EntityPayload::FinanceAccount(value) => &value.meta.id,
            EntityPayload::TransactionCategory(value) => &value.meta.id,
            EntityPayload::Transaction(value) => &value.meta.id,
            EntityPayload::TransactionEvidence(value) => &value.meta.id,
            EntityPayload::Activity(value) => &value.meta.id,
            EntityPayload::ActivityLog(value) => &value.meta.id,
            EntityPayload::DailyReview(value) => &value.meta.id,
            EntityPayload::NoteFolder(value) => &value.meta.id,
            EntityPayload::Note(value) => &value.meta.id,
            EntityPayload::NoteTag(value) => &value.meta.id,
            EntityPayload::NoteTagRelation(value) => &value.meta.id,
            EntityPayload::NoteRelation(value) => &value.meta.id,
            EntityPayload::NoteRevision(value) => &value.meta.id,
            EntityPayload::EnglishArticle(value) => &value.meta.id,
            EntityPayload::EnglishLearningRecord(value) => &value.meta.id,
            EntityPayload::EnglishHighlight(value) => &value.meta.id,
            EntityPayload::EnglishNote(value) => &value.meta.id,
            EntityPayload::EnglishVocabulary(value) => &value.meta.id,
            EntityPayload::VocabularyOccurrence(value) => &value.meta.id,
            EntityPayload::VocabularyReviewState(value) => &value.meta.id,
            EntityPayload::WorkoutImport(value) => &value.meta.id,
            EntityPayload::Workout(value) => &value.meta.id,
            EntityPayload::WorkoutExercise(value) => &value.meta.id,
            EntityPayload::WorkoutSet(value) => &value.meta.id,
            EntityPayload::TrainingNote(value) => &value.meta.id,
            EntityPayload::FileMetadata(value) => &value.meta.id,
            EntityPayload::EntityLink(value) => &value.meta.id,
            EntityPayload::UserPreference(value) => &value.meta.id,
            EntityPayload::RegisteredJson { entity_id, .. } => entity_id,
        }
    }

    /// Serialize this payload back to wire JSON.
    pub fn to_json(&self) -> JsonValue {
        match self {
            EntityPayload::User(value) => serde_json::to_value(value).unwrap().into(),
            EntityPayload::Device(value) => serde_json::to_value(value).unwrap().into(),
            EntityPayload::FinanceAccount(value) => serde_json::to_value(value).unwrap().into(),
            EntityPayload::TransactionCategory(value) => {
                serde_json::to_value(value).unwrap().into()
            }
            EntityPayload::Transaction(value) => serde_json::to_value(value).unwrap().into(),
            EntityPayload::TransactionEvidence(value) => {
                serde_json::to_value(value).unwrap().into()
            }
            EntityPayload::Activity(value) => serde_json::to_value(value).unwrap().into(),
            EntityPayload::ActivityLog(value) => serde_json::to_value(value).unwrap().into(),
            EntityPayload::DailyReview(value) => serde_json::to_value(value).unwrap().into(),
            EntityPayload::NoteFolder(value) => serde_json::to_value(value).unwrap().into(),
            EntityPayload::Note(value) => serde_json::to_value(value).unwrap().into(),
            EntityPayload::NoteTag(value) => serde_json::to_value(value).unwrap().into(),
            EntityPayload::NoteTagRelation(value) => serde_json::to_value(value).unwrap().into(),
            EntityPayload::NoteRelation(value) => serde_json::to_value(value).unwrap().into(),
            EntityPayload::NoteRevision(value) => serde_json::to_value(value).unwrap().into(),
            EntityPayload::EnglishArticle(value) => serde_json::to_value(value).unwrap().into(),
            EntityPayload::EnglishLearningRecord(value) => {
                serde_json::to_value(value).unwrap().into()
            }
            EntityPayload::EnglishHighlight(value) => serde_json::to_value(value).unwrap().into(),
            EntityPayload::EnglishNote(value) => serde_json::to_value(value).unwrap().into(),
            EntityPayload::EnglishVocabulary(value) => serde_json::to_value(value).unwrap().into(),
            EntityPayload::VocabularyOccurrence(value) => {
                serde_json::to_value(value).unwrap().into()
            }
            EntityPayload::VocabularyReviewState(value) => {
                serde_json::to_value(value).unwrap().into()
            }
            EntityPayload::WorkoutImport(value) => serde_json::to_value(value).unwrap().into(),
            EntityPayload::Workout(value) => serde_json::to_value(value).unwrap().into(),
            EntityPayload::WorkoutExercise(value) => serde_json::to_value(value).unwrap().into(),
            EntityPayload::WorkoutSet(value) => serde_json::to_value(value).unwrap().into(),
            EntityPayload::TrainingNote(value) => serde_json::to_value(value).unwrap().into(),
            EntityPayload::FileMetadata(value) => serde_json::to_value(value).unwrap().into(),
            EntityPayload::EntityLink(value) => serde_json::to_value(value).unwrap().into(),
            EntityPayload::UserPreference(value) => serde_json::to_value(value).unwrap().into(),
            EntityPayload::RegisteredJson { payload, .. } => payload.clone(),
        }
    }
}

impl TryFrom<(&EntityType, JsonValue)> for EntityPayload {
    type Error = String;

    fn try_from((entity_type, value): (&EntityType, JsonValue)) -> Result<Self, Self::Error> {
        fn parse<T: serde::de::DeserializeOwned>(
            value: &JsonValue,
            name: &'static str,
        ) -> Result<T, String> {
            serde_json::from_value(value.0.clone())
                .map_err(|error| format!("invalid {name} payload: {error}"))
        }
        match entity_type.as_str() {
            EntityType::IDENTITY_USER => {
                parse::<User>(&value, "identity.user").map(EntityPayload::User)
            }
            EntityType::IDENTITY_DEVICE => {
                parse::<Device>(&value, "identity.device").map(EntityPayload::Device)
            }
            EntityType::FINANCE_ACCOUNT => parse::<FinanceAccount>(&value, "finance.account")
                .map(EntityPayload::FinanceAccount),
            EntityType::FINANCE_CATEGORY => {
                parse::<TransactionCategory>(&value, "finance.category")
                    .map(EntityPayload::TransactionCategory)
            }
            EntityType::FINANCE_TRANSACTION => {
                parse::<Transaction>(&value, "finance.transaction").map(EntityPayload::Transaction)
            }
            EntityType::FINANCE_TRANSACTION_EVIDENCE => {
                parse::<TransactionEvidence>(&value, "finance.transaction_evidence")
                    .map(EntityPayload::TransactionEvidence)
            }
            EntityType::HABIT_ACTIVITY => {
                parse::<Activity>(&value, "habit.activity").map(EntityPayload::Activity)
            }
            EntityType::HABIT_LOG => {
                parse::<ActivityLog>(&value, "habit.log").map(EntityPayload::ActivityLog)
            }
            EntityType::REVIEW_DAILY => {
                parse::<DailyReview>(&value, "review.daily").map(EntityPayload::DailyReview)
            }
            EntityType::NOTE_FOLDER => {
                parse::<NoteFolder>(&value, "note.folder").map(EntityPayload::NoteFolder)
            }
            EntityType::NOTE_NOTE => parse::<Note>(&value, "note.note").map(EntityPayload::Note),
            EntityType::NOTE_TAG => {
                parse::<NoteTag>(&value, "note.tag").map(EntityPayload::NoteTag)
            }
            EntityType::NOTE_TAG_RELATION => parse::<NoteTagRelation>(&value, "note.tag_relation")
                .map(EntityPayload::NoteTagRelation),
            EntityType::NOTE_RELATION => {
                parse::<NoteRelation>(&value, "note.relation").map(EntityPayload::NoteRelation)
            }
            EntityType::NOTE_REVISION => {
                parse::<NoteRevision>(&value, "note.revision").map(EntityPayload::NoteRevision)
            }
            EntityType::ENGLISH_ARTICLE => parse::<EnglishArticle>(&value, "english.article")
                .map(EntityPayload::EnglishArticle),
            EntityType::ENGLISH_LEARNING_RECORD => {
                parse::<EnglishLearningRecord>(&value, "english.learning_record")
                    .map(EntityPayload::EnglishLearningRecord)
            }
            EntityType::ENGLISH_HIGHLIGHT => parse::<EnglishHighlight>(&value, "english.highlight")
                .map(EntityPayload::EnglishHighlight),
            EntityType::ENGLISH_NOTE => {
                parse::<EnglishNote>(&value, "english.note").map(EntityPayload::EnglishNote)
            }
            EntityType::ENGLISH_VOCABULARY => {
                parse::<EnglishVocabulary>(&value, "english.vocabulary")
                    .map(EntityPayload::EnglishVocabulary)
            }
            EntityType::ENGLISH_VOCABULARY_OCCURRENCE => {
                parse::<VocabularyOccurrence>(&value, "english.vocabulary_occurrence")
                    .map(EntityPayload::VocabularyOccurrence)
            }
            EntityType::ENGLISH_VOCABULARY_REVIEW_STATE => {
                parse::<VocabularyReviewState>(&value, "english.vocabulary_review_state")
                    .map(EntityPayload::VocabularyReviewState)
            }
            EntityType::WORKOUT_IMPORT => {
                parse::<WorkoutImport>(&value, "workout.import").map(EntityPayload::WorkoutImport)
            }
            EntityType::WORKOUT_WORKOUT => {
                parse::<Workout>(&value, "workout.workout").map(EntityPayload::Workout)
            }
            EntityType::WORKOUT_EXERCISE => parse::<WorkoutExercise>(&value, "workout.exercise")
                .map(EntityPayload::WorkoutExercise),
            EntityType::WORKOUT_SET => {
                parse::<WorkoutSet>(&value, "workout.set").map(EntityPayload::WorkoutSet)
            }
            EntityType::WORKOUT_TRAINING_NOTE => {
                parse::<TrainingNote>(&value, "workout.training_note")
                    .map(EntityPayload::TrainingNote)
            }
            EntityType::FILE_METADATA => {
                parse::<FileMetadata>(&value, "file.metadata").map(EntityPayload::FileMetadata)
            }
            EntityType::ENTITY_LINK => {
                parse::<EntityLink>(&value, "entity.link").map(EntityPayload::EntityLink)
            }
            EntityType::USER_PREFERENCE => parse::<UserPreference>(&value, "user.preference")
                .map(EntityPayload::UserPreference),
            other => {
                let registered_type = match other {
                    EntityType::EXECUTION_PROJECT => Some(EntityType::EXECUTION_PROJECT),
                    EntityType::EXECUTION_RECURRENCE_RULE => {
                        Some(EntityType::EXECUTION_RECURRENCE_RULE)
                    }
                    EntityType::EXECUTION_TASK => Some(EntityType::EXECUTION_TASK),
                    EntityType::EXECUTION_TASK_DEPENDENCY => {
                        Some(EntityType::EXECUTION_TASK_DEPENDENCY)
                    }
                    EntityType::EXECUTION_TASK_OCCURRENCE => {
                        Some(EntityType::EXECUTION_TASK_OCCURRENCE)
                    }
                    EntityType::EXECUTION_WAITING_ITEM => Some(EntityType::EXECUTION_WAITING_ITEM),
                    EntityType::EXECUTION_CALENDAR_EVENT => {
                        Some(EntityType::EXECUTION_CALENDAR_EVENT)
                    }
                    EntityType::EXECUTION_CALENDAR_OCCURRENCE => {
                        Some(EntityType::EXECUTION_CALENDAR_OCCURRENCE)
                    }
                    EntityType::EXECUTION_MEMO => Some(EntityType::EXECUTION_MEMO),
                    EntityType::EXECUTION_MEMO_TAG => Some(EntityType::EXECUTION_MEMO_TAG),
                    EntityType::EXECUTION_MEMO_TAG_RELATION => {
                        Some(EntityType::EXECUTION_MEMO_TAG_RELATION)
                    }
                    EntityType::EXECUTION_REMINDER => Some(EntityType::EXECUTION_REMINDER),
                    EntityType::EXECUTION_COMPLETION_RESULT => {
                        Some(EntityType::EXECUTION_COMPLETION_RESULT)
                    }
                    EntityType::EXECUTION_ENTITY_LINK => Some(EntityType::EXECUTION_ENTITY_LINK),
                    _ => None,
                };
                let Some(registered_type) = registered_type else {
                    return Err(format!("unknown entity type: {other}"));
                };
                if crate::registry::describe(registered_type).is_none() {
                    return Err(format!("unregistered entity type: {other}"));
                }
                let entity_id = value
                    .0
                    .get("meta")
                    .and_then(serde_json::Value::as_object)
                    .and_then(|meta| meta.get("id"))
                    .and_then(serde_json::Value::as_str)
                    .filter(|id| !id.is_empty())
                    .ok_or_else(|| format!("invalid {other} payload: meta.id is required"))?;
                Ok(EntityPayload::RegisteredJson {
                    entity_type: registered_type,
                    entity_id: EntityId::new(entity_id),
                    payload: value,
                })
            }
        }
    }
}

#[cfg(test)]
mod execution_registered_json_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn registered_execution_payload_preserves_json_and_id() {
        let entity_type = EntityType::new(EntityType::EXECUTION_TASK);
        let raw = JsonValue(json!({
            "meta": {"id": "task-1", "userId": "local"},
            "title": "Ship EPIC20",
            "status": "todo"
        }));
        let payload = EntityPayload::try_from((&entity_type, raw.clone())).unwrap();
        assert_eq!(payload.entity_type().as_str(), EntityType::EXECUTION_TASK);
        assert_eq!(payload.entity_id().as_str(), "task-1");
        assert_eq!(payload.to_json(), raw);
    }

    #[test]
    fn execution_payload_requires_meta_id_and_unknown_type_stays_rejected() {
        let entity_type = EntityType::new(EntityType::EXECUTION_MEMO);
        let missing_id = JsonValue(json!({"meta": {}, "content": "memo"}));
        assert!(EntityPayload::try_from((&entity_type, missing_id)).is_err());
        let unknown = EntityType::new("future.secret");
        let raw = JsonValue(json!({"meta": {"id": "x"}}));
        assert!(EntityPayload::try_from((&unknown, raw)).is_err());
    }
}
