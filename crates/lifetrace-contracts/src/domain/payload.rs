//! Typed domain payload dispatch.
//!
//! Change payloads travel as opaque JSON on the wire; `EntityPayload` provides
//! strict validation for typed LifeTrace domains before the generic sync store
//! accepts them.

use crate::domain::english::*;
use crate::domain::execution::{FocusSession, ImportantDate};
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

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum EntityPayload {
    User(User),
    Device(Device),
    FinanceLedger(FinanceLedger),
    FinanceAccount(FinanceAccount),
    TransactionCategory(TransactionCategory),
    Transaction(Transaction),
    RecurringTransaction(RecurringTransaction),
    FinanceTag(FinanceTag),
    FinanceTransactionTag(FinanceTransactionTag),
    FinanceBudget(FinanceBudget),
    TransactionAttachment(TransactionAttachment),
    TransactionEvidence(TransactionEvidence),
    Activity(Activity),
    ActivityLog(ActivityLog),
    DailyReview(DailyReview),
    ImportantDate(ImportantDate),
    FocusSession(FocusSession),
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
    pub fn entity_type(&self) -> EntityType {
        EntityType::new(match self {
            EntityPayload::User(_) => EntityType::IDENTITY_USER,
            EntityPayload::Device(_) => EntityType::IDENTITY_DEVICE,
            EntityPayload::FinanceLedger(_) => EntityType::FINANCE_LEDGER,
            EntityPayload::FinanceAccount(_) => EntityType::FINANCE_ACCOUNT,
            EntityPayload::TransactionCategory(_) => EntityType::FINANCE_CATEGORY,
            EntityPayload::Transaction(_) => EntityType::FINANCE_TRANSACTION,
            EntityPayload::RecurringTransaction(_) => EntityType::FINANCE_RECURRING_TRANSACTION,
            EntityPayload::FinanceTag(_) => EntityType::FINANCE_TAG,
            EntityPayload::FinanceTransactionTag(_) => EntityType::FINANCE_TRANSACTION_TAG,
            EntityPayload::FinanceBudget(_) => EntityType::FINANCE_BUDGET,
            EntityPayload::TransactionAttachment(_) => EntityType::FINANCE_TRANSACTION_ATTACHMENT,
            EntityPayload::TransactionEvidence(_) => EntityType::FINANCE_TRANSACTION_EVIDENCE,
            EntityPayload::Activity(_) => EntityType::HABIT_ACTIVITY,
            EntityPayload::ActivityLog(_) => EntityType::HABIT_LOG,
            EntityPayload::DailyReview(_) => EntityType::REVIEW_DAILY,
            EntityPayload::ImportantDate(_) => EntityType::EXECUTION_IMPORTANT_DATE,
            EntityPayload::FocusSession(_) => EntityType::EXECUTION_FOCUS_SESSION,
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

    pub fn entity_id(&self) -> &EntityId {
        match self {
            EntityPayload::User(value) => &value.meta.id,
            EntityPayload::Device(value) => &value.meta.id,
            EntityPayload::FinanceLedger(value) => &value.meta.id,
            EntityPayload::FinanceAccount(value) => &value.meta.id,
            EntityPayload::TransactionCategory(value) => &value.meta.id,
            EntityPayload::Transaction(value) => &value.meta.id,
            EntityPayload::RecurringTransaction(value) => &value.meta.id,
            EntityPayload::FinanceTag(value) => &value.meta.id,
            EntityPayload::FinanceTransactionTag(value) => &value.meta.id,
            EntityPayload::FinanceBudget(value) => &value.meta.id,
            EntityPayload::TransactionAttachment(value) => &value.meta.id,
            EntityPayload::TransactionEvidence(value) => &value.meta.id,
            EntityPayload::Activity(value) => &value.meta.id,
            EntityPayload::ActivityLog(value) => &value.meta.id,
            EntityPayload::DailyReview(value) => &value.meta.id,
            EntityPayload::ImportantDate(value) => &value.id,
            EntityPayload::FocusSession(value) => &value.id,
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

    pub fn to_json(&self) -> JsonValue {
        macro_rules! json {
            ($value:expr) => {
                serde_json::to_value($value).unwrap().into()
            };
        }
        match self {
            EntityPayload::User(v) => json!(v),
            EntityPayload::Device(v) => json!(v),
            EntityPayload::FinanceLedger(v) => json!(v),
            EntityPayload::FinanceAccount(v) => json!(v),
            EntityPayload::TransactionCategory(v) => json!(v),
            EntityPayload::Transaction(v) => json!(v),
            EntityPayload::RecurringTransaction(v) => json!(v),
            EntityPayload::FinanceTag(v) => json!(v),
            EntityPayload::FinanceTransactionTag(v) => json!(v),
            EntityPayload::FinanceBudget(v) => json!(v),
            EntityPayload::TransactionAttachment(v) => json!(v),
            EntityPayload::TransactionEvidence(v) => json!(v),
            EntityPayload::Activity(v) => json!(v),
            EntityPayload::ActivityLog(v) => json!(v),
            EntityPayload::DailyReview(v) => json!(v),
            EntityPayload::ImportantDate(v) => json!(v),
            EntityPayload::FocusSession(v) => json!(v),
            EntityPayload::NoteFolder(v) => json!(v),
            EntityPayload::Note(v) => json!(v),
            EntityPayload::NoteTag(v) => json!(v),
            EntityPayload::NoteTagRelation(v) => json!(v),
            EntityPayload::NoteRelation(v) => json!(v),
            EntityPayload::NoteRevision(v) => json!(v),
            EntityPayload::EnglishArticle(v) => json!(v),
            EntityPayload::EnglishLearningRecord(v) => json!(v),
            EntityPayload::EnglishHighlight(v) => json!(v),
            EntityPayload::EnglishNote(v) => json!(v),
            EntityPayload::EnglishVocabulary(v) => json!(v),
            EntityPayload::VocabularyOccurrence(v) => json!(v),
            EntityPayload::VocabularyReviewState(v) => json!(v),
            EntityPayload::WorkoutImport(v) => json!(v),
            EntityPayload::Workout(v) => json!(v),
            EntityPayload::WorkoutExercise(v) => json!(v),
            EntityPayload::WorkoutSet(v) => json!(v),
            EntityPayload::TrainingNote(v) => json!(v),
            EntityPayload::FileMetadata(v) => json!(v),
            EntityPayload::EntityLink(v) => json!(v),
            EntityPayload::UserPreference(v) => json!(v),
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
        fn registered(
            value: JsonValue,
            entity_type: &'static str,
        ) -> Result<EntityPayload, String> {
            let id = value
                .0
                .get("meta")
                .and_then(|meta| meta.get("id"))
                .or_else(|| value.0.get("id"))
                .and_then(|id| id.as_str())
                .filter(|id| !id.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| {
                    format!("invalid {entity_type} payload: meta.id or id is required")
                })?;
            Ok(EntityPayload::RegisteredJson {
                entity_type,
                entity_id: EntityId::new(id),
                payload: value,
            })
        }

        match entity_type.as_str() {
            EntityType::IDENTITY_USER => {
                parse::<User>(&value, EntityType::IDENTITY_USER).map(EntityPayload::User)
            }
            EntityType::IDENTITY_DEVICE => {
                parse::<Device>(&value, EntityType::IDENTITY_DEVICE).map(EntityPayload::Device)
            }
            EntityType::FINANCE_LEDGER => {
                parse::<FinanceLedger>(&value, EntityType::FINANCE_LEDGER)
                    .map(EntityPayload::FinanceLedger)
            }
            EntityType::FINANCE_ACCOUNT => {
                parse::<FinanceAccount>(&value, EntityType::FINANCE_ACCOUNT)
                    .map(EntityPayload::FinanceAccount)
            }
            EntityType::FINANCE_CATEGORY => {
                parse::<TransactionCategory>(&value, EntityType::FINANCE_CATEGORY)
                    .map(EntityPayload::TransactionCategory)
            }
            EntityType::FINANCE_TRANSACTION => {
                parse::<Transaction>(&value, EntityType::FINANCE_TRANSACTION)
                    .map(EntityPayload::Transaction)
            }
            EntityType::FINANCE_RECURRING_TRANSACTION => {
                parse::<RecurringTransaction>(&value, EntityType::FINANCE_RECURRING_TRANSACTION)
                    .map(EntityPayload::RecurringTransaction)
            }
            EntityType::FINANCE_TAG => {
                parse::<FinanceTag>(&value, EntityType::FINANCE_TAG).map(EntityPayload::FinanceTag)
            }
            EntityType::FINANCE_TRANSACTION_TAG => {
                parse::<FinanceTransactionTag>(&value, EntityType::FINANCE_TRANSACTION_TAG)
                    .map(EntityPayload::FinanceTransactionTag)
            }
            EntityType::FINANCE_BUDGET => {
                parse::<FinanceBudget>(&value, EntityType::FINANCE_BUDGET)
                    .map(EntityPayload::FinanceBudget)
            }
            EntityType::FINANCE_TRANSACTION_ATTACHMENT => {
                parse::<TransactionAttachment>(&value, EntityType::FINANCE_TRANSACTION_ATTACHMENT)
                    .map(EntityPayload::TransactionAttachment)
            }
            EntityType::FINANCE_TRANSACTION_EVIDENCE => {
                parse::<TransactionEvidence>(&value, EntityType::FINANCE_TRANSACTION_EVIDENCE)
                    .map(EntityPayload::TransactionEvidence)
            }
            EntityType::HABIT_ACTIVITY => {
                parse::<Activity>(&value, EntityType::HABIT_ACTIVITY).map(EntityPayload::Activity)
            }
            EntityType::HABIT_LOG => {
                parse::<ActivityLog>(&value, EntityType::HABIT_LOG).map(EntityPayload::ActivityLog)
            }
            EntityType::REVIEW_DAILY => parse::<DailyReview>(&value, EntityType::REVIEW_DAILY)
                .map(EntityPayload::DailyReview),
            EntityType::NOTE_FOLDER => {
                parse::<NoteFolder>(&value, EntityType::NOTE_FOLDER).map(EntityPayload::NoteFolder)
            }
            EntityType::NOTE_NOTE => {
                parse::<Note>(&value, EntityType::NOTE_NOTE).map(EntityPayload::Note)
            }
            EntityType::NOTE_TAG => {
                parse::<NoteTag>(&value, EntityType::NOTE_TAG).map(EntityPayload::NoteTag)
            }
            EntityType::NOTE_TAG_RELATION => {
                parse::<NoteTagRelation>(&value, EntityType::NOTE_TAG_RELATION)
                    .map(EntityPayload::NoteTagRelation)
            }
            EntityType::NOTE_RELATION => parse::<NoteRelation>(&value, EntityType::NOTE_RELATION)
                .map(EntityPayload::NoteRelation),
            EntityType::NOTE_REVISION => parse::<NoteRevision>(&value, EntityType::NOTE_REVISION)
                .map(EntityPayload::NoteRevision),
            EntityType::ENGLISH_ARTICLE => {
                parse::<EnglishArticle>(&value, EntityType::ENGLISH_ARTICLE)
                    .map(EntityPayload::EnglishArticle)
            }
            EntityType::ENGLISH_LEARNING_RECORD => {
                parse::<EnglishLearningRecord>(&value, EntityType::ENGLISH_LEARNING_RECORD)
                    .map(EntityPayload::EnglishLearningRecord)
            }
            EntityType::ENGLISH_HIGHLIGHT => {
                parse::<EnglishHighlight>(&value, EntityType::ENGLISH_HIGHLIGHT)
                    .map(EntityPayload::EnglishHighlight)
            }
            EntityType::ENGLISH_NOTE => parse::<EnglishNote>(&value, EntityType::ENGLISH_NOTE)
                .map(EntityPayload::EnglishNote),
            EntityType::ENGLISH_VOCABULARY => {
                parse::<EnglishVocabulary>(&value, EntityType::ENGLISH_VOCABULARY)
                    .map(EntityPayload::EnglishVocabulary)
            }
            EntityType::ENGLISH_VOCABULARY_OCCURRENCE => {
                parse::<VocabularyOccurrence>(&value, EntityType::ENGLISH_VOCABULARY_OCCURRENCE)
                    .map(EntityPayload::VocabularyOccurrence)
            }
            EntityType::ENGLISH_VOCABULARY_REVIEW_STATE => {
                parse::<VocabularyReviewState>(&value, EntityType::ENGLISH_VOCABULARY_REVIEW_STATE)
                    .map(EntityPayload::VocabularyReviewState)
            }
            EntityType::WORKOUT_IMPORT => {
                parse::<WorkoutImport>(&value, EntityType::WORKOUT_IMPORT)
                    .map(EntityPayload::WorkoutImport)
            }
            EntityType::WORKOUT_WORKOUT => {
                parse::<Workout>(&value, EntityType::WORKOUT_WORKOUT).map(EntityPayload::Workout)
            }
            EntityType::WORKOUT_EXERCISE => {
                parse::<WorkoutExercise>(&value, EntityType::WORKOUT_EXERCISE)
                    .map(EntityPayload::WorkoutExercise)
            }
            EntityType::WORKOUT_SET => {
                parse::<WorkoutSet>(&value, EntityType::WORKOUT_SET).map(EntityPayload::WorkoutSet)
            }
            EntityType::WORKOUT_TRAINING_NOTE => {
                parse::<TrainingNote>(&value, EntityType::WORKOUT_TRAINING_NOTE)
                    .map(EntityPayload::TrainingNote)
            }
            EntityType::FILE_METADATA => parse::<FileMetadata>(&value, EntityType::FILE_METADATA)
                .map(EntityPayload::FileMetadata),
            EntityType::ENTITY_LINK => {
                parse::<EntityLink>(&value, EntityType::ENTITY_LINK).map(EntityPayload::EntityLink)
            }
            EntityType::USER_PREFERENCE => {
                parse::<UserPreference>(&value, EntityType::USER_PREFERENCE)
                    .map(EntityPayload::UserPreference)
            }
            EntityType::EXECUTION_GOAL => registered(value, EntityType::EXECUTION_GOAL),
            EntityType::EXECUTION_WEEKLY_REVIEW => {
                registered(value, EntityType::EXECUTION_WEEKLY_REVIEW)
            }
            EntityType::EXECUTION_PROJECT => registered(value, EntityType::EXECUTION_PROJECT),
            EntityType::EXECUTION_RECURRENCE_RULE => {
                registered(value, EntityType::EXECUTION_RECURRENCE_RULE)
            }
            EntityType::EXECUTION_TASK => registered(value, EntityType::EXECUTION_TASK),
            EntityType::EXECUTION_TASK_DEPENDENCY => {
                registered(value, EntityType::EXECUTION_TASK_DEPENDENCY)
            }
            EntityType::EXECUTION_TASK_OCCURRENCE => {
                registered(value, EntityType::EXECUTION_TASK_OCCURRENCE)
            }
            EntityType::EXECUTION_WAITING_ITEM => {
                registered(value, EntityType::EXECUTION_WAITING_ITEM)
            }
            EntityType::EXECUTION_CALENDAR_EVENT => {
                registered(value, EntityType::EXECUTION_CALENDAR_EVENT)
            }
            EntityType::EXECUTION_CALENDAR_OCCURRENCE => {
                registered(value, EntityType::EXECUTION_CALENDAR_OCCURRENCE)
            }
            EntityType::EXECUTION_IMPORTANT_DATE => {
                parse::<ImportantDate>(&value, EntityType::EXECUTION_IMPORTANT_DATE)
                    .map(EntityPayload::ImportantDate)
            }
            EntityType::EXECUTION_FOCUS_SESSION => {
                parse::<FocusSession>(&value, EntityType::EXECUTION_FOCUS_SESSION)
                    .map(EntityPayload::FocusSession)
            }
            EntityType::EXECUTION_MEMO => registered(value, EntityType::EXECUTION_MEMO),
            EntityType::EXECUTION_MEMO_TAG => registered(value, EntityType::EXECUTION_MEMO_TAG),
            EntityType::EXECUTION_MEMO_TAG_RELATION => {
                registered(value, EntityType::EXECUTION_MEMO_TAG_RELATION)
            }
            EntityType::EXECUTION_REMINDER => registered(value, EntityType::EXECUTION_REMINDER),
            EntityType::EXECUTION_COMPLETION_RESULT => {
                registered(value, EntityType::EXECUTION_COMPLETION_RESULT)
            }
            EntityType::EXECUTION_ENTITY_LINK => {
                registered(value, EntityType::EXECUTION_ENTITY_LINK)
            }
            other => Err(format!("unknown entity type: {other}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::EntityMeta;
    use crate::ids::UserId;
    use crate::time::UtcTimestamp;

    fn meta(id: &str) -> EntityMeta {
        let stamp: UtcTimestamp = "2026-08-12T00:00:00Z".parse().unwrap();
        EntityMeta {
            id: EntityId::new(id),
            user_id: UserId::new("user-1"),
            created_at: stamp,
            updated_at: stamp,
            deleted_at: None,
            local_version: 1,
            server_version: None,
            modified_by_device: None,
        }
    }

    #[test]
    fn finance_ledger_dispatch_round_trips() {
        let payload = FinanceLedger {
            meta: meta("ledger-1"),
            name: "日常账本".to_owned(),
            currency: crate::money::CurrencyCode::cny(),
            ledger_type: "personal".to_owned(),
            month_start_day: 1,
            sort_order: 0,
            is_archived: false,
        };
        let value: JsonValue = serde_json::to_value(&payload).unwrap().into();
        let parsed =
            EntityPayload::try_from((&EntityType::new(EntityType::FINANCE_LEDGER), value)).unwrap();
        assert_eq!(parsed.entity_id().as_str(), "ledger-1");
        assert_eq!(parsed.entity_type().as_str(), EntityType::FINANCE_LEDGER);
    }

    #[test]
    fn execute_important_date_dispatch_accepts_android_wire_payload() {
        let value: JsonValue = serde_json::json!({
            "id": "important-1",
            "userId": "user-1",
            "title": "生日",
            "date": "2026-09-02",
            "repeat": "yearly",
            "kind": "birthday",
            "calendar": "solar",
            "lunarMonth": null,
            "lunarDay": null,
            "lunarLeapMonth": false
        })
        .into();

        let parsed = EntityPayload::try_from((
            &EntityType::new(EntityType::EXECUTION_IMPORTANT_DATE),
            value,
        ))
        .unwrap();

        assert_eq!(parsed.entity_id().as_str(), "important-1");
        assert_eq!(parsed.to_json().0["title"], "生日");
    }

    #[test]
    fn execute_important_date_rejects_incomplete_payload() {
        let value: JsonValue = serde_json::json!({
            "id": "important-1",
            "userId": "user-1"
        })
        .into();

        assert!(EntityPayload::try_from((
            &EntityType::new(EntityType::EXECUTION_IMPORTANT_DATE),
            value,
        ))
        .is_err());
    }

    #[test]
    fn execute_focus_session_dispatch_accepts_android_wire_payload() {
        let value: JsonValue = serde_json::json!({
            "id": "focus-1",
            "userId": "user-1",
            "taskId": null,
            "mode": "short",
            "startedAt": "2026-09-02T00:00:00Z",
            "endedAt": "2026-09-02T00:25:00Z",
            "focusSeconds": 1500,
            "completed": true
        })
        .into();

        let parsed =
            EntityPayload::try_from((&EntityType::new(EntityType::EXECUTION_FOCUS_SESSION), value))
                .unwrap();

        assert_eq!(parsed.entity_id().as_str(), "focus-1");
        assert_eq!(parsed.to_json().0["focusSeconds"], 1500);
    }

    #[test]
    fn registered_execute_payload_accepts_android_top_level_identity() {
        let value: JsonValue = serde_json::json!({
            "id": "task-1",
            "userId": "user-1",
            "title": "Android task"
        })
        .into();

        let parsed =
            EntityPayload::try_from((&EntityType::new(EntityType::EXECUTION_TASK), value)).unwrap();

        assert_eq!(parsed.entity_id().as_str(), "task-1");
    }
}
