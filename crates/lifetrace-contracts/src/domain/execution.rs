//! Typed wire contracts for the LifeTrace Execute domain.
//!
//! Execute clients are local-first, therefore every syncable payload carries
//! the shared `EntityMeta` revision fields. These DTOs deliberately describe
//! authoritative snapshots rather than Android/desktop UI state.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::common::EntityMeta;
use crate::ids::EntityId;
use crate::time::{LocalDate, UtcTimestamp};

/// `execution.project`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct ExecutionProject {
    pub meta: EntityMeta,
    pub name: String,
    pub description: Option<String>,
    /// `active | paused | completed | archived`.
    pub status: String,
    pub due_at: Option<UtcTimestamp>,
}

/// `execution.task`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct ExecutionTask {
    pub meta: EntityMeta,
    pub title: String,
    pub description: Option<String>,
    pub project_id: Option<EntityId>,
    /// `todo | in_progress | waiting | done`.
    pub status: String,
    /// `low | normal | high | urgent`.
    pub priority: String,
    pub due_at: Option<UtcTimestamp>,
    pub scheduled_at: Option<UtcTimestamp>,
    pub completed_at: Option<UtcTimestamp>,
}

/// `execution.important_date`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct ExecutionImportantDate {
    pub meta: EntityMeta,
    pub title: String,
    /// `birthday | anniversary | milestone | other`.
    pub r#type: String,
    /// `solar | lunar`.
    pub calendar_type: String,
    /// `once | yearly`.
    pub repeat_type: String,
    /// Present for solar dates. For yearly dates the year is retained as the
    /// source year but consumers compare/resolve month + day.
    pub solar_date: Option<LocalDate>,
    /// Required for a one-time lunar date, optional for yearly recurrence.
    pub lunar_year: Option<i32>,
    pub lunar_month: Option<u8>,
    pub lunar_day: Option<u8>,
    pub lunar_is_leap_month: bool,
    pub enabled: bool,
}

/// `execution.focus_session`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename_all = "camelCase")]
pub struct ExecutionFocusSession {
    pub meta: EntityMeta,
    pub task_id: Option<EntityId>,
    /// Timer preset/mode, for example `classic` or `deep`.
    pub mode: String,
    pub focus_seconds: u32,
    pub break_seconds: u32,
    pub round: u32,
    pub started_at: UtcTimestamp,
    pub ended_at: UtcTimestamp,
    /// Actual elapsed focus time, independent from the configured duration.
    pub elapsed_focus_seconds: u32,
    /// `completed | interrupted`.
    pub outcome: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::UserId;

    fn meta(id: &str) -> EntityMeta {
        let stamp: UtcTimestamp = "2026-09-01T00:00:00Z".parse().unwrap();
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
    fn task_wire_matches_android_contract() {
        let task = ExecutionTask {
            meta: meta("task-1"),
            title: "Ship 1.0".to_owned(),
            description: None,
            project_id: Some(EntityId::new("project-1")),
            status: "in_progress".to_owned(),
            priority: "high".to_owned(),
            due_at: None,
            scheduled_at: None,
            completed_at: None,
        };
        let value = serde_json::to_value(&task).unwrap();
        assert_eq!(value["meta"]["id"], "task-1");
        assert_eq!(value["projectId"], "project-1");
        assert_eq!(value["status"], "in_progress");
        assert_eq!(serde_json::from_value::<ExecutionTask>(value).unwrap(), task);
    }

    #[test]
    fn important_date_requires_declared_schema_fields() {
        let date = ExecutionImportantDate {
            meta: meta("date-1"),
            title: "Birthday".to_owned(),
            r#type: "birthday".to_owned(),
            calendar_type: "lunar".to_owned(),
            repeat_type: "yearly".to_owned(),
            solar_date: None,
            lunar_year: None,
            lunar_month: Some(8),
            lunar_day: Some(20),
            lunar_is_leap_month: false,
            enabled: true,
        };
        let value = serde_json::to_value(&date).unwrap();
        assert_eq!(value["type"], "birthday");
        assert_eq!(value["calendarType"], "lunar");
        assert_eq!(serde_json::from_value::<ExecutionImportantDate>(value).unwrap(), date);
    }
}
