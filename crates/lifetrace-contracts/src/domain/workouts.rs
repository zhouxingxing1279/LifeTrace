//! Workout domain DTOs.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::common::EntityMeta;
use crate::domain::enums::{ImportStatus, WorkoutSource, WorkoutStatus};
use crate::ids::EntityId;
use crate::time::{LocalDate, UtcTimestamp};

/// `workout.import`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct WorkoutImport {
    pub meta: EntityMeta,
    pub source: WorkoutSource,
    pub share_url: Option<String>,
    pub status: ImportStatus,
    pub parser: Option<String>,
    pub parser_version: Option<String>,
    pub error: Option<String>,
    pub workout_id: Option<EntityId>,
}

/// `workout.workout` (summary; exercises and sets are separate entities).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct Workout {
    pub meta: EntityMeta,
    pub source: WorkoutSource,
    pub source_id: Option<String>,
    pub name: String,
    pub occurred_at: UtcTimestamp,
    pub local_date: LocalDate,
    pub duration_seconds: i64,
    pub exercise_count: i64,
    pub set_count: i64,
    pub planned_set_count: Option<i64>,
    pub volume_kg: Option<f64>,
    pub calories_kcal: Option<f64>,
    pub status: Option<WorkoutStatus>,
}

/// `workout.exercise`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct WorkoutExercise {
    pub meta: EntityMeta,
    pub workout_id: EntityId,
    pub name: String,
    pub sort_order: i64,
    pub planned_sets: i64,
    pub completed_sets: i64,
}

/// `workout.set`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct WorkoutSet {
    pub meta: EntityMeta,
    pub exercise_id: EntityId,
    pub set_number: i64,
    pub weight_kg: Option<f64>,
    pub reps: Option<i64>,
    pub completed: bool,
}

/// `workout.training_note`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct TrainingNote {
    pub meta: EntityMeta,
    pub title: String,
    pub content: String,
    pub workout_id: Option<EntityId>,
    pub source: WorkoutSource,
    pub note_date: LocalDate,
}
