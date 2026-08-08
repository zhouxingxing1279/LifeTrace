use chrono::{DateTime, Datelike, Duration, FixedOffset, Utc};
use rusqlite::Connection;
use serde::Deserialize;

use crate::{
    database::{
        profile,
        repositories::{
            execution as task_repository,
            execution_waiting::{
                self as repository, WaitingItemRecord, WaitingItemWrite, WaitingListFilter,
            },
        },
    },
    execution::{
        self, ExecutionError, ExecutionErrorKind, ExecutionResult, TaskInput, TaskStatusInput,
    },
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaitingItemInput {
    pub title: String,
    pub description: Option<String>,
    pub waiting_for: String,
    pub expected_at: Option<String>,
    pub follow_up_at: Option<String>,
    pub source_task_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WaitingQuery {
    pub view: Option<String>,
    pub now: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResolveWaitingInput {
    pub resolution_summary: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskToWaitingInput {
    pub waiting_for: String,
    pub description: Option<String>,
    pub expected_at: Option<String>,
    pub follow_up_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertWaitingToTaskInput {
    pub project_id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub estimated_minutes: Option<i64>,
    pub due_at: Option<String>,
    pub scheduled_start_at: Option<String>,
    pub scheduled_end_at: Option<String>,
    pub timezone: Option<String>,
    pub context: Option<String>,
    pub resolve_source: Option<bool>,
}

fn error(kind: ExecutionErrorKind, message: impl Into<String>) -> ExecutionError {
    ExecutionError {
        kind,
        message: message.into(),
    }
}

fn storage(message: impl Into<String>) -> ExecutionError {
    error(ExecutionErrorKind::Storage, message)
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

fn validate_timestamp(value: &Option<String>, label: &str) -> ExecutionResult<()> {
    if let Some(value) = value {
        DateTime::parse_from_rfc3339(value)
            .map_err(|_| validation(format!("{label}必须是 RFC3339 时间")))?;
    }
    Ok(())
}

fn normalize(
    user_id: String,
    id: Option<String>,
    input: WaitingItemInput,
    current: Option<&WaitingItemRecord>,
) -> ExecutionResult<WaitingItemWrite> {
    validate_timestamp(&input.expected_at, "预计返回时间")?;
    validate_timestamp(&input.follow_up_at, "跟进时间")?;
    Ok(WaitingItemWrite {
        id,
        user_id,
        title: clean_required(&input.title, "等待事项标题", 240)?,
        description: clean_optional(input.description, "等待事项描述", 20_000)?,
        status: current
            .map(|item| item.status.clone())
            .unwrap_or_else(|| "open".to_owned()),
        waiting_for: clean_required(&input.waiting_for, "等待对象", 240)?,
        expected_at: input.expected_at,
        follow_up_at: input.follow_up_at,
        resolved_at: current.and_then(|item| item.resolved_at.clone()),
        resolution_summary: current.and_then(|item| item.resolution_summary.clone()),
        source_task_id: clean_optional(input.source_task_id, "来源任务 ID", 128)?,
    })
}

fn parse_now(value: Option<String>) -> ExecutionResult<DateTime<FixedOffset>> {
    match value {
        Some(value) => {
            DateTime::parse_from_rfc3339(&value).map_err(|_| validation("now 必须是 RFC3339 时间"))
        }
        None => Ok(Utc::now().fixed_offset()),
    }
}

fn timestamp_in_offset(value: &str, offset: &FixedOffset) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(offset))
}

fn filter_view(
    items: Vec<WaitingItemRecord>,
    view: &str,
    now: DateTime<FixedOffset>,
) -> ExecutionResult<Vec<WaitingItemRecord>> {
    let today = now.date_naive();
    let days_to_sunday = 7_i64 - i64::from(today.weekday().number_from_monday());
    let week_end = today + Duration::days(days_to_sunday);
    let filtered = match view {
        "all" => items,
        "open" => items
            .into_iter()
            .filter(|item| item.status == "open")
            .collect(),
        "today" => items
            .into_iter()
            .filter(|item| {
                item.status == "open"
                    && item
                        .follow_up_at
                        .as_deref()
                        .and_then(|value| timestamp_in_offset(value, now.offset()))
                        .is_some_and(|value| value.date_naive() == today)
            })
            .collect(),
        "overdue" => items
            .into_iter()
            .filter(|item| {
                item.status == "open"
                    && item
                        .expected_at
                        .as_deref()
                        .and_then(|value| timestamp_in_offset(value, now.offset()))
                        .is_some_and(|value| value < now)
            })
            .collect(),
        "this_week" => items
            .into_iter()
            .filter(|item| {
                item.status == "open"
                    && item
                        .expected_at
                        .as_deref()
                        .and_then(|value| timestamp_in_offset(value, now.offset()))
                        .is_some_and(|value| {
                            let date = value.date_naive();
                            date >= today && date <= week_end
                        })
            })
            .collect(),
        "resolved" => items
            .into_iter()
            .filter(|item| item.status == "resolved")
            .collect(),
        "cancelled" => items
            .into_iter()
            .filter(|item| item.status == "cancelled")
            .collect(),
        _ => {
            return Err(validation(
                "view 必须是 all/open/today/overdue/this_week/resolved/cancelled",
            ))
        }
    };
    Ok(filtered)
}

fn ensure_source_task(
    connection: &Connection,
    user_id: &str,
    source_task_id: Option<&str>,
) -> ExecutionResult<()> {
    let Some(task_id) = source_task_id else {
        return Ok(());
    };
    if task_repository::get_task(connection, user_id, task_id)
        .map_err(storage)?
        .is_none()
    {
        return Err(validation("来源任务不存在或不属于当前资料"));
    }
    Ok(())
}

pub fn list_waiting_items(
    connection: &Connection,
    query: WaitingQuery,
) -> ExecutionResult<Vec<WaitingItemRecord>> {
    let user_id = active_user(connection)?;
    let items = repository::list_waiting_items(connection, &user_id, &WaitingListFilter::default())
        .map_err(storage)?;
    let now = parse_now(query.now)?;
    filter_view(items, query.view.as_deref().unwrap_or("open"), now)
}

pub fn get_waiting_item(connection: &Connection, id: &str) -> ExecutionResult<WaitingItemRecord> {
    let user_id = active_user(connection)?;
    repository::get_waiting_item(connection, &user_id, id)
        .map_err(storage)?
        .ok_or_else(|| not_found("等待事项不存在"))
}

pub fn create_waiting_item(
    connection: &Connection,
    input: WaitingItemInput,
) -> ExecutionResult<WaitingItemRecord> {
    let user_id = active_user(connection)?;
    let write = normalize(user_id.clone(), None, input, None)?;
    ensure_source_task(connection, &user_id, write.source_task_id.as_deref())?;
    repository::save_waiting_item(connection, &write).map_err(storage)
}

pub fn update_waiting_item(
    connection: &Connection,
    id: &str,
    input: WaitingItemInput,
) -> ExecutionResult<WaitingItemRecord> {
    let user_id = active_user(connection)?;
    let current = repository::get_waiting_item(connection, &user_id, id)
        .map_err(storage)?
        .ok_or_else(|| not_found("等待事项不存在"))?;
    if current.status != "open" {
        return Err(conflict("只有 open 状态的等待事项可以直接编辑"));
    }
    let write = normalize(user_id.clone(), Some(id.to_owned()), input, Some(&current))?;
    ensure_source_task(connection, &user_id, write.source_task_id.as_deref())?;
    repository::save_waiting_item(connection, &write).map_err(storage)
}

pub fn resolve_waiting_item(
    connection: &Connection,
    id: &str,
    input: ResolveWaitingInput,
) -> ExecutionResult<WaitingItemRecord> {
    let user_id = active_user(connection)?;
    let current = repository::get_waiting_item(connection, &user_id, id)
        .map_err(storage)?
        .ok_or_else(|| not_found("等待事项不存在"))?;
    if current.status == "resolved" {
        return Ok(current);
    }
    if current.status != "open" {
        return Err(conflict("只有 open 状态的等待事项可以标记 resolved"));
    }
    let write = WaitingItemWrite {
        id: Some(current.id),
        user_id,
        title: current.title,
        description: current.description,
        status: "resolved".to_owned(),
        waiting_for: current.waiting_for,
        expected_at: current.expected_at,
        follow_up_at: current.follow_up_at,
        resolved_at: Some(Utc::now().to_rfc3339()),
        resolution_summary: clean_optional(input.resolution_summary, "解决摘要", 10_000)?,
        source_task_id: current.source_task_id,
    };
    repository::save_waiting_item(connection, &write).map_err(storage)
}

pub fn cancel_waiting_item(
    connection: &Connection,
    id: &str,
) -> ExecutionResult<WaitingItemRecord> {
    let user_id = active_user(connection)?;
    let current = repository::get_waiting_item(connection, &user_id, id)
        .map_err(storage)?
        .ok_or_else(|| not_found("等待事项不存在"))?;
    if current.status == "cancelled" {
        return Ok(current);
    }
    if current.status != "open" {
        return Err(conflict("只有 open 状态的等待事项可以取消"));
    }
    let write = WaitingItemWrite {
        id: Some(current.id),
        user_id,
        title: current.title,
        description: current.description,
        status: "cancelled".to_owned(),
        waiting_for: current.waiting_for,
        expected_at: current.expected_at,
        follow_up_at: current.follow_up_at,
        resolved_at: None,
        resolution_summary: current.resolution_summary,
        source_task_id: current.source_task_id,
    };
    repository::save_waiting_item(connection, &write).map_err(storage)
}

pub fn delete_waiting_item(connection: &Connection, id: &str) -> ExecutionResult<()> {
    let user_id = active_user(connection)?;
    if repository::get_waiting_item(connection, &user_id, id)
        .map_err(storage)?
        .is_none()
    {
        return Err(not_found("等待事项不存在"));
    }
    if repository::soft_delete_waiting_item(connection, &user_id, id).map_err(storage)? {
        Ok(())
    } else {
        Err(not_found("等待事项不存在"))
    }
}

pub fn create_waiting_from_task(
    connection: &Connection,
    task_id: &str,
    input: TaskToWaitingInput,
) -> ExecutionResult<WaitingItemRecord> {
    let user_id = active_user(connection)?;
    let task = execution::get_task(connection, task_id)?;
    if matches!(task.status.as_str(), "done" | "cancelled") {
        return Err(conflict("已完成或已取消的任务不能转入等待"));
    }
    validate_timestamp(&input.expected_at, "预计返回时间")?;
    validate_timestamp(&input.follow_up_at, "跟进时间")?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| storage(error.to_string()))?;

    let existing =
        repository::find_open_by_source_task(&transaction, &user_id, task_id).map_err(storage)?;
    let waiting = if let Some(existing) = existing {
        existing
    } else {
        let write = WaitingItemWrite {
            id: None,
            user_id: user_id.clone(),
            title: task.title.clone(),
            description: clean_optional(input.description, "等待事项描述", 20_000)?
                .or(task.description.clone()),
            status: "open".to_owned(),
            waiting_for: clean_required(&input.waiting_for, "等待对象", 240)?,
            expected_at: input.expected_at,
            follow_up_at: input.follow_up_at,
            resolved_at: None,
            resolution_summary: None,
            source_task_id: Some(task_id.to_owned()),
        };
        repository::save_waiting_item(&transaction, &write).map_err(storage)?
    };

    if task.status != "waiting" {
        execution::change_task_status(
            &transaction,
            task_id,
            TaskStatusInput {
                status: "waiting".to_owned(),
            },
        )?;
    }
    transaction
        .commit()
        .map_err(|error| storage(error.to_string()))?;
    Ok(waiting)
}

pub fn convert_waiting_to_task(
    connection: &Connection,
    waiting_item_id: &str,
    input: ConvertWaitingToTaskInput,
) -> ExecutionResult<task_repository::TaskRecord> {
    let user_id = active_user(connection)?;
    let waiting = repository::get_waiting_item(connection, &user_id, waiting_item_id)
        .map_err(storage)?
        .ok_or_else(|| not_found("等待事项不存在"))?;

    if let Some(existing_task_id) =
        repository::find_conversion_target_task_id(connection, &user_id, waiting_item_id)
            .map_err(storage)?
    {
        return task_repository::get_task(connection, &user_id, &existing_task_id)
            .map_err(storage)?
            .ok_or_else(|| conflict("等待事项已转换过，但目标任务已不存在"));
    }

    let task_input = TaskInput {
        project_id: input.project_id,
        title: clean_optional(input.title, "任务标题", 240)?
            .unwrap_or_else(|| waiting.title.clone()),
        description: clean_optional(input.description, "任务描述", 20_000)?
            .or(waiting.description.clone()),
        priority: input.priority,
        estimated_minutes: input.estimated_minutes,
        actual_minutes: None,
        due_at: input.due_at,
        scheduled_start_at: input.scheduled_start_at,
        scheduled_end_at: input.scheduled_end_at,
        timezone: input.timezone,
        context: input.context,
    };

    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| storage(error.to_string()))?;
    let task = execution::create_task(&transaction, task_input)?;
    repository::create_conversion_links(&transaction, &user_id, waiting_item_id, &task.id)
        .map_err(storage)?;
    if input.resolve_source.unwrap_or(true) && waiting.status == "open" {
        resolve_waiting_item(
            &transaction,
            waiting_item_id,
            ResolveWaitingInput {
                resolution_summary: Some(format!("已转为任务 {}", task.id)),
            },
        )?;
    }
    transaction
        .commit()
        .map_err(|error| storage(error.to_string()))?;
    Ok(task)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migration_runner::{run, MigrationContext};
    use crate::database::migrations::all;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn database() -> Connection {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("lifetrace-waiting-service-{unique}"));
        std::fs::create_dir_all(&data_dir).unwrap();
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run(&mut connection, &MigrationContext::new(data_dir), &all()).unwrap();
        connection
    }

    fn task_input(title: &str) -> TaskInput {
        TaskInput {
            project_id: None,
            title: title.to_owned(),
            description: None,
            priority: None,
            estimated_minutes: None,
            actual_minutes: None,
            due_at: None,
            scheduled_start_at: None,
            scheduled_end_at: None,
            timezone: None,
            context: None,
        }
    }

    fn waiting_input(title: &str) -> WaitingItemInput {
        WaitingItemInput {
            title: title.to_owned(),
            description: None,
            waiting_for: "Alice".to_owned(),
            expected_at: Some("2026-08-10T09:00:00+08:00".to_owned()),
            follow_up_at: Some("2026-08-09T09:00:00+08:00".to_owned()),
            source_task_id: None,
        }
    }

    #[test]
    fn waiting_lifecycle_is_explicit_and_idempotent() {
        let connection = database();
        let item = create_waiting_item(&connection, waiting_input("等待确认")).unwrap();
        let resolved = resolve_waiting_item(
            &connection,
            &item.id,
            ResolveWaitingInput {
                resolution_summary: Some("已确认".to_owned()),
            },
        )
        .unwrap();
        assert_eq!(resolved.status, "resolved");
        assert!(resolved.resolved_at.is_some());
        let second =
            resolve_waiting_item(&connection, &item.id, ResolveWaitingInput::default()).unwrap();
        assert_eq!(second.version, resolved.version);
        assert!(update_waiting_item(&connection, &item.id, waiting_input("不能编辑")).is_err());
    }

    #[test]
    fn task_to_waiting_is_atomic_and_does_not_duplicate_open_item() {
        let connection = database();
        let task = execution::create_task(&connection, task_input("等待对方回复")).unwrap();
        let input = TaskToWaitingInput {
            waiting_for: "Bob".to_owned(),
            description: None,
            expected_at: None,
            follow_up_at: None,
        };
        let first = create_waiting_from_task(&connection, &task.id, input.clone()).unwrap();
        let second = create_waiting_from_task(&connection, &task.id, input).unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(
            execution::get_task(&connection, &task.id).unwrap().status,
            "waiting"
        );
    }

    #[test]
    fn waiting_to_task_conversion_is_idempotent_and_preserves_links() {
        let connection = database();
        let waiting = create_waiting_item(&connection, waiting_input("下一步行动")).unwrap();
        let input = ConvertWaitingToTaskInput {
            project_id: None,
            title: None,
            description: None,
            priority: Some("high".to_owned()),
            estimated_minutes: None,
            due_at: None,
            scheduled_start_at: None,
            scheduled_end_at: None,
            timezone: None,
            context: None,
            resolve_source: Some(true),
        };
        let first = convert_waiting_to_task(&connection, &waiting.id, input.clone()).unwrap();
        let second = convert_waiting_to_task(&connection, &waiting.id, input).unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(first.title, waiting.title);
        assert_eq!(
            get_waiting_item(&connection, &waiting.id).unwrap().status,
            "resolved"
        );
        let link_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM execution_entity_links WHERE (source_id=?1 OR target_id=?1) AND deleted_at IS NULL",
                [waiting.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(link_count, 2);
    }

    #[test]
    fn overview_views_use_the_callers_offset_for_today() {
        let connection = database();
        let mut today = waiting_input("今天跟进");
        today.follow_up_at = Some("2026-08-08T23:30:00+08:00".to_owned());
        create_waiting_item(&connection, today).unwrap();
        let items = list_waiting_items(
            &connection,
            WaitingQuery {
                view: Some("today".to_owned()),
                now: Some("2026-08-08T12:00:00+08:00".to_owned()),
            },
        )
        .unwrap();
        assert_eq!(items.len(), 1);
    }
}
