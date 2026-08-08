use std::collections::{HashMap, HashSet};

use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::{
    database::{
        profile,
        repositories::{
            execution::{self as task_repository, TaskListFilter, TaskRecord},
            execution_structure::{
                self as structure_repository, DependencyRecord, RecurrenceRuleRecord,
                RecurrenceRuleWrite, TaskOccurrenceRecord, TaskOccurrenceWrite,
            },
        },
    },
    execution::{self, ExecutionError, ExecutionErrorKind, ExecutionResult, TaskInput},
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyInput {
    pub depends_on_task_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskBlocker {
    pub task_id: String,
    pub title: String,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceRuleInput {
    pub frequency: String,
    pub interval_value: Option<i64>,
    #[serde(default)]
    pub weekdays: Vec<u8>,
    pub month_day: Option<i64>,
    pub timezone: Option<String>,
    pub until_at: Option<String>,
    pub max_occurrences: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OccurrenceInput {
    pub occurrence_key: String,
    pub scheduled_start_at: Option<String>,
    pub scheduled_end_at: Option<String>,
    pub due_at: Option<String>,
    pub title_override: Option<String>,
    pub description_override: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OccurrenceUpdateInput {
    pub scheduled_start_at: Option<String>,
    pub scheduled_end_at: Option<String>,
    pub due_at: Option<String>,
    pub title_override: Option<String>,
    pub description_override: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OccurrenceStatusInput {
    pub status: String,
}

fn storage(message: impl Into<String>) -> ExecutionError {
    ExecutionError {
        kind: ExecutionErrorKind::Storage,
        message: message.into(),
    }
}

fn validation(message: impl Into<String>) -> ExecutionError {
    ExecutionError {
        kind: ExecutionErrorKind::Validation,
        message: message.into(),
    }
}

fn not_found(message: impl Into<String>) -> ExecutionError {
    ExecutionError {
        kind: ExecutionErrorKind::NotFound,
        message: message.into(),
    }
}

fn conflict(message: impl Into<String>) -> ExecutionError {
    ExecutionError {
        kind: ExecutionErrorKind::Conflict,
        message: message.into(),
    }
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

fn clean_optional(value: Option<String>, label: &str, max: usize) -> ExecutionResult<Option<String>> {
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

fn parse_timestamp(value: &Option<String>, label: &str) -> ExecutionResult<Option<DateTime<Utc>>> {
    value
        .as_ref()
        .map(|value| {
            DateTime::parse_from_rfc3339(value)
                .map(|value| value.with_timezone(&Utc))
                .map_err(|_| validation(format!("{label}必须是 RFC3339 时间")))
        })
        .transpose()
}

fn validate_time_window(
    start: &Option<String>,
    end: &Option<String>,
    due: &Option<String>,
) -> ExecutionResult<()> {
    let start = parse_timestamp(start, "计划开始时间")?;
    let end = parse_timestamp(end, "计划结束时间")?;
    let _due = parse_timestamp(due, "截止时间")?;
    if start.is_some() != end.is_some() {
        return Err(validation("计划开始时间和结束时间必须同时设置"));
    }
    if let (Some(start), Some(end)) = (start, end) {
        if end < start {
            return Err(validation("计划结束时间不能早于开始时间"));
        }
    }
    Ok(())
}

pub fn list_subtasks(connection: &Connection, parent_id: &str) -> ExecutionResult<Vec<TaskRecord>> {
    let user_id = active_user(connection)?;
    execution::get_task(connection, parent_id)?;
    task_repository::list_tasks(
        connection,
        &user_id,
        &TaskListFilter {
            parent_task_id: Some(parent_id.to_owned()),
            ..TaskListFilter::default()
        },
    )
    .map_err(storage)
}

pub fn add_subtask(
    connection: &Connection,
    parent_id: &str,
    mut input: TaskInput,
) -> ExecutionResult<TaskRecord> {
    let user_id = active_user(connection)?;
    let parent = execution::get_task(connection, parent_id)?;
    if let Some(project_id) = input.project_id.as_ref() {
        if parent.project_id.as_ref() != Some(project_id) {
            return Err(validation("子任务必须与父任务属于同一项目"));
        }
    } else {
        input.project_id = parent.project_id.clone();
    }

    let transaction = connection.unchecked_transaction().map_err(|error| storage(error.to_string()))?;
    let child = execution::create_task(&transaction, input)?;
    if !structure_repository::set_task_parent(
        &transaction,
        &user_id,
        &child.id,
        Some(parent_id),
    )
    .map_err(storage)?
    {
        return Err(storage("设置子任务父级失败"));
    }
    transaction.commit().map_err(|error| storage(error.to_string()))?;
    execution::get_task(connection, &child.id)
}

fn creates_dependency_cycle(edges: &[(String, String)], task_id: &str, prerequisite_id: &str) -> bool {
    let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();
    for (task, prerequisite) in edges {
        graph.entry(task.as_str()).or_default().push(prerequisite.as_str());
    }
    graph.entry(task_id).or_default().push(prerequisite_id);

    let mut stack = vec![prerequisite_id];
    let mut visited = HashSet::new();
    while let Some(current) = stack.pop() {
        if current == task_id {
            return true;
        }
        if !visited.insert(current) {
            continue;
        }
        if let Some(next) = graph.get(current) {
            stack.extend(next.iter().copied());
        }
    }
    false
}

pub fn list_dependencies(
    connection: &Connection,
    task_id: &str,
) -> ExecutionResult<Vec<DependencyRecord>> {
    let user_id = active_user(connection)?;
    execution::get_task(connection, task_id)?;
    structure_repository::list_dependencies(connection, &user_id, task_id).map_err(storage)
}

pub fn add_dependency(
    connection: &Connection,
    task_id: &str,
    input: DependencyInput,
) -> ExecutionResult<DependencyRecord> {
    let user_id = active_user(connection)?;
    let prerequisite_id = clean_required(&input.depends_on_task_id, "前置任务 ID", 128)?;
    execution::get_task(connection, task_id)?;
    execution::get_task(connection, &prerequisite_id)?;
    if task_id == prerequisite_id {
        return Err(validation("任务不能依赖自身"));
    }
    let existing = structure_repository::list_dependencies(connection, &user_id, task_id)
        .map_err(storage)?;
    if let Some(record) = existing
        .into_iter()
        .find(|record| record.depends_on_task_id == prerequisite_id)
    {
        return Ok(record);
    }
    let edges = structure_repository::list_dependency_edges(connection, &user_id).map_err(storage)?;
    if creates_dependency_cycle(&edges, task_id, &prerequisite_id) {
        return Err(conflict("新增前置依赖会形成有向环"));
    }
    structure_repository::create_dependency(connection, &user_id, task_id, &prerequisite_id)
        .map_err(storage)
}

pub fn remove_dependency(
    connection: &Connection,
    task_id: &str,
    prerequisite_id: &str,
) -> ExecutionResult<()> {
    let user_id = active_user(connection)?;
    execution::get_task(connection, task_id)?;
    if structure_repository::remove_dependency(connection, &user_id, task_id, prerequisite_id)
        .map_err(storage)?
    {
        Ok(())
    } else {
        Err(not_found("前置依赖不存在"))
    }
}

pub fn list_blockers(connection: &Connection, task_id: &str) -> ExecutionResult<Vec<TaskBlocker>> {
    let user_id = active_user(connection)?;
    execution::get_task(connection, task_id)?;
    let dependencies = structure_repository::list_dependencies(connection, &user_id, task_id)
        .map_err(storage)?;
    let mut blockers = Vec::new();
    for dependency in dependencies {
        let prerequisite = task_repository::get_task(
            connection,
            &user_id,
            &dependency.depends_on_task_id,
        )
        .map_err(storage)?;
        if let Some(prerequisite) = prerequisite {
            if prerequisite.status != "done" {
                blockers.push(TaskBlocker {
                    task_id: prerequisite.id,
                    title: prerequisite.title,
                    status: prerequisite.status,
                });
            }
        }
    }
    Ok(blockers)
}

fn normalize_recurrence(
    user_id: String,
    id: Option<String>,
    mut input: RecurrenceRuleInput,
) -> ExecutionResult<RecurrenceRuleWrite> {
    if !matches!(input.frequency.as_str(), "daily" | "weekly" | "monthly") {
        return Err(validation("重复频率必须是 daily、weekly 或 monthly"));
    }
    let interval_value = input.interval_value.unwrap_or(1);
    if interval_value < 1 {
        return Err(validation("重复间隔必须大于等于 1"));
    }
    input.weekdays.sort_unstable();
    input.weekdays.dedup();
    if input.weekdays.iter().any(|day| !(1..=7).contains(day)) {
        return Err(validation("星期值必须位于 1..=7（周一到周日）"));
    }
    match input.frequency.as_str() {
        "daily" => {
            if !input.weekdays.is_empty() || input.month_day.is_some() {
                return Err(validation("daily 规则不能设置 weekdays 或 monthDay"));
            }
        }
        "weekly" => {
            if input.weekdays.is_empty() || input.month_day.is_some() {
                return Err(validation("weekly 规则必须设置 weekdays，且不能设置 monthDay"));
            }
        }
        "monthly" => {
            if !input.weekdays.is_empty() {
                return Err(validation("monthly 规则不能设置 weekdays"));
            }
            match input.month_day {
                Some(day) if (1..=31).contains(&day) => {}
                _ => return Err(validation("monthly 规则必须设置 1..=31 的 monthDay")),
            }
        }
        _ => unreachable!(),
    }
    if let Some(max_occurrences) = input.max_occurrences {
        if max_occurrences < 1 {
            return Err(validation("maxOccurrences 必须大于等于 1"));
        }
    }
    if let Some(until_at) = input.until_at.as_ref() {
        let valid_date = NaiveDate::parse_from_str(until_at, "%Y-%m-%d").is_ok();
        let valid_timestamp = DateTime::parse_from_rfc3339(until_at).is_ok();
        if !valid_date && !valid_timestamp {
            return Err(validation("untilAt 必须是 YYYY-MM-DD 或 RFC3339 时间"));
        }
    }
    Ok(RecurrenceRuleWrite {
        id,
        user_id,
        frequency: input.frequency,
        interval_value,
        weekdays: input.weekdays,
        month_day: input.month_day,
        timezone: clean_optional(input.timezone, "重复规则时区", 128)?,
        until_at: input.until_at,
        max_occurrences: input.max_occurrences,
    })
}

pub fn get_task_recurrence(
    connection: &Connection,
    task_id: &str,
) -> ExecutionResult<Option<RecurrenceRuleRecord>> {
    let user_id = active_user(connection)?;
    execution::get_task(connection, task_id)?;
    let rule_id = structure_repository::task_recurrence_rule_id(connection, &user_id, task_id)
        .map_err(storage)?;
    let Some(rule_id) = rule_id else {
        return Ok(None);
    };
    structure_repository::get_recurrence_rule(connection, &user_id, &rule_id).map_err(storage)
}

pub fn set_task_recurrence(
    connection: &Connection,
    task_id: &str,
    input: RecurrenceRuleInput,
) -> ExecutionResult<RecurrenceRuleRecord> {
    let user_id = active_user(connection)?;
    execution::get_task(connection, task_id)?;
    let current_rule_id = structure_repository::task_recurrence_rule_id(connection, &user_id, task_id)
        .map_err(storage)?;
    let write = normalize_recurrence(user_id.clone(), current_rule_id, input)?;
    let transaction = connection.unchecked_transaction().map_err(|error| storage(error.to_string()))?;
    let rule = structure_repository::save_recurrence_rule(&transaction, &write).map_err(storage)?;
    if !structure_repository::set_task_recurrence_rule(
        &transaction,
        &user_id,
        task_id,
        Some(&rule.id),
    )
    .map_err(storage)?
    {
        return Err(not_found("任务不存在"));
    }
    transaction.commit().map_err(|error| storage(error.to_string()))?;
    Ok(rule)
}

pub fn clear_task_recurrence(connection: &Connection, task_id: &str) -> ExecutionResult<()> {
    let user_id = active_user(connection)?;
    execution::get_task(connection, task_id)?;
    let rule_id = structure_repository::task_recurrence_rule_id(connection, &user_id, task_id)
        .map_err(storage)?;
    let Some(rule_id) = rule_id else {
        return Ok(());
    };
    let transaction = connection.unchecked_transaction().map_err(|error| storage(error.to_string()))?;
    structure_repository::set_task_recurrence_rule(&transaction, &user_id, task_id, None)
        .map_err(storage)?;
    structure_repository::soft_delete_recurrence_rule(&transaction, &user_id, &rule_id)
        .map_err(storage)?;
    transaction.commit().map_err(|error| storage(error.to_string()))?;
    Ok(())
}

pub fn list_occurrences(
    connection: &Connection,
    task_id: &str,
) -> ExecutionResult<Vec<TaskOccurrenceRecord>> {
    let user_id = active_user(connection)?;
    execution::get_task(connection, task_id)?;
    structure_repository::list_occurrences(connection, &user_id, task_id).map_err(storage)
}

pub fn materialize_occurrence(
    connection: &Connection,
    task_id: &str,
    input: OccurrenceInput,
) -> ExecutionResult<TaskOccurrenceRecord> {
    let user_id = active_user(connection)?;
    execution::get_task(connection, task_id)?;
    let occurrence_key = clean_required(&input.occurrence_key, "occurrenceKey", 160)?;
    validate_time_window(
        &input.scheduled_start_at,
        &input.scheduled_end_at,
        &input.due_at,
    )?;
    if let Some(existing) = structure_repository::get_occurrence_by_key(
        connection,
        &user_id,
        task_id,
        &occurrence_key,
    )
    .map_err(storage)?
    {
        return Ok(existing);
    }
    let recurrence = get_task_recurrence(connection, task_id)?
        .ok_or_else(|| conflict("只有重复任务才能物化 occurrence"))?;
    if let Some(max_occurrences) = recurrence.max_occurrences {
        let current = structure_repository::count_occurrences(connection, &user_id, task_id)
            .map_err(storage)?;
        if current >= max_occurrences {
            return Err(conflict("重复任务已达到最大实例数量"));
        }
    }
    let write = TaskOccurrenceWrite {
        task_id: task_id.to_owned(),
        occurrence_key,
        scheduled_start_at: input.scheduled_start_at,
        scheduled_end_at: input.scheduled_end_at,
        due_at: input.due_at,
        status: "pending".to_owned(),
        title_override: clean_optional(input.title_override, "本次标题", 240)?,
        description_override: clean_optional(input.description_override, "本次描述", 20_000)?,
        completed_at: None,
        skipped_at: None,
    };
    structure_repository::create_occurrence(connection, &user_id, &write).map_err(storage)
}

pub fn update_occurrence(
    connection: &Connection,
    task_id: &str,
    occurrence_id: &str,
    input: OccurrenceUpdateInput,
) -> ExecutionResult<TaskOccurrenceRecord> {
    let user_id = active_user(connection)?;
    execution::get_task(connection, task_id)?;
    validate_time_window(
        &input.scheduled_start_at,
        &input.scheduled_end_at,
        &input.due_at,
    )?;
    let current = structure_repository::get_occurrence(
        connection,
        &user_id,
        task_id,
        occurrence_id,
    )
    .map_err(storage)?
    .ok_or_else(|| not_found("重复任务实例不存在"))?;
    let write = TaskOccurrenceWrite {
        task_id: task_id.to_owned(),
        occurrence_key: current.occurrence_key,
        scheduled_start_at: input.scheduled_start_at,
        scheduled_end_at: input.scheduled_end_at,
        due_at: input.due_at,
        status: current.status,
        title_override: clean_optional(input.title_override, "本次标题", 240)?,
        description_override: clean_optional(input.description_override, "本次描述", 20_000)?,
        completed_at: current.completed_at,
        skipped_at: current.skipped_at,
    };
    structure_repository::update_occurrence(connection, &user_id, occurrence_id, &write)
        .map_err(storage)
}

fn occurrence_transition_allowed(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        ("pending", "in_progress")
            | ("pending", "waiting")
            | ("pending", "done")
            | ("pending", "skipped")
            | ("pending", "cancelled")
            | ("in_progress", "waiting")
            | ("in_progress", "done")
            | ("in_progress", "cancelled")
            | ("waiting", "in_progress")
            | ("waiting", "done")
            | ("waiting", "cancelled")
            | ("done", "pending")
            | ("skipped", "pending")
    )
}

pub fn change_occurrence_status(
    connection: &Connection,
    task_id: &str,
    occurrence_id: &str,
    input: OccurrenceStatusInput,
) -> ExecutionResult<TaskOccurrenceRecord> {
    if !matches!(
        input.status.as_str(),
        "pending" | "in_progress" | "waiting" | "done" | "skipped" | "cancelled"
    ) {
        return Err(validation("无效的 occurrence 状态"));
    }
    let user_id = active_user(connection)?;
    execution::get_task(connection, task_id)?;
    let current = structure_repository::get_occurrence(
        connection,
        &user_id,
        task_id,
        occurrence_id,
    )
    .map_err(storage)?
    .ok_or_else(|| not_found("重复任务实例不存在"))?;
    if current.status == input.status {
        return Ok(current);
    }
    if !occurrence_transition_allowed(&current.status, &input.status) {
        return Err(conflict(format!(
            "不允许 occurrence 从 {} 切换到 {}",
            current.status, input.status
        )));
    }
    let stamp = Utc::now().to_rfc3339();
    let write = TaskOccurrenceWrite {
        task_id: task_id.to_owned(),
        occurrence_key: current.occurrence_key,
        scheduled_start_at: current.scheduled_start_at,
        scheduled_end_at: current.scheduled_end_at,
        due_at: current.due_at,
        status: input.status.clone(),
        title_override: current.title_override,
        description_override: current.description_override,
        completed_at: if input.status == "done" {
            Some(stamp.clone())
        } else if current.status == "done" {
            None
        } else {
            current.completed_at
        },
        skipped_at: if input.status == "skipped" {
            Some(stamp)
        } else if current.status == "skipped" {
            None
        } else {
            current.skipped_at
        },
    };
    structure_repository::update_occurrence(connection, &user_id, occurrence_id, &write)
        .map_err(storage)
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
        let data_dir = std::env::temp_dir().join(format!("lifetrace-structure-service-{unique}"));
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

    #[test]
    fn subtasks_are_attached_atomically() {
        let connection = database();
        let parent = execution::create_task(&connection, task_input("parent")).unwrap();
        let child = add_subtask(&connection, &parent.id, task_input("child")).unwrap();
        assert_eq!(child.parent_task_id.as_deref(), Some(parent.id.as_str()));
        assert_eq!(list_subtasks(&connection, &parent.id).unwrap().len(), 1);
    }

    #[test]
    fn dependencies_reject_direct_and_transitive_cycles() {
        let connection = database();
        let a = execution::create_task(&connection, task_input("a")).unwrap();
        let b = execution::create_task(&connection, task_input("b")).unwrap();
        let c = execution::create_task(&connection, task_input("c")).unwrap();
        add_dependency(
            &connection,
            &a.id,
            DependencyInput {
                depends_on_task_id: b.id.clone(),
            },
        )
        .unwrap();
        add_dependency(
            &connection,
            &b.id,
            DependencyInput {
                depends_on_task_id: c.id.clone(),
            },
        )
        .unwrap();
        let error = add_dependency(
            &connection,
            &c.id,
            DependencyInput {
                depends_on_task_id: a.id.clone(),
            },
        )
        .unwrap_err();
        assert_eq!(error.kind, ExecutionErrorKind::Conflict);
    }

    #[test]
    fn blockers_disappear_after_prerequisite_is_done() {
        let connection = database();
        let task = execution::create_task(&connection, task_input("blocked")).unwrap();
        let prerequisite = execution::create_task(&connection, task_input("prerequisite")).unwrap();
        add_dependency(
            &connection,
            &task.id,
            DependencyInput {
                depends_on_task_id: prerequisite.id.clone(),
            },
        )
        .unwrap();
        assert_eq!(list_blockers(&connection, &task.id).unwrap().len(), 1);
        execution::change_task_status(
            &connection,
            &prerequisite.id,
            execution::TaskStatusInput {
                status: "done".to_owned(),
            },
        )
        .unwrap();
        assert!(list_blockers(&connection, &task.id).unwrap().is_empty());
    }

    #[test]
    fn recurrence_update_preserves_historical_occurrences() {
        let connection = database();
        let task = execution::create_task(&connection, task_input("repeat")).unwrap();
        set_task_recurrence(
            &connection,
            &task.id,
            RecurrenceRuleInput {
                frequency: "daily".to_owned(),
                interval_value: Some(1),
                weekdays: vec![],
                month_day: None,
                timezone: Some("Asia/Shanghai".to_owned()),
                until_at: None,
                max_occurrences: Some(10),
            },
        )
        .unwrap();
        let occurrence = materialize_occurrence(
            &connection,
            &task.id,
            OccurrenceInput {
                occurrence_key: "2026-08-08".to_owned(),
                scheduled_start_at: None,
                scheduled_end_at: None,
                due_at: Some("2026-08-08T09:00:00+08:00".to_owned()),
                title_override: None,
                description_override: None,
            },
        )
        .unwrap();
        let done = change_occurrence_status(
            &connection,
            &task.id,
            &occurrence.id,
            OccurrenceStatusInput {
                status: "done".to_owned(),
            },
        )
        .unwrap();
        assert!(done.completed_at.is_some());
        set_task_recurrence(
            &connection,
            &task.id,
            RecurrenceRuleInput {
                frequency: "daily".to_owned(),
                interval_value: Some(2),
                weekdays: vec![],
                month_day: None,
                timezone: Some("Asia/Shanghai".to_owned()),
                until_at: None,
                max_occurrences: Some(10),
            },
        )
        .unwrap();
        let occurrences = list_occurrences(&connection, &task.id).unwrap();
        assert_eq!(occurrences.len(), 1);
        assert_eq!(occurrences[0].status, "done");
        assert!(occurrences[0].completed_at.is_some());
    }
}
