//! English learning domain DTOs.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::common::EntityMeta;
use crate::domain::enums::{
    EnglishCategory, EnglishCompletionStatus, EnglishFetchStatus, EnglishLevel,
    EnglishProcessingStatus, EnglishReadingStatus, HighlightColor, VocabularyStatus,
};
use crate::ids::EntityId;
use crate::json_value::JsonValue;
use crate::time::{LocalDate, UtcTimestamp};

/// `english.article` (shared catalog; server to client).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct EnglishArticle {
    pub meta: EntityMeta,
    pub title: String,
    pub level: EnglishLevel,
    pub category: EnglishCategory,
    pub content: String,
    pub word_count: i64,
    pub difficulty: Option<i64>,
    pub estimated_minutes: Option<i64>,
    pub source: Option<String>,
    pub source_key: Option<String>,
    pub source_name: Option<String>,
    pub source_category: Option<String>,
    pub source_url: Option<String>,
    pub normalized_source_url: Option<String>,
    pub external_id: Option<String>,
    pub published_at: Option<UtcTimestamp>,
    pub source_updated_at: Option<UtcTimestamp>,
    pub image_url: Option<String>,
    pub audio_url: Option<String>,
    pub author: Option<String>,
    pub summary: Option<String>,
    pub fetched_at: Option<UtcTimestamp>,
    pub rights_note: Option<String>,
    pub content_hash: Option<String>,
    pub language: Option<String>,
    pub quality_score: Option<f64>,
    pub has_audio: bool,
    pub license_type: Option<String>,
    pub attribution: Option<String>,
    pub processing_status: Option<EnglishProcessingStatus>,
    pub fetch_status: Option<EnglishFetchStatus>,
    pub retry_count: i64,
    pub last_error: Option<String>,
    pub created_time: Option<UtcTimestamp>,
    pub questions: Vec<String>,
    pub vocabulary: Vec<ArticleVocabularyItem>,
}

/// Embedded article vocabulary item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ArticleVocabularyItem {
    pub word: String,
    pub phonetic: Option<String>,
    pub meaning: String,
    pub example: Option<String>,
}

/// `english.learning_record`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct EnglishLearningRecord {
    pub meta: EntityMeta,
    pub article_id: Option<EntityId>,
    pub record_date: LocalDate,
    pub reading_time_seconds: i64,
    pub summary: String,
    pub score: Option<f64>,
    pub analysis_id: Option<EntityId>,
    pub new_words: Vec<String>,
    pub completion_status: EnglishCompletionStatus,
    pub reading_status: Option<EnglishReadingStatus>,
    pub started_at: Option<UtcTimestamp>,
    pub completed_at: Option<UtcTimestamp>,
}

/// `english.highlight`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct EnglishHighlight {
    pub meta: EntityMeta,
    pub article_id: Option<EntityId>,
    pub selected_text: String,
    pub block_id: Option<String>,
    pub start_offset: Option<i64>,
    pub end_offset: Option<i64>,
    pub color: HighlightColor,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub note: Option<String>,
}

/// `english.note`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct EnglishNote {
    pub meta: EntityMeta,
    pub article_id: Option<EntityId>,
    pub quote: Option<String>,
    pub content: String,
    pub block_id: Option<String>,
    pub start_offset: Option<i64>,
    pub end_offset: Option<i64>,
    pub selected_text: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub highlight_id: Option<EntityId>,
}

/// `english.vocabulary`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct EnglishVocabulary {
    pub meta: EntityMeta,
    pub normalized_word: String,
    pub display_word: String,
    pub definition: String,
    pub phonetic: String,
    pub part_of_speech: String,
    pub selected_meanings: Vec<String>,
    pub lemma: String,
    pub source_article_id: Option<EntityId>,
    pub source_article_title: Option<String>,
    pub source_sentence: Option<String>,
    pub notes: String,
    pub mastery_level: i64,
    pub review_stage: i64,
    pub review_count: i64,
    pub correct_count: i64,
    pub incorrect_count: i64,
    pub encounter_count: i64,
    pub last_reviewed_at: Option<UtcTimestamp>,
    pub next_review_at: Option<UtcTimestamp>,
    pub status: VocabularyStatus,
    pub frequency_rank: Option<i64>,
    pub tags: Vec<String>,
    pub metadata: Option<JsonValue>,
}

/// `english.vocabulary_occurrence`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct VocabularyOccurrence {
    pub meta: EntityMeta,
    pub vocabulary_id: EntityId,
    pub article_id: Option<EntityId>,
    pub article_title: Option<String>,
    pub source_sentence: String,
}

/// `english.vocabulary_review_state`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct VocabularyReviewState {
    pub meta: EntityMeta,
    pub vocabulary_id: EntityId,
    pub due_at: Option<UtcTimestamp>,
    pub difficulty: Option<f64>,
    pub stability: Option<f64>,
    pub retrievability: Option<f64>,
    pub review_count: i64,
    pub lapse_count: i64,
    pub scheduler_version: Option<String>,
}
