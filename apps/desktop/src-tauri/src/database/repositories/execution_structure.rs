use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DependencyRecord {
    pub id: String,
    pub task_id: String,
    pub depends_on_task_id: String,
    pub dependency_type: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceRuleRecord {
    pub id: String,
    pub user_id: String,
    pub frequency: String,
    pub interval_value: i64,
    pub weekdays: Vec<u8>,
    pub month_day: Option<i64>,
    pub timezone: Option<String>,
    pub until_at: Option<String>,
    pub max_occurrences: Option<i64>,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct RecurrenceRuleWrite {
    pub id: Option<String>,
    pub user_id: String,
    pub frequency: String,
    pub interval_value: i64,
    pub weekdays: Vec<u8>,
    pub month_day: Option<i64>,
    pub timezone: Option<String>,
    pub until_at: Option<String>,
    pub max_occurrences: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskOccurrenceRecord {
    pub id: String,
    pub task_id: String,
    pub occurrence_key: String,
    pub scheduled_start_at: Option<String>,
    pub scheduled_end_at: Option<String>,
    pub due_at: Option<String>,
    pub status: String,
    pub title_override: Option<String>,
    pub description_override: Option<String>,
    pub completed_at: Option<String>,
    pub skipped_at: Option<String>,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct TaskOccurrenceWrite {
    pub task_id: String,
    pub occurrence_key: String,
    pub scheduled_start_at: Option<String>,
    pub scheduled_end_at: Option<String>,
    pub due_at: Option<String>,
    pub status: String,
    pub title_override: Option<String>,
    pub description_override: Option<String>,
    pub completed_at: Option<String>,
    pub skipped_at: Option<String>,
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn dependency_from_row(row: &Row<'_>) -> rusqlite::Result<DependencyRecord> {
    Ok(DependencyRecord {
        id: row.get(0)?,
        task_id: row.get(1)?,
        depends_on_task_id: row.get(2)?,
        dependency_type: row.get(3)?,
        created_at: row.get(4)?,
    })
}

fn recurrence_from_row(row: &Row<'_>) -> rusqlite::Result<RecurrenceRuleRecord> {
    let weekdays_json: Option<String> = row.get(4)?;
    Ok(RecurrenceRuleRecord {
        id: row.get(0)?,
        user_id: row.get(1)?,
        frequency: row.get(2)?,
        interval_value: row.get(3)?,
        weekdays: weekdays_json
            .as_deref()
            .and_then(|value| serde_json::from_str(value).ok())
            .unwrap_or_default(),
        month_day: row.get(5)?,
        timezone: row.get(6)?,
        until_at: row.get(7)?,
        max_occurrences: row.get(8)?,
        version: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn occurrence_from_row(row: &Row<'_>) -> rusqlite::Result<TaskOccurrenceRecord> {
    Ok(TaskOccurrenceRecord {
        id: row.get(0)?,
        task_id: row.get(1)?,
        occurrence_key: row.get(2)?,
        scheduled_start_at: row.get(3)?,
        scheduled_end_at: row.get(4)?,
        due_at: row.get(5)?,
        status: row.get(6)?,
        title_override: row.get(7)?,
        description_override: row.get(8)?,
        completed_at: row.get(9)?,
        skipped_at: row.get(10)?,
        version: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

const RECURRENCE_COLUMNS: &str = "id,user_id,frequency,interval_value,weekdays_json,month_day,timezone,until_at,max_occurrences,version,created_at,updated_at";
const OCCURRENCE_COLUMNS: &str = "id,task_id,occurrence_key,scheduled_start_at,scheduled_end_at,due_at,status,title_override,description_override,completed_at,skipped_at,version,created_at,updated_at";

pub fn list_dependencies(
    connection: &Connection,
    user_id: &str,
    task_id: &str,
) -> Result<Vec<DependencyRecord>, String> {
    let mut statement = connection
        .prepare(
            "SELECT id,task_id,depends_on_task_id,dependency_type,created_at
             FROM execution_task_dependencies
             WHERE user_id=?1 AND task_id=?2
             ORDER BY created_at ASC",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(params![user_id, task_id], dependency_from_row)
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

pub fn list_dependency_edges(
    connection: &Connection,
    user_id: &str,
) -> Result<Vec<(String, String)>, String> {
    let mut statement = connection
        .prepare(
            "SELECT d.task_id,d.depends_on_task_id
             FROM execution_task_dependencies d
             JOIN execution_tasks task ON task.id=d.task_id
             JOIN execution_tasks prerequisite ON prerequisite.id=d.depends_on_task_id
             WHERE d.user_id=?1
               AND task.deleted_at IS NULL
               AND prerequisite.deleted_at IS NULL",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([user_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

pub fn create_dependency(
    connection: &Connection,
    user_id: &str,
    task_id: &str,
    depends_on_task_id: &str,
) -> Result<DependencyRecord, String> {
    let id = Uuid::new_v4().to_string();
    let stamp = now();
    connection
        .execute(
            "INSERT INTO execution_task_dependencies(id,user_id,task_id,depends_on_task_id,dependency_type,created_at)
             VALUES(?1,?2,?3,?4,'finish_before_start',?5)",
            params![id, user_id, task_id, depends_on_task_id, stamp],
        )
        .map_err(|error| error.to_string())?;
    connection
        .query_row(
            "SELECT id,task_id,depends_on_task_id,dependency_type,created_at
             FROM execution_task_dependencies WHERE id=?1 AND user_id=?2",
            params![id, user_id],
            dependency_from_row,
        )
        .map_err(|error| error.to_string())
}

pub fn remove_dependency(
    connection: &Connection,
    user_id: &str,
    task_id: &str,
    depends_on_task_id: &str,
) -> Result<bool, String> {
    connection
        .execute(
            "DELETE FROM execution_task_dependencies
             WHERE user_id=?1 AND task_id=?2 AND depends_on_task_id=?3",
            params![user_id, task_id, depends_on_task_id],
        )
        .map(|changed| changed > 0)
        .map_err(|error| error.to_string())
}

pub fn set_task_parent(
    connection: &Connection,
    user_id: &str,
    task_id: &str,
    parent_task_id: Option<&str>,
) -> Result<bool, String> {
    let stamp = now();
    connection
        .execute(
            "UPDATE execution_tasks
             SET parent_task_id=?1,updated_at=?2,version=version+1
             WHERE id=?3 AND user_id=?4 AND deleted_at IS NULL",
            params![parent_task_id, stamp, task_id, user_id],
        )
        .map(|changed| changed == 1)
        .map_err(|error| error.to_string())
}

pub fn task_recurrence_rule_id(
    connection: &Connection,
    user_id: &str,
    task_id: &str,
) -> Result<Option<String>, String> {
    connection
        .query_row(
            "SELECT recurrence_rule_id FROM execution_tasks
             WHERE id=?1 AND user_id=?2 AND deleted_at IS NULL",
            params![task_id, user_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?
        .flatten()
        .pipe(Ok)
}

pub fn set_task_recurrence_rule(
    connection: &Connection,
    user_id: &str,
    task_id: &str,
    recurrence_rule_id: Option<&str>,
) -> Result<bool, String> {
    let stamp = now();
    connection
        .execute(
            "UPDATE execution_tasks
             SET recurrence_rule_id=?1,updated_at=?2,version=version+1
             WHERE id=?3 AND user_id=?4 AND deleted_at IS NULL",
            params![recurrence_rule_id, stamp, task_id, user_id],
        )
        .map(|changed| changed == 1)
        .map_err(|error| error.to_string())
}

pub fn get_recurrence_rule(
    connection: &Connection,
    user_id: &str,
    id: &str,
) -> Result<Option<RecurrenceRuleRecord>, String> {
    let sql = format!(
        "SELECT {RECURRENCE_COLUMNS} FROM execution_recurrence_rules
         WHERE id=?1 AND user_id=?2 AND deleted_at IS NULL"
    );
    connection
        .query_row(&sql, params![id, user_id], recurrence_from_row)
        .optional()
        .map_err(|error| error.to_string())
}

pub fn save_recurrence_rule(
    connection: &Connection,
    input: &RecurrenceRuleWrite,
) -> Result<RecurrenceRuleRecord, String> {
    let id = input
        .id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let stamp = now();
    let weekdays_json = if input.weekdays.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&input.weekdays).map_err(|error| error.to_string())?)
    };
    if get_recurrence_rule(connection, &input.user_id, &id)?.is_some() {
        connection
            .execute(
                "UPDATE execution_recurrence_rules
                 SET frequency=?1,interval_value=?2,weekdays_json=?3,month_day=?4,timezone=?5,
                     until_at=?6,max_occurrences=?7,updated_at=?8,version=version+1
                 WHERE id=?9 AND user_id=?10 AND deleted_at IS NULL",
                params![
                    input.frequency,
                    input.interval_value,
                    weekdays_json,
                    input.month_day,
                    input.timezone,
                    input.until_at,
                    input.max_occurrences,
                    stamp,
                    id,
                    input.user_id
                ],
            )
            .map_err(|error| error.to_string())?;
    } else {
        connection
            .execute(
                "INSERT INTO execution_recurrence_rules(
                   id,user_id,frequency,interval_value,weekdays_json,month_day,timezone,
                   until_at,max_occurrences,created_at,updated_at,version
                 ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?10,1)",
                params![
                    id,
                    input.user_id,
                    input.frequency,
                    input.interval_value,
                    weekdays_json,
                    input.month_day,
                    input.timezone,
                    input.until_at,
                    input.max_occurrences,
                    stamp
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    get_recurrence_rule(connection, &input.user_id, &id)?
        .ok_or_else(|| "重复规则保存后读取失败".to_owned())
}

pub fn soft_delete_recurrence_rule(
    connection: &Connection,
    user_id: &str,
    id: &str,
) -> Result<bool, String> {
    let stamp = now();
    connection
        .execute(
            "UPDATE execution_recurrence_rules
             SET deleted_at=?1,updated_at=?1,version=version+1
             WHERE id=?2 AND user_id=?3 AND deleted_at IS NULL",
            params![stamp, id, user_id],
        )
        .map(|changed| changed == 1)
        .map_err(|error| error.to_string())
}

pub fn get_occurrence_by_key(
    connection: &Connection,
    user_id: &str,
    task_id: &str,
    occurrence_key: &str,
) -> Result<Option<TaskOccurrenceRecord>, String> {
    let sql = format!(
        "SELECT {OCCURRENCE_COLUMNS} FROM execution_task_occurrences
         WHERE user_id=?1 AND task_id=?2 AND occurrence_key=?3 AND deleted_at IS NULL"
    );
    connection
        .query_row(
            &sql,
            params![user_id, task_id, occurrence_key],
            occurrence_from_row,
        )
        .optional()
        .map_err(|error| error.to_string())
}

pub fn get_occurrence(
    connection: &Connection,
    user_id: &str,
    task_id: &str,
    occurrence_id: &str,
) -> Result<Option<TaskOccurrenceRecord>, String> {
    let sql = format!(
        "SELECT {OCCURRENCE_COLUMNS} FROM execution_task_occurrences
         WHERE user_id=?1 AND task_id=?2 AND id=?3 AND deleted_at IS NULL"
    );
    connection
        .query_row(
            &sql,
            params![user_id, task_id, occurrence_id],
            occurrence_from_row,
        )
        .optional()
        .map_err(|error| error.to_string())
}

pub fn list_occurrences(
    connection: &Connection,
    user_id: &str,
    task_id: &str,
) -> Result<Vec<TaskOccurrenceRecord>, String> {
    let sql = format!(
        "SELECT {OCCURRENCE_COLUMNS} FROM execution_task_occurrences
         WHERE user_id=?1 AND task_id=?2 AND deleted_at IS NULL
         ORDER BY occurrence_key ASC"
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(params![user_id, task_id], occurrence_from_row)
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

pub fn count_occurrences(
    connection: &Connection,
    user_id: &str,
    task_id: &str,
) -> Result<i64, String> {
    connection
        .query_row(
            "SELECT COUNT(*) FROM execution_task_occurrences
             WHERE user_id=?1 AND task_id=?2 AND deleted_at IS NULL",
            params![user_id, task_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())
}

pub fn create_occurrence(
    connection: &Connection,
    user_id: &str,
    input: &TaskOccurrenceWrite,
) -> Result<TaskOccurrenceRecord, String> {
    if let Some(existing) =
        get_occurrence_by_key(connection, user_id, &input.task_id, &input.occurrence_key)?
    {
        return Ok(existing);
    }
    let id = Uuid::new_v4().to_string();
    let stamp = now();
    connection
        .execute(
            "INSERT INTO execution_task_occurrences(
               id,user_id,task_id,occurrence_key,scheduled_start_at,scheduled_end_at,due_at,status,
               title_override,description_override,completed_at,skipped_at,created_at,updated_at,version
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?13,1)",
            params![
                id,
                user_id,
                input.task_id,
                input.occurrence_key,
                input.scheduled_start_at,
                input.scheduled_end_at,
                input.due_at,
                input.status,
                input.title_override,
                input.description_override,
                input.completed_at,
                input.skipped_at,
                stamp
            ],
        )
        .map_err(|error| error.to_string())?;
    get_occurrence(connection, user_id, &input.task_id, &id)?
        .ok_or_else(|| "重复任务实例创建后读取失败".to_owned())
}

pub fn update_occurrence(
    connection: &Connection,
    user_id: &str,
    occurrence_id: &str,
    input: &TaskOccurrenceWrite,
) -> Result<TaskOccurrenceRecord, String> {
    let stamp = now();
    let changed = connection
        .execute(
            "UPDATE execution_task_occurrences
             SET scheduled_start_at=?1,scheduled_end_at=?2,due_at=?3,status=?4,title_override=?5,
                 description_override=?6,completed_at=?7,skipped_at=?8,updated_at=?9,version=version+1
             WHERE id=?10 AND user_id=?11 AND task_id=?12 AND deleted_at IS NULL",
            params![
                input.scheduled_start_at,
                input.scheduled_end_at,
                input.due_at,
                input.status,
                input.title_override,
                input.description_override,
                input.completed_at,
                input.skipped_at,
                stamp,
                occurrence_id,
                user_id,
                input.task_id
            ],
        )
        .map_err(|error| error.to_string())?;
    if changed != 1 {
        return Err("重复任务实例不存在".to_owned());
    }
    get_occurrence(connection, user_id, &input.task_id, occurrence_id)?
        .ok_or_else(|| "重复任务实例更新后读取失败".to_owned())
}

trait Pipe: Sized {
    fn pipe<T>(self, function: impl FnOnce(Self) -> T) -> T {
        function(self)
    }
}

impl<T> Pipe for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migration_runner::{run, MigrationContext};
    use crate::database::migrations::all;
    use crate::database::repositories::execution::{save_task, TaskWrite};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn database() -> (Connection, String) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("lifetrace-execution-structure-{unique}"));
        std::fs::create_dir_all(&data_dir).unwrap();
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run(&mut connection, &MigrationContext::new(data_dir), &all()).unwrap();
        let user_id = crate::database::profile::active_profile_id(&connection).unwrap();
        (connection, user_id)
    }

    fn task(connection: &Connection, user_id: &str, id: &str) {
        save_task(
            connection,
            &TaskWrite {
                id: Some(id.to_owned()),
                user_id: user_id.to_owned(),
                project_id: None,
                parent_task_id: None,
                title: id.to_owned(),
                description: None,
                status: "todo".to_owned(),
                priority: "normal".to_owned(),
                estimated_minutes: None,
                actual_minutes: None,
                due_at: None,
                scheduled_start_at: None,
                scheduled_end_at: None,
                timezone: None,
                context: None,
                completed_at: None,
                cancelled_at: None,
            },
        )
        .unwrap();
    }

    #[test]
    fn dependency_edges_and_parent_links_round_trip() {
        let (connection, user_id) = database();
        task(&connection, &user_id, "parent");
        task(&connection, &user_id, "child");
        task(&connection, &user_id, "prerequisite");
        assert!(set_task_parent(&connection, &user_id, "child", Some("parent")).unwrap());
        create_dependency(&connection, &user_id, "child", "prerequisite").unwrap();
        assert_eq!(
            list_dependencies(&connection, &user_id, "child")
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            list_dependency_edges(&connection, &user_id).unwrap().len(),
            1
        );
    }

    #[test]
    fn recurrence_and_occurrence_are_idempotent() {
        let (connection, user_id) = database();
        task(&connection, &user_id, "series");
        let rule = save_recurrence_rule(
            &connection,
            &RecurrenceRuleWrite {
                id: None,
                user_id: user_id.clone(),
                frequency: "weekly".to_owned(),
                interval_value: 1,
                weekdays: vec![1, 3, 5],
                month_day: None,
                timezone: Some("Asia/Shanghai".to_owned()),
                until_at: None,
                max_occurrences: None,
            },
        )
        .unwrap();
        assert!(set_task_recurrence_rule(&connection, &user_id, "series", Some(&rule.id)).unwrap());
        let write = TaskOccurrenceWrite {
            task_id: "series".to_owned(),
            occurrence_key: "2026-08-10".to_owned(),
            scheduled_start_at: None,
            scheduled_end_at: None,
            due_at: Some("2026-08-10T09:00:00+08:00".to_owned()),
            status: "pending".to_owned(),
            title_override: None,
            description_override: None,
            completed_at: None,
            skipped_at: None,
        };
        let first = create_occurrence(&connection, &user_id, &write).unwrap();
        let second = create_occurrence(&connection, &user_id, &write).unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(
            count_occurrences(&connection, &user_id, "series").unwrap(),
            1
        );
    }
}
