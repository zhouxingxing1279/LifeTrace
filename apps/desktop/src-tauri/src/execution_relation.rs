use rusqlite::{params, Connection};
use serde::Deserialize;

use crate::{
    database::{
        profile,
        repositories::{
            execution as task_repository,
            execution_relation::{
                self as repository, CompletionResultRecord, CompletionResultWrite,
                EntityLinkRecord, EntityLinkWrite,
            },
        },
    },
    execution::{ExecutionError, ExecutionErrorKind, ExecutionResult},
};

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CompletionResultInput {
    pub summary: Option<String>,
    pub actual_minutes: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityLinksQuery {
    pub entity_type: String,
    pub entity_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityLinkInput {
    pub source_type: String,
    pub source_id: String,
    pub relation_type: String,
    pub target_type: String,
    pub target_id: String,
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

fn execution_table(entity_type: &str) -> Option<&'static str> {
    match entity_type {
        "project" => Some("execution_projects"),
        "task" => Some("execution_tasks"),
        "task_occurrence" => Some("execution_task_occurrences"),
        "waiting_item" => Some("execution_waiting_items"),
        "calendar_event" => Some("execution_calendar_events"),
        "calendar_occurrence" => Some("execution_calendar_occurrences"),
        "memo" => Some("execution_memos"),
        "reminder" => Some("execution_reminders"),
        _ => None,
    }
}

fn ensure_execution_entity(
    connection: &Connection,
    user_id: &str,
    entity_type: &str,
    entity_id: &str,
) -> ExecutionResult<bool> {
    let Some(table) = execution_table(entity_type) else {
        return Ok(false);
    };
    let sql = format!(
        "SELECT EXISTS(SELECT 1 FROM {table} WHERE id=?1 AND user_id=?2 AND deleted_at IS NULL)"
    );
    let exists: bool = connection
        .query_row(&sql, params![entity_id, user_id], |row| row.get(0))
        .map_err(|error| storage(error.to_string()))?;
    if !exists {
        return Err(not_found(format!(
            "关联对象不存在：{entity_type}/{entity_id}"
        )));
    }
    Ok(true)
}

fn validate_relation_type(value: &str) -> ExecutionResult<String> {
    let value = clean_required(value, "relationType", 64)?;
    if !matches!(
        value.as_str(),
        "related_to" | "derived_from" | "converted_to" | "attachment" | "reference"
    ) {
        return Err(validation(
            "relationType 必须是 related_to/derived_from/converted_to/attachment/reference",
        ));
    }
    Ok(value)
}

pub fn get_completion_result(
    connection: &Connection,
    task_id: &str,
) -> ExecutionResult<Option<CompletionResultRecord>> {
    let user_id = active_user(connection)?;
    task_repository::get_task(connection, &user_id, task_id)
        .map_err(storage)?
        .ok_or_else(|| not_found("任务不存在"))?;
    repository::get_completion_result(connection, &user_id, task_id).map_err(storage)
}

pub fn save_completion_result(
    connection: &Connection,
    task_id: &str,
    input: CompletionResultInput,
) -> ExecutionResult<CompletionResultRecord> {
    let user_id = active_user(connection)?;
    let task = task_repository::get_task(connection, &user_id, task_id)
        .map_err(storage)?
        .ok_or_else(|| not_found("任务不存在"))?;
    if task.status != "done" {
        return Err(validation("只有已完成任务可以记录完成结果"));
    }
    if input.actual_minutes.is_some_and(|value| value < 0) {
        return Err(validation("actualMinutes 不能小于 0"));
    }
    let completed_at = task
        .completed_at
        .clone()
        .ok_or_else(|| validation("已完成任务缺少 completedAt"))?;
    repository::save_completion_result(
        connection,
        &CompletionResultWrite {
            user_id,
            task_id: task.id,
            summary: clean_optional(input.summary, "完成总结", 20_000)?,
            completed_at,
            actual_minutes: input.actual_minutes.or(task.actual_minutes),
        },
    )
    .map_err(storage)
}

pub fn ensure_completion_for_task(
    connection: &Connection,
    user_id: &str,
    task_id: &str,
    completed_at: &str,
    actual_minutes: Option<i64>,
) -> ExecutionResult<CompletionResultRecord> {
    let summary = repository::get_completion_result(connection, user_id, task_id)
        .map_err(storage)?
        .and_then(|result| result.summary);
    repository::save_completion_result(
        connection,
        &CompletionResultWrite {
            user_id: user_id.to_owned(),
            task_id: task_id.to_owned(),
            summary,
            completed_at: completed_at.to_owned(),
            actual_minutes,
        },
    )
    .map_err(storage)
}

pub fn clear_completion_for_task(
    connection: &Connection,
    user_id: &str,
    task_id: &str,
) -> ExecutionResult<()> {
    repository::soft_delete_completion_result(connection, user_id, task_id)
        .map(|_| ())
        .map_err(storage)
}

pub fn list_links(
    connection: &Connection,
    query: EntityLinksQuery,
) -> ExecutionResult<Vec<EntityLinkRecord>> {
    let user_id = active_user(connection)?;
    let entity_type = clean_required(&query.entity_type, "entityType", 64)?;
    let entity_id = clean_required(&query.entity_id, "entityId", 128)?;
    let _ = ensure_execution_entity(connection, &user_id, &entity_type, &entity_id)?;
    repository::list_links_for_entity(connection, &user_id, &entity_type, &entity_id)
        .map_err(storage)
}

pub fn create_link(
    connection: &Connection,
    input: EntityLinkInput,
) -> ExecutionResult<EntityLinkRecord> {
    let user_id = active_user(connection)?;
    let source_type = clean_required(&input.source_type, "sourceType", 64)?;
    let source_id = clean_required(&input.source_id, "sourceId", 128)?;
    let target_type = clean_required(&input.target_type, "targetType", 64)?;
    let target_id = clean_required(&input.target_id, "targetId", 128)?;
    if source_type == target_type && source_id == target_id {
        return Err(validation("不能把实体关联到自身"));
    }
    let source_execution = ensure_execution_entity(connection, &user_id, &source_type, &source_id)?;
    let target_execution = ensure_execution_entity(connection, &user_id, &target_type, &target_id)?;
    if !source_execution && !target_execution {
        return Err(validation("至少一端必须是执行系统实体"));
    }
    repository::save_link(
        connection,
        &EntityLinkWrite {
            user_id,
            source_type,
            source_id,
            relation_type: validate_relation_type(&input.relation_type)?,
            target_type,
            target_id,
        },
    )
    .map_err(storage)
}

pub fn delete_link(connection: &Connection, link_id: &str) -> ExecutionResult<()> {
    let user_id = active_user(connection)?;
    if repository::soft_delete_link(connection, &user_id, link_id).map_err(storage)? {
        Ok(())
    } else {
        Err(not_found("关联不存在"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        database::{
            migration_runner::{run, MigrationContext},
            migrations::all,
        },
        execution::{self, ProjectInput, TaskInput, TaskStatusInput},
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    fn database() -> Connection {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("lifetrace-execution-relation-service-{unique}"));
        std::fs::create_dir_all(&directory).unwrap();
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run(&mut connection, &MigrationContext::new(directory), &all()).unwrap();
        connection
    }

    fn task_input(title: &str) -> TaskInput {
        TaskInput {
            project_id: None,
            title: title.to_owned(),
            description: None,
            priority: None,
            estimated_minutes: None,
            actual_minutes: Some(25),
            due_at: None,
            scheduled_start_at: None,
            scheduled_end_at: None,
            timezone: None,
            context: None,
        }
    }

    #[test]
    fn completion_result_requires_done_task_and_is_editable() {
        let connection = database();
        let task = execution::create_task(&connection, task_input("完成结果")).unwrap();
        assert!(
            save_completion_result(&connection, &task.id, CompletionResultInput::default())
                .is_err()
        );
        execution::change_task_status(
            &connection,
            &task.id,
            TaskStatusInput {
                status: "done".to_owned(),
            },
        )
        .unwrap();
        let automatic = get_completion_result(&connection, &task.id)
            .unwrap()
            .unwrap();
        assert_eq!(automatic.actual_minutes, Some(25));
        let result = save_completion_result(
            &connection,
            &task.id,
            CompletionResultInput {
                summary: Some("完成并验证".to_owned()),
                actual_minutes: Some(30),
            },
        )
        .unwrap();
        assert_eq!(result.summary.as_deref(), Some("完成并验证"));
        assert_eq!(result.actual_minutes, Some(30));
        execution::change_task_status(
            &connection,
            &task.id,
            TaskStatusInput {
                status: "todo".to_owned(),
            },
        )
        .unwrap();
        assert!(
            get_completion_result(&connection, &task.id)
                .unwrap()
                .is_none(),
            "completion is cleared when task reopens"
        );
    }

    #[test]
    fn relation_requires_at_least_one_existing_execution_entity() {
        let connection = database();
        let task = execution::create_task(&connection, task_input("关联任务")).unwrap();
        let link = create_link(
            &connection,
            EntityLinkInput {
                source_type: "task".to_owned(),
                source_id: task.id.clone(),
                relation_type: "related_to".to_owned(),
                target_type: "note".to_owned(),
                target_id: "note-1".to_owned(),
            },
        )
        .unwrap();
        assert_eq!(link.target_type, "note");
        assert_eq!(
            list_links(
                &connection,
                EntityLinksQuery {
                    entity_type: "task".to_owned(),
                    entity_id: task.id
                }
            )
            .unwrap()
            .len(),
            1
        );
        assert!(create_link(
            &connection,
            EntityLinkInput {
                source_type: "note".to_owned(),
                source_id: "a".to_owned(),
                relation_type: "related_to".to_owned(),
                target_type: "habit".to_owned(),
                target_id: "b".to_owned()
            }
        )
        .is_err());
    }

    #[test]
    fn internal_execution_targets_must_exist() {
        let connection = database();
        let project = execution::create_project(
            &connection,
            ProjectInput {
                name: "关联项目".to_owned(),
                description: None,
                status: None,
                color: None,
                icon: None,
                sort_order: None,
            },
        )
        .unwrap();
        assert!(create_link(
            &connection,
            EntityLinkInput {
                source_type: "project".to_owned(),
                source_id: project.id,
                relation_type: "reference".to_owned(),
                target_type: "task".to_owned(),
                target_id: "missing".to_owned()
            }
        )
        .is_err());
    }
}
