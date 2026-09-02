//! Daily review DTO.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::common::EntityMeta;
use crate::time::LocalDate;

/// `review.daily`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct DailyReview {
    pub meta: EntityMeta,
    pub review_date: LocalDate,
    pub energy: Option<i64>,
    pub mood: Option<i64>,
    pub completion_score: Option<f64>,
    pub best_thing: Option<String>,
    pub problem: Option<String>,
    pub tomorrow_priority: Option<String>,
    pub note: Option<String>,
    #[serde(default)]
    pub completed_task_count: Option<u64>,
    #[serde(default)]
    pub total_task_count: Option<u64>,
}
