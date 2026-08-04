//! Extensible string enums.
//!
//! Wire enums are string newtypes, not closed Rust enums, so a value added by
//! a newer server/client can never fail parsing of an entire sync batch.
//! Known values are exposed as associated constants.

use std::borrow::Cow;
use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ts_rs::TS;

macro_rules! wire_string_enum {
    ($name:ident, $doc:expr, [$($variant:ident => $value:expr),* $(,)?]) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            $(pub const $variant: &'static str = $value;)*

            /// Wrap any string. Unknown values are preserved (forward compatible).
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let raw = String::deserialize(deserializer)?;
                Ok(Self(raw))
            }
        }

        impl JsonSchema for $name {
            fn schema_name() -> Cow<'static, str> {
                Cow::Borrowed(stringify!($name))
            }

            fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
                let mut schema = String::json_schema(generator);
                if let Some(object) = schema.as_object_mut() {
                    object.insert(
                        "description".to_owned(),
                        serde_json::Value::String(format!(
                            "{} Known values: {}. Unknown values are preserved (forward compatible).",
                            $doc,
                            [$($value),*].join(", ")
                        )),
                    );
                }
                schema
            }
        }

        impl TS for $name {
            type WithoutGenerics = Self;
            type OptionInnerType = Self;

            fn decl() -> String {
                format!("type {} = string;", stringify!($name))
            }

            fn decl_concrete() -> String {
                Self::decl()
            }

            fn name() -> String {
                stringify!($name).to_owned()
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
    };
}

wire_string_enum!(TransactionType, "Financial transaction type.", [
    EXPENSE => "expense",
    INCOME => "income",
    TRANSFER => "transfer",
    REFUND => "refund",
    FEE => "fee",
]);

wire_string_enum!(TransactionStatus, "Financial transaction status.", [
    CANDIDATE => "candidate",
    PROVISIONAL => "provisional",
    CONFIRMED => "confirmed",
    IGNORED => "ignored",
]);

wire_string_enum!(AccountType, "Finance account type.", [
    CASH => "cash",
    BANK => "bank",
    WECHAT => "wechat",
    ALIPAY => "alipay",
    INVESTMENT => "investment",
    OTHER => "other",
]);

wire_string_enum!(ActivityType, "Habit activity type.", [
    DURATION => "duration",
    COUNT => "count",
    COMPLETION => "completion",
    WEEKLY => "weekly",
    CONTROL => "control",
]);

wire_string_enum!(ActivityScheduleType, "Habit schedule type.", [
    DAILY => "daily",
    WEEKLY => "weekly",
    CUSTOM => "custom",
]);

wire_string_enum!(ActivityCheckinMethod, "Habit check-in method.", [
    MANUAL => "manual",
    AUTOMATIC => "automatic",
]);

wire_string_enum!(ActivitySyncSource, "Habit sync source.", [
    FITNESS => "fitness",
    ENGLISH => "english",
]);

wire_string_enum!(ActivityLogStatus, "Habit log status.", [
    COMPLETED => "completed",
    PARTIAL => "partial",
    SKIPPED => "skipped",
]);

wire_string_enum!(NoteType, "Note type.", [
    QUICK => "quick",
    DOCUMENT => "document",
    DAILY => "daily",
    HABIT_LOG => "habit_log",
    WORKOUT_REVIEW => "workout_review",
    EXPENSE_NOTE => "expense_note",
    WEEKLY_REVIEW => "weekly_review",
    MONTHLY_REVIEW => "monthly_review",
]);

wire_string_enum!(EnglishLevel, "CEFR level.", [
    A1 => "A1",
    A2 => "A2",
    B1 => "B1",
    B2 => "B2",
    C1 => "C1",
]);

wire_string_enum!(EnglishCategory, "English article category.", [
    TECHNOLOGY => "Technology",
    SCIENCE => "Science",
    LIFE => "Life",
    BUSINESS => "Business",
    CULTURE => "Culture",
]);

wire_string_enum!(EnglishProcessingStatus, "English article processing status.", [
    FETCHED => "FETCHED",
    CLEANED => "CLEANED",
    ANALYZED => "ANALYZED",
    READY => "READY",
    REJECTED => "REJECTED",
    FAILED => "FAILED",
]);

wire_string_enum!(EnglishFetchStatus, "English article fetch status.", [
    PENDING => "PENDING",
    SUCCESS => "SUCCESS",
    FAILED => "FAILED",
    SKIPPED => "SKIPPED",
]);

wire_string_enum!(EnglishCompletionStatus, "English learning record completion status.", [
    READING => "reading",
    SUMMARIZED => "summarized",
    ANALYZED => "analyzed",
    COMPLETED => "completed",
]);

wire_string_enum!(EnglishReadingStatus, "English reading status.", [
    UNREAD => "unread",
    READING => "reading",
    COMPLETED => "completed",
]);

wire_string_enum!(VocabularyStatus, "Vocabulary status.", [
    LEARNING => "LEARNING",
    REVIEWING => "REVIEWING",
    MASTERED => "MASTERED",
    ARCHIVED => "ARCHIVED",
]);

wire_string_enum!(VocabularyReviewResult, "Vocabulary review result.", [
    FORGOT => "FORGOT",
    HARD => "HARD",
    GOOD => "GOOD",
    EASY => "EASY",
]);

wire_string_enum!(HighlightColor, "English highlight color.", [
    YELLOW => "yellow",
    GREEN => "green",
    BLUE => "blue",
]);

wire_string_enum!(WorkoutSource, "Workout source.", [
    MANUAL => "manual",
    XUNJI => "xunji",
]);

wire_string_enum!(WorkoutStatus, "Workout status.", [
    COMPLETED => "completed",
    PARTIAL => "partial",
]);

wire_string_enum!(ImportStatus, "Workout import status.", [
    PENDING => "pending",
    SUCCESS => "success",
    FAILED => "failed",
]);

wire_string_enum!(FileStorageState, "File storage state.", [
    LOCAL_ONLY => "local_only",
    PENDING_UPLOAD => "pending_upload",
    SERVER_STORED => "server_stored",
]);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_type_round_trips_known_and_unknown() {
        let known = TransactionType::new(TransactionType::EXPENSE);
        let json = serde_json::to_string(&known).unwrap();
        assert_eq!(json, "\"expense\"");
        let back: TransactionType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, known);

        let unknown: TransactionType = serde_json::from_str("\"future_type\"").unwrap();
        assert_eq!(unknown.as_str(), "future_type");
        assert_eq!(serde_json::to_string(&unknown).unwrap(), "\"future_type\"");
    }

    #[test]
    fn all_required_domain_enums_are_present() {
        assert_eq!(TransactionType::REFUND, "refund");
        assert_eq!(TransactionType::FEE, "fee");
        assert_eq!(TransactionStatus::CANDIDATE, "candidate");
        assert_eq!(ActivityLogStatus::PARTIAL, "partial");
        assert_eq!(NoteType::WEEKLY_REVIEW, "weekly_review");
        assert_eq!(EnglishProcessingStatus::FETCHED, "FETCHED");
        assert_eq!(VocabularyStatus::MASTERED, "MASTERED");
        assert_eq!(FileStorageState::LOCAL_ONLY, "local_only");
    }
}
