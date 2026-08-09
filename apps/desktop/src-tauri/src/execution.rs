use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::Deserialize;

use crate::database::{
    profile,
    repositories::execution::{
        self as repository, ProjectRecord, ProjectWrite, TaskListFilter, TaskRecord, TaskWrite,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionErrorKind {
    Validation,
    NotFound,
    Conflict,
    Storage,
}

#[derive(Debug, Clone)]
pub struct ExecutionError {
    pub kind: ExecutionErrorKind,
    pub message: String,
}

impl ExecutionError {
    fn validation(message: impl Into<String>) -> Self {
        Self {
            kind: ExecutionErrorKind::Validation,
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            kind: ExecutionErrorKind::NotFound,
            message: message.into(),
        }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            kind: ExecutionErrorKind::Conflict,
            message: message.into(),
        }
    }

    fn storage(message: impl Into<String>) -> Self {
        Self {
            kind: ExecutionErrorKind::Storage,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for ExecutionError {}

pub type ExecutionResult<T> = Result<T, ExecutionError>;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInput {
    pub name: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub sort_order: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskInput {
    pub project_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub estimated_minutes: Option<i64>,
    pub actual_minutes: Option<i64>,
    pub due_at: Option<String>,
    pub scheduled_start_at: Option<String>,
    pub scheduled_end_at: Option<String>,
    pub timezone: Option<String>,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TaskQuery {
    pub status: Option<String>,
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskStatusInput {
    pub status: String,
}

fn active_user(connection: &Connection) -> ExecutionResult<String> {
    profile::active_profile_id(connection).map_err(ExecutionError::storage)
}

fn clean_required(value: &str, label: &str, max: usize) -> ExecutionResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ExecutionError::validation(format!("{label}不能为空")));
    }
    if value.chars().count() > max {
        return Err(ExecutionError::validation(format!(
            "{label}不能超过 {max} 个字符"
        )));
    }
    Ok(value.to_owned())
}

fn clean_optional(
    value: Option<String>,
    max: usize,
    label: &str,
) -> ExecutionResult<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > max {
        return Err(ExecutionError::validation(format!(
            "{label}不能超过 {max} 个字符"
        )));
    }
    Ok(Some(value.to_owned()))
}

fn validate_project_status(status: &str) -> ExecutionResult<()> {
    if matches!(status, "active" | "completed" | "archived" | "cancelled") {
        Ok(())
    } else {
        Err(ExecutionError::validation("无效的项目状态"))
    }
}

fn validate_task_status(status: &str) -> ExecutionResult<()> {
    if matches!(
        status,
        "todo" | "in_progress" | "waiting" | "done" | "cancelled"
    ) {
        Ok(())
    } else {
        Err(ExecutionError::validation("无效的任务状态"))
    }
}

fn validate_priority(priority: &str) -> ExecutionResult<()> {
    if matches!(priority, "low" | "normal" | "high" | "urgent") {
        Ok(())
    } else {
        Err(ExecutionError::validation("无效的任务优先级"))
    }
}

fn parse_optional_timestamp(
    value: &Option<String>,
    label: &str,
) -> ExecutionResult<Option<DateTime<Utc>>> {
    value
        .as_ref()
        .map(|value| {
            DateTime::parse_from_rfc3339(value)
                .map(|date| date.with_timezone(&Utc))
                .map_err(|_| ExecutionError::validation(format!("{label}必须是 RFC3339 时间")))
        })
        .transpose()
}

fn validate_task_time(input: &TaskInput) -> ExecutionResult<()> {
    let _due = parse_optional_timestamp(&input.due_at, "截止时间")?;
    let start = parse_optional_timestamp(&input.scheduled_start_at, "计划开始时间")?;
    let end = parse_optional_timestamp(&input.scheduled_end_at, "计划结束时间")?;
    if start.is_some() != end.is_some() {
        return Err(ExecutionError::validation(
            "计划开始时间和结束时间必须同时设置".to_owned(),
        ));
    }
    if let (Some(start), Some(end)) = (start, end) {
        if end < start {
            return Err(ExecutionError::validation(
                "计划结束时间不能早于开始时间".to_owned(),
            ));
        }
    }
    if let Some(minutes) = input.estimated_minutes {
        if minutes < 0 {
            return Err(ExecutionError::validation("预计时长不能为负数"));
        }
    }
    if let Some(minutes) = input.actual_minutes {
        if minutes < 0 {
            return Err(ExecutionError::validation("实际时长不能为负数"));
        }
    }
    Ok(())
}

fn normalize_project_input(
    user_id: String,
    id: Option<String>,
    input: ProjectInput,
    default_status: &str,
) -> ExecutionResult<ProjectWrite> {
    let status = input.status.unwrap_or_else(|| default_status.to_owned());
    validate_project_status(&status)?;
    Ok(ProjectWrite {
        id,
        user_id,
        name: clean_required(&input.name, "项目名称", 120)?,
        description: clean_optional(input.description, 10_000, "项目描述")?,
        status,
        color: clean_optional(input.color, 64, "项目颜色")?,
        icon: clean_optional(input.icon, 64, "项目图标")?,
        sort_order: input.sort_order.unwrap_or(0),
    })
}

fn normalize_task_input(
    user_id: String,
    id: Option<String>,
    input: TaskInput,
    current: Option<&TaskRecord>,
) -> ExecutionResult<TaskWrite> {
    validate_task_time(&input)?;
    let priority = input.priority.unwrap_or_else(|| "normal".to_owned());
    validate_priority(&priority)?;
    let status = current
        .map(|task| task.status.clone())
        .unwrap_or_else(|| "todo".to_owned());
    Ok(TaskWrite {
        id,
        user_id,
        project_id: clean_optional(input.project_id, 128, "项目 ID")?,
        parent_task_id: current.and_then(|task| task.parent_task_id.clone()),
        title: clean_required(&input.title, "任务标题", 240)?,
        description: clean_optional(input.description, 20_000, "任务描述")?,
        status,
        priority,
        estimated_minutes: input.estimated_minutes,
        actual_minutes: input.actual_minutes,
        due_at: input.due_at,
        scheduled_start_at: input.scheduled_start_at,
        scheduled_end_at: input.scheduled_end_at,
        timezone: clean_optional(input.timezone, 128, "时区")?,
        context: clean_optional(input.context, 512, "上下文")?,
        completed_at: current.and_then(|task| task.completed_at.clone()),
        cancelled_at: current.and_then(|task| task.cancelled_at.clone()),
    })
}

fn ensure_project_reference(
    connection: &Connection,
    user_id: &str,
    project_id: Option<&str>,
) -> ExecutionResult<()> {
    let Some(project_id) = project_id else {
        return Ok(());
    };
    let project = repository::get_project(connection, user_id, project_id)
        .map_err(ExecutionError::storage)?;
    if project.is_none() {
        return Err(ExecutionError::validation("关联项目不存在或不属于当前资料"));
    }
    Ok(())
}

pub fn list_projects(connection: &Connection) -> ExecutionResult<Vec<ProjectRecord>> {
    let user_id = active_user(connection)?;
    repository::list_projects(connection, &user_id).map_err(ExecutionError::storage)
}

pub fn get_project(connection: &Connection, id: &str) -> ExecutionResult<ProjectRecord> {
    let user_id = active_user(connection)?;
    repository::get_project(connection, &user_id, id)
        .map_err(ExecutionError::storage)?
        .ok_or_else(|| ExecutionError::not_found("项目不存在"))
}

pub fn create_project(
    connection: &Connection,
    input: ProjectInput,
) -> ExecutionResult<ProjectRecord> {
    let user_id = active_user(connection)?;
    let input = normalize_project_input(user_id, None, input, "active")?;
    repository::save_project(connection, &input).map_err(ExecutionError::storage)
}

pub fn update_project(
    connection: &Connection,
    id: &str,
    input: ProjectInput,
) -> ExecutionResult<ProjectRecord> {
    let user_id = active_user(connection)?;
    let current = repository::get_project(connection, &user_id, id)
        .map_err(ExecutionError::storage)?
        .ok_or_else(|| ExecutionError::not_found("项目不存在"))?;
    let input = normalize_project_input(user_id, Some(id.to_owned()), input, &current.status)?;
    repository::save_project(connection, &input).map_err(ExecutionError::storage)
}

pub fn delete_project(connection: &Connection, id: &str) -> ExecutionResult<()> {
    let user_id = active_user(connection)?;
    if repository::get_project(connection, &user_id, id)
        .map_err(ExecutionError::storage)?
        .is_none()
    {
        return Err(ExecutionError::not_found("项目不存在"));
    }
    let active_tasks = repository::list_tasks(
        connection,
        &user_id,
        &TaskListFilter {
            project_id: Some(id.to_owned()),
            ..TaskListFilter::default()
        },
    )
    .map_err(ExecutionError::storage)?;
    if !active_tasks.is_empty() {
        return Err(ExecutionError::conflict(
            "项目仍有关联任务，请先移动或删除这些任务".to_owned(),
        ));
    }
    if repository::soft_delete_project(connection, &user_id, id).map_err(ExecutionError::storage)? {
        Ok(())
    } else {
        Err(ExecutionError::not_found("项目不存在"))
    }
}

pub fn list_tasks(connection: &Connection, query: TaskQuery) -> ExecutionResult<Vec<TaskRecord>> {
    if let Some(status) = query.status.as_deref() {
        validate_task_status(status)?;
    }
    let user_id = active_user(connection)?;
    if let Some(project_id) = query.project_id.as_deref() {
        ensure_project_reference(connection, &user_id, Some(project_id))?;
    }
    repository::list_tasks(
        connection,
        &user_id,
        &TaskListFilter {
            status: query.status,
            project_id: query.project_id,
            parent_task_id: None,
        },
    )
    .map_err(ExecutionError::storage)
}

pub fn get_task(connection: &Connection, id: &str) -> ExecutionResult<TaskRecord> {
    let user_id = active_user(connection)?;
    repository::get_task(connection, &user_id, id)
        .map_err(ExecutionError::storage)?
        .ok_or_else(|| ExecutionError::not_found("任务不存在"))
}

pub fn create_task(connection: &Connection, input: TaskInput) -> ExecutionResult<TaskRecord> {
    let user_id = active_user(connection)?;
    let input = normalize_task_input(user_id.clone(), None, input, None)?;
    ensure_project_reference(connection, &user_id, input.project_id.as_deref())?;
    repository::save_task(connection, &input).map_err(ExecutionError::storage)
}

pub fn update_task(
    connection: &Connection,
    id: &str,
    input: TaskInput,
) -> ExecutionResult<TaskRecord> {
    let user_id = active_user(connection)?;
    let current = repository::get_task(connection, &user_id, id)
        .map_err(ExecutionError::storage)?
        .ok_or_else(|| ExecutionError::not_found("任务不存在"))?;
    let input = normalize_task_input(user_id.clone(), Some(id.to_owned()), input, Some(&current))?;
    ensure_project_reference(connection, &user_id, input.project_id.as_deref())?;
    repository::save_task(connection, &input).map_err(ExecutionError::storage)
}

fn transition_allowed(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        ("todo", "in_progress")
            | ("todo", "waiting")
            | ("todo", "done")
            | ("todo", "cancelled")
            | ("in_progress", "waiting")
            | ("in_progress", "done")
            | ("in_progress", "cancelled")
            | ("waiting", "in_progress")
            | ("waiting", "done")
            | ("waiting", "cancelled")
            | ("done", "todo")
    )
}

fn persist_task_status_change(
    connection: &Connection,
    user_id: &str,
    current: &TaskRecord,
    write: &TaskWrite,
) -> ExecutionResult<TaskRecord> {
    let saved = repository::save_task(connection, write).map_err(ExecutionError::storage)?;
    if saved.status == "done" {
        let completed_at = saved
            .completed_at
            .as_deref()
            .ok_or_else(|| ExecutionError::validation("已完成任务缺少 completedAt"))?;
        crate::execution_relation::ensure_completion_for_task(
            connection,
            user_id,
            &saved.id,
            completed_at,
            saved.actual_minutes,
        )?;
    } else if current.status == "done" {
        crate::execution_relation::clear_completion_for_task(connection, user_id, &saved.id)?;
    }
    Ok(saved)
}

pub fn change_task_status(
    connection: &Connection,
    id: &str,
    input: TaskStatusInput,
) -> ExecutionResult<TaskRecord> {
    validate_task_status(&input.status)?;
    let user_id = active_user(connection)?;
    let current = repository::get_task(connection, &user_id, id)
        .map_err(ExecutionError::storage)?
        .ok_or_else(|| ExecutionError::not_found("任务不存在"))?;
    if current.status == input.status {
        if current.status == "done"
            && crate::database::repositories::execution_relation::get_completion_result(
                connection, &user_id, id,
            )
            .map_err(ExecutionError::storage)?
            .is_none()
        {
            let completed_at = current
                .completed_at
                .as_deref()
                .ok_or_else(|| ExecutionError::validation("已完成任务缺少 completedAt"))?;
            crate::execution_relation::ensure_completion_for_task(
                connection,
                &user_id,
                id,
                completed_at,
                current.actual_minutes,
            )?;
        }
        return Ok(current);
    }
    if !transition_allowed(&current.status, &input.status) {
        return Err(ExecutionError::conflict(format!(
            "不允许从 {} 切换到 {}",
            current.status, input.status
        )));
    }
    let stamp = Utc::now().to_rfc3339();
    let write = TaskWrite {
        id: Some(current.id.clone()),
        user_id: user_id.clone(),
        project_id: current.project_id.clone(),
        parent_task_id: current.parent_task_id.clone(),
        title: current.title.clone(),
        description: current.description.clone(),
        status: input.status.clone(),
        priority: current.priority.clone(),
        estimated_minutes: current.estimated_minutes,
        actual_minutes: current.actual_minutes,
        due_at: current.due_at.clone(),
        scheduled_start_at: current.scheduled_start_at.clone(),
        scheduled_end_at: current.scheduled_end_at.clone(),
        timezone: current.timezone.clone(),
        context: current.context.clone(),
        completed_at: if input.status == "done" {
            Some(stamp.clone())
        } else if current.status == "done" {
            None
        } else {
            current.completed_at.clone()
        },
        cancelled_at: if input.status == "cancelled" {
            Some(stamp)
        } else {
            current.cancelled_at.clone()
        },
    };

    if !connection.is_autocommit() {
        return persist_task_status_change(connection, &user_id, &current, &write);
    }

    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| ExecutionError::storage(error.to_string()))?;
    let saved = persist_task_status_change(&transaction, &user_id, &current, &write)?;
    transaction
        .commit()
        .map_err(|error| ExecutionError::storage(error.to_string()))?;
    Ok(saved)
}

pub fn delete_task(connection: &Connection, id: &str) -> ExecutionResult<()> {
    let user_id = active_user(connection)?;
    if repository::get_task(connection, &user_id, id)
        .map_err(ExecutionError::storage)?
        .is_none()
    {
        return Err(ExecutionError::not_found("任务不存在"));
    }
    if repository::task_has_children(connection, &user_id, id).map_err(ExecutionError::storage)? {
        return Err(ExecutionError::conflict(
            "任务仍有子任务，不能直接删除".to_owned(),
        ));
    }
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| ExecutionError::storage(error.to_string()))?;
    if !repository::soft_delete_task(&transaction, &user_id, id).map_err(ExecutionError::storage)? {
        return Err(ExecutionError::not_found("任务不存在"));
    }
    crate::execution_relation::clear_completion_for_task(&transaction, &user_id, id)?;
    transaction
        .commit()
        .map_err(|error| ExecutionError::storage(error.to_string()))?;
    Ok(())
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
        let directory = std::env::temp_dir().join(format!("lifetrace-execution-service-{unique}"));
        std::fs::create_dir_all(&directory).unwrap();
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run(&mut connection, &MigrationContext::new(directory), &all()).unwrap();
        connection
    }

    fn project_input(name: &str) -> ProjectInput {
        ProjectInput {
            name: name.to_owned(),
            description: None,
            status: None,
            color: None,
            icon: None,
            sort_order: None,
        }
    }

    fn task_input(title: &str, project_id: Option<String>) -> TaskInput {
        TaskInput {
            project_id,
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

    #[test]
    fn project_and_task_crud_obeys_domain_rules() {
        let connection = database();
        let project = create_project(&connection, project_input("EPIC20")).unwrap();
        let task = create_task(
            &connection,
            task_input("Implement task", Some(project.id.clone())),
        )
        .unwrap();
        assert_eq!(task.status, "todo");
        assert!(delete_project(&connection, &project.id).is_err());
        delete_task(&connection, &task.id).unwrap();
        delete_project(&connection, &project.id).unwrap();
        assert!(get_project(&connection, &project.id).is_err());
    }

    #[test]
    fn task_status_machine_sets_and_resets_completion_timestamp() {
        let connection = database();
        let task = create_task(&connection, task_input("Status test", None)).unwrap();
        let in_progress = change_task_status(
            &connection,
            &task.id,
            TaskStatusInput {
                status: "in_progress".to_owned(),
            },
        )
        .unwrap();
        assert_eq!(in_progress.status, "in_progress");
        let done = change_task_status(
            &connection,
            &task.id,
            TaskStatusInput {
                status: "done".to_owned(),
            },
        )
        .unwrap();
        assert!(done.completed_at.is_some());
        let reopened = change_task_status(
            &connection,
            &task.id,
            TaskStatusInput {
                status: "todo".to_owned(),
            },
        )
        .unwrap();
        assert!(reopened.completed_at.is_none());
        assert_eq!(reopened.status, "todo");
    }

    #[test]
    fn invalid_status_transition_is_rejected() {
        let connection = database();
        let task = create_task(&connection, task_input("Transition test", None)).unwrap();
        let cancelled = change_task_status(
            &connection,
            &task.id,
            TaskStatusInput {
                status: "cancelled".to_owned(),
            },
        )
        .unwrap();
        assert!(cancelled.cancelled_at.is_some());
        let error = change_task_status(
            &connection,
            &task.id,
            TaskStatusInput {
                status: "in_progress".to_owned(),
            },
        )
        .unwrap_err();
        assert_eq!(error.kind, ExecutionErrorKind::Conflict);
    }

    #[test]
    fn task_time_window_and_project_ownership_are_validated() {
        let connection = database();
        let mut invalid = task_input("Bad time", None);
        invalid.scheduled_start_at = Some("2026-08-08T10:00:00Z".to_owned());
        assert!(create_task(&connection, invalid).is_err());

        let mut missing_project = task_input("Bad project", Some("missing".to_owned()));
        missing_project.priority = Some("urgent".to_owned());
        assert!(create_task(&connection, missing_project).is_err());
    }
}
