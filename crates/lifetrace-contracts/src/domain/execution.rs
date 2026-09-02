//! Execute Android wire DTOs that do not use the desktop `EntityMeta` envelope.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ids::{EntityId, UserId};
use crate::time::{LocalDate, UtcTimestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ImportantDateRepeat {
    Once,
    Yearly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ImportantDateKind {
    Birthday,
    Anniversary,
    Milestone,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum ImportantDateCalendar {
    Solar,
    Lunar,
}

/// `execution.important_date`, matching `ImportantDateWireMapper` on Android.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ImportantDate {
    pub id: EntityId,
    pub user_id: UserId,
    pub title: String,
    pub date: LocalDate,
    pub repeat: ImportantDateRepeat,
    pub kind: ImportantDateKind,
    pub calendar: ImportantDateCalendar,
    pub lunar_month: Option<u8>,
    pub lunar_day: Option<u8>,
    pub lunar_leap_month: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename_all = "snake_case")]
pub enum FocusMode {
    Short,
    Long,
}

/// `execution.focus_session`, matching `FocusSessionWireMapper` on Android.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct FocusSession {
    pub id: EntityId,
    pub user_id: UserId,
    pub task_id: Option<EntityId>,
    pub mode: FocusMode,
    pub started_at: UtcTimestamp,
    pub ended_at: UtcTimestamp,
    pub focus_seconds: u64,
    pub completed: bool,
}
