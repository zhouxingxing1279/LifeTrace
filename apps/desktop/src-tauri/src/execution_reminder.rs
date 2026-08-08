use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::Deserialize;

use crate::{
    database::{
        profile,
        repositories::execution_reminder::{self as repository, ReminderRecord, ReminderWrite},
    },
    execution::{ExecutionError, ExecutionErrorKind, ExecutionResult},
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderInput {
    pub subject_type: String,
    pub subject_id: String,
    pub trigger_at: String,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderUpdateInput {
    pub trigger_at: String,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderSnoozeInput {
    pub until_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DueReminderQuery {
    pub now: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubjectReminderQuery {
    pub subject_type: String,
    pub subject_id: String,
}

fn error(kind: ExecutionErrorKind, message: impl Into<String>) -> ExecutionError {
    ExecutionError {
        kind,
        message: message.into(),
    }
}

fn validation(message: impl Into<String>) -> ExecutionError {
    error(ExecutionErrorKind::Validation, message)
}

fn not_found(message: impl Into<String>) -> ExecutionError {
    error(ExecutionErrorKind::NotFound, message)
}

fn conflict(message: impl Into<String>) -> ExecutionError {
    error(ExecutionErrorKind::Conflict, message)
}

fn storage(message: impl Into<String>) -> ExecutionError {
    error(ExecutionErrorKind::Storage, message)
}

fn active_user(connection: &Connection) -> ExecutionResult<String> {
    profile::active_profile_id(connection).map_err(storage)
}

fn clean_required(value: &str, label: &str, max: usize) -> ExecutionResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(validation(format!("{label}不能为空")));
    }
    if value.chars().count() > max {
        return Err(validation(format!("{label}不能超过 {max} 个字符")));
    }
    Ok(value.to_owned())
}

fn clean_optional(
    value: Option<String>,
    label: &str,
    max: usize,
) -> ExecutionResult<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > max {
        return Err(validation(format!("{label}不能超过 {max} 个字符")));
    }
    Ok(Some(value.to_owned()))
}

fn utc_timestamp(value: &str, label: &str) -> ExecutionResult<String> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc).to_rfc3339())
        .map_err(|_| validation(format!("{label}必须是 RFC3339 时间")))
}

fn valid_subject_type(subject_type: &str) -> bool {
    matches!(
        subject_type,
        "task" | "calendar_event" | "waiting_item" | "memo"
    )
}

fn subject_exists(
    connection: &Connection,
    user_id: &str,
    subject_type: &str,
    subject_id: &str,
) -> ExecutionResult<bool> {
    let table = match subject_type {
        "task" => "execution_tasks",
        "calendar_event" => "execution_calendar_events",
        "waiting_item" => "execution_waiting_items",
        "memo" => "execution_memos",
        _ => {
            return Err(validation(
                "subjectType 必须是 task/calendar_event/waiting_item/memo",
            ))
        }
    };
    let sql = format!(
        "SELECT EXISTS(SELECT 1 FROM {table} WHERE id=?1 AND user_id=?2 AND deleted_at IS NULL)"
    );
    connection
        .query_row(&sql, params![subject_id, user_id], |row| row.get(0))
        .map_err(|error| storage(error.to_string()))
}

fn fire_key(subject_type: &str, subject_id: &str, trigger_at: &str) -> String {
    format!("{subject_type}:{subject_id}:{trigger_at}")
}

pub fn get_reminder(connection: &Connection, id: &str) -> ExecutionResult<ReminderRecord> {
    let user_id = active_user(connection)?;
    repository::get(connection, &user_id, id)
        .map_err(storage)?
        .ok_or_else(|| not_found("提醒不存在"))
}

pub fn list_subject_reminders(
    connection: &Connection,
    query: SubjectReminderQuery,
) -> ExecutionResult<Vec<ReminderRecord>> {
    let user_id = active_user(connection)?;
    let subject_type = clean_required(&query.subject_type, "subjectType", 64)?;
    let subject_id = clean_required(&query.subject_id, "subjectId", 128)?;
    if !valid_subject_type(&subject_type) {
        return Err(validation(
            "subjectType 必须是 task/calendar_event/waiting_item/memo",
        ));
    }
    repository::list_for_subject(connection, &user_id, &subject_type, &subject_id).map_err(storage)
}

pub fn list_due_reminders(
    connection: &Connection,
    query: DueReminderQuery,
) -> ExecutionResult<Vec<ReminderRecord>> {
    let user_id = active_user(connection)?;
    let now_at = match query.now {
        Some(value) => utc_timestamp(&value, "now")?,
        None => Utc::now().to_rfc3339(),
    };
    let limit = query.limit.unwrap_or(100);
    if !(1..=500).contains(&limit) {
        return Err(validation("limit 必须位于 1..=500"));
    }
    repository::list_due(connection, &user_id, &now_at, limit).map_err(storage)
}

pub fn create_reminder(
    connection: &Connection,
    input: ReminderInput,
) -> ExecutionResult<ReminderRecord> {
    let user_id = active_user(connection)?;
    let subject_type = clean_required(&input.subject_type, "subjectType", 64)?;
    let subject_id = clean_required(&input.subject_id, "subjectId", 128)?;
    if !valid_subject_type(&subject_type) {
        return Err(validation(
            "subjectType 必须是 task/calendar_event/waiting_item/memo",
        ));
    }
    if !subject_exists(connection, &user_id, &subject_type, &subject_id)? {
        return Err(validation("提醒目标不存在或不属于当前资料"));
    }
    let trigger_at = utc_timestamp(&input.trigger_at, "triggerAt")?;
    let fire_key = fire_key(&subject_type, &subject_id, &trigger_at);
    if let Some(existing) =
        repository::find_by_fire_key(connection, &user_id, &fire_key).map_err(storage)?
    {
        return Ok(existing);
    }
    repository::save(
        connection,
        &ReminderWrite {
            id: None,
            user_id,
            subject_type,
            subject_id,
            trigger_at,
            timezone: clean_optional(input.timezone, "timezone", 128)?,
            status: "scheduled".to_owned(),
            snoozed_until: None,
            last_fired_at: None,
            fire_key,
        },
    )
    .map_err(storage)
}

pub fn update_reminder(
    connection: &Connection,
    id: &str,
    input: ReminderUpdateInput,
) -> ExecutionResult<ReminderRecord> {
    let user_id = active_user(connection)?;
    let current = repository::get(connection, &user_id, id)
        .map_err(storage)?
        .ok_or_else(|| not_found("提醒不存在"))?;
    if current.status != "scheduled" {
        return Err(conflict("只有 scheduled 状态的提醒可以修改时间"));
    }
    let trigger_at = utc_timestamp(&input.trigger_at, "triggerAt")?;
    let key = fire_key(&current.subject_type, &current.subject_id, &trigger_at);
    if let Some(existing) =
        repository::find_by_fire_key(connection, &user_id, &key).map_err(storage)?
    {
        if existing.id != current.id {
            return Err(conflict("同一对象在该时间已存在提醒"));
        }
    }
    repository::save(
        connection,
        &ReminderWrite {
            id: Some(current.id),
            user_id,
            subject_type: current.subject_type,
            subject_id: current.subject_id,
            trigger_at,
            timezone: clean_optional(input.timezone, "timezone", 128)?,
            status: current.status,
            snoozed_until: None,
            last_fired_at: current.last_fired_at,
            fire_key: key,
        },
    )
    .map_err(storage)
}

fn transition(
    connection: &Connection,
    id: &str,
    status: &str,
    snoozed_until: Option<String>,
    last_fired_at: Option<String>,
) -> ExecutionResult<ReminderRecord> {
    let user_id = active_user(connection)?;
    let current = repository::get(connection, &user_id, id)
        .map_err(storage)?
        .ok_or_else(|| not_found("提醒不存在"))?;
    repository::save(
        connection,
        &ReminderWrite {
            id: Some(current.id),
            user_id,
            subject_type: current.subject_type,
            subject_id: current.subject_id,
            trigger_at: current.trigger_at,
            timezone: current.timezone,
            status: status.to_owned(),
            snoozed_until,
            last_fired_at,
            fire_key: current.fire_key,
        },
    )
    .map_err(storage)
}

pub fn mark_fired(connection: &Connection, id: &str) -> ExecutionResult<ReminderRecord> {
    let current = get_reminder(connection, id)?;
    if current.status == "fired" {
        return Ok(current);
    }
    if current.status != "scheduled" {
        return Err(conflict("只有 scheduled 提醒可以标记 fired"));
    }
    transition(connection, id, "fired", None, Some(Utc::now().to_rfc3339()))
}

pub fn snooze_reminder(
    connection: &Connection,
    id: &str,
    input: ReminderSnoozeInput,
) -> ExecutionResult<ReminderRecord> {
    let current = get_reminder(connection, id)?;
    if !matches!(current.status.as_str(), "scheduled" | "fired") {
        return Err(conflict("只有 scheduled 或 fired 提醒可以稍后提醒"));
    }
    let until_at = utc_timestamp(&input.until_at, "untilAt")?;
    if DateTime::parse_from_rfc3339(&until_at)
        .unwrap()
        .with_timezone(&Utc)
        <= Utc::now()
    {
        return Err(validation("untilAt 必须晚于当前时间"));
    }
    transition(
        connection,
        id,
        "scheduled",
        Some(until_at),
        current.last_fired_at,
    )
}

pub fn dismiss_reminder(connection: &Connection, id: &str) -> ExecutionResult<ReminderRecord> {
    let current = get_reminder(connection, id)?;
    if current.status == "dismissed" {
        return Ok(current);
    }
    if !matches!(current.status.as_str(), "scheduled" | "fired") {
        return Err(conflict("当前提醒不能 dismissed"));
    }
    transition(connection, id, "dismissed", None, current.last_fired_at)
}

pub fn cancel_reminder(connection: &Connection, id: &str) -> ExecutionResult<ReminderRecord> {
    let current = get_reminder(connection, id)?;
    if current.status == "cancelled" {
        return Ok(current);
    }
    if current.status == "dismissed" {
        return Err(conflict("已 dismissed 的提醒不能再取消"));
    }
    transition(connection, id, "cancelled", None, current.last_fired_at)
}

pub fn delete_reminder(connection: &Connection, id: &str) -> ExecutionResult<()> {
    let user_id = active_user(connection)?;
    if repository::get(connection, &user_id, id)
        .map_err(storage)?
        .is_none()
    {
        return Err(not_found("提醒不存在"));
    }
    if repository::soft_delete(connection, &user_id, id).map_err(storage)? {
        Ok(())
    } else {
        Err(not_found("提醒不存在"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migration_runner::{run, MigrationContext};
    use crate::database::migrations::all;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn db() -> Connection {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("lifetrace-reminder-service-{unique}"));
        std::fs::create_dir_all(&data_dir).unwrap();
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run(&mut connection, &MigrationContext::new(data_dir), &all()).unwrap();
        let user_id = profile::active_profile_id(&connection).unwrap();
        connection.execute(
            "INSERT INTO execution_tasks(id,user_id,title,status,priority,created_at,updated_at,version) VALUES('t1',?1,'task','todo','normal','2026-08-08','2026-08-08',1)",
            [user_id],
        ).unwrap();
        connection
    }

    fn input() -> ReminderInput {
        ReminderInput {
            subject_type: "task".to_owned(),
            subject_id: "t1".to_owned(),
            trigger_at: "2026-08-09T09:00:00+08:00".to_owned(),
            timezone: Some("Asia/Shanghai".to_owned()),
        }
    }

    #[test]
    fn create_is_idempotent_and_normalizes_trigger_to_utc() {
        let connection = db();
        let first = create_reminder(&connection, input()).unwrap();
        let second = create_reminder(&connection, input()).unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(first.trigger_at, "2026-08-09T01:00:00+00:00");
    }

    #[test]
    fn due_query_and_lifecycle_do_not_duplicate_fire() {
        let connection = db();
        let reminder = create_reminder(&connection, input()).unwrap();
        assert!(list_due_reminders(
            &connection,
            DueReminderQuery {
                now: Some("2026-08-09T00:59:59Z".to_owned()),
                limit: None
            }
        )
        .unwrap()
        .is_empty());
        assert_eq!(
            list_due_reminders(
                &connection,
                DueReminderQuery {
                    now: Some("2026-08-09T01:00:00Z".to_owned()),
                    limit: None
                }
            )
            .unwrap()
            .len(),
            1
        );
        let fired = mark_fired(&connection, &reminder.id).unwrap();
        let fired_again = mark_fired(&connection, &reminder.id).unwrap();
        assert_eq!(fired.version, fired_again.version);
        assert!(list_due_reminders(
            &connection,
            DueReminderQuery {
                now: Some("2026-08-10T00:00:00Z".to_owned()),
                limit: None
            }
        )
        .unwrap()
        .is_empty());
    }

    #[test]
    fn reminder_rejects_missing_subject() {
        let connection = db();
        let mut missing = input();
        missing.subject_id = "missing".to_owned();
        let error = create_reminder(&connection, missing).unwrap_err();
        assert_eq!(error.kind, ExecutionErrorKind::Validation);
    }
}
