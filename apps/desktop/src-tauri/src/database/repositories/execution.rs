use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRecord {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub sort_order: i64,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct ProjectWrite {
    pub id: Option<String>,
    pub user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskRecord {
    pub id: String,
    pub user_id: String,
    pub project_id: Option<String>,
    pub parent_task_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub estimated_minutes: Option<i64>,
    pub actual_minutes: Option<i64>,
    pub due_at: Option<String>,
    pub scheduled_start_at: Option<String>,
    pub scheduled_end_at: Option<String>,
    pub timezone: Option<String>,
    pub context: Option<String>,
    pub completed_at: Option<String>,
    pub cancelled_at: Option<String>,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct TaskWrite {
    pub id: Option<String>,
    pub user_id: String,
    pub project_id: Option<String>,
    pub parent_task_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub estimated_minutes: Option<i64>,
    pub actual_minutes: Option<i64>,
    pub due_at: Option<String>,
    pub scheduled_start_at: Option<String>,
    pub scheduled_end_at: Option<String>,
    pub timezone: Option<String>,
    pub context: Option<String>,
    pub completed_at: Option<String>,
    pub cancelled_at: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TaskListFilter {
    pub status: Option<String>,
    pub project_id: Option<String>,
    pub parent_task_id: Option<String>,
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn project_from_row(row: &Row<'_>) -> rusqlite::Result<ProjectRecord> {
    Ok(ProjectRecord {
        id: row.get(0)?,
        user_id: row.get(1)?,
        name: row.get(2)?,
        description: row.get(3)?,
        status: row.get(4)?,
        color: row.get(5)?,
        icon: row.get(6)?,
        sort_order: row.get(7)?,
        version: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn task_from_row(row: &Row<'_>) -> rusqlite::Result<TaskRecord> {
    Ok(TaskRecord {
        id: row.get(0)?,
        user_id: row.get(1)?,
        project_id: row.get(2)?,
        parent_task_id: row.get(3)?,
        title: row.get(4)?,
        description: row.get(5)?,
        status: row.get(6)?,
        priority: row.get(7)?,
        estimated_minutes: row.get(8)?,
        actual_minutes: row.get(9)?,
        due_at: row.get(10)?,
        scheduled_start_at: row.get(11)?,
        scheduled_end_at: row.get(12)?,
        timezone: row.get(13)?,
        context: row.get(14)?,
        completed_at: row.get(15)?,
        cancelled_at: row.get(16)?,
        version: row.get(17)?,
        created_at: row.get(18)?,
        updated_at: row.get(19)?,
    })
}

const PROJECT_COLUMNS: &str = "id,user_id,name,description,status,color,icon,sort_order,version,created_at,updated_at";
const TASK_COLUMNS: &str = "id,user_id,project_id,parent_task_id,title,description,status,priority,estimated_minutes,actual_minutes,due_at,scheduled_start_at,scheduled_end_at,timezone,context,completed_at,cancelled_at,version,created_at,updated_at";

pub fn list_projects(connection: &Connection, user_id: &str) -> Result<Vec<ProjectRecord>, String> {
    let sql = format!(
        "SELECT {PROJECT_COLUMNS} FROM execution_projects WHERE user_id=?1 AND deleted_at IS NULL ORDER BY sort_order ASC, updated_at DESC"
    );
    let mut statement = connection.prepare(&sql).map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([user_id], project_from_row)
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

pub fn get_project(
    connection: &Connection,
    user_id: &str,
    id: &str,
) -> Result<Option<ProjectRecord>, String> {
    let sql = format!(
        "SELECT {PROJECT_COLUMNS} FROM execution_projects WHERE id=?1 AND user_id=?2 AND deleted_at IS NULL"
    );
    connection
        .query_row(&sql, params![id, user_id], project_from_row)
        .optional()
        .map_err(|error| error.to_string())
}

pub fn save_project(connection: &Connection, input: &ProjectWrite) -> Result<ProjectRecord, String> {
    let id = input
        .id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let stamp = now();
    let existing = get_project(connection, &input.user_id, &id)?;
    if existing.is_some() {
        let changed = connection
            .execute(
                "UPDATE execution_projects SET name=?1,description=?2,status=?3,color=?4,icon=?5,sort_order=?6,updated_at=?7,version=version+1 WHERE id=?8 AND user_id=?9 AND deleted_at IS NULL",
                params![
                    input.name,
                    input.description,
                    input.status,
                    input.color,
                    input.icon,
                    input.sort_order,
                    stamp,
                    id,
                    input.user_id
                ],
            )
            .map_err(|error| error.to_string())?;
        if changed != 1 {
            return Err("项目更新失败".to_owned());
        }
    } else {
        connection
            .execute(
                "INSERT INTO execution_projects(id,user_id,name,description,status,color,icon,sort_order,created_at,updated_at,version) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?9,1)",
                params![
                    id,
                    input.user_id,
                    input.name,
                    input.description,
                    input.status,
                    input.color,
                    input.icon,
                    input.sort_order,
                    stamp
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    get_project(connection, &input.user_id, &id)?
        .ok_or_else(|| "项目保存后读取失败".to_owned())
}

pub fn soft_delete_project(
    connection: &Connection,
    user_id: &str,
    id: &str,
) -> Result<bool, String> {
    let stamp = now();
    connection
        .execute(
            "UPDATE execution_projects SET deleted_at=?1,updated_at=?1,version=version+1 WHERE id=?2 AND user_id=?3 AND deleted_at IS NULL",
            params![stamp, id, user_id],
        )
        .map(|changed| changed == 1)
        .map_err(|error| error.to_string())
}

pub fn list_tasks(
    connection: &Connection,
    user_id: &str,
    filter: &TaskListFilter,
) -> Result<Vec<TaskRecord>, String> {
    let sql = format!(
        "SELECT {TASK_COLUMNS} FROM execution_tasks
         WHERE user_id=?1 AND deleted_at IS NULL
           AND (?2 IS NULL OR status=?2)
           AND (?3 IS NULL OR project_id=?3)
           AND (?4 IS NULL OR parent_task_id=?4)
         ORDER BY
           CASE priority WHEN 'urgent' THEN 0 WHEN 'high' THEN 1 WHEN 'normal' THEN 2 ELSE 3 END,
           CASE WHEN due_at IS NULL THEN 1 ELSE 0 END,
           due_at ASC,
           updated_at DESC"
    );
    let mut statement = connection.prepare(&sql).map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(
            params![
                user_id,
                filter.status.as_deref(),
                filter.project_id.as_deref(),
                filter.parent_task_id.as_deref()
            ],
            task_from_row,
        )
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

pub fn get_task(
    connection: &Connection,
    user_id: &str,
    id: &str,
) -> Result<Option<TaskRecord>, String> {
    let sql = format!(
        "SELECT {TASK_COLUMNS} FROM execution_tasks WHERE id=?1 AND user_id=?2 AND deleted_at IS NULL"
    );
    connection
        .query_row(&sql, params![id, user_id], task_from_row)
        .optional()
        .map_err(|error| error.to_string())
}

pub fn save_task(connection: &Connection, input: &TaskWrite) -> Result<TaskRecord, String> {
    let id = input
        .id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let stamp = now();
    let existing = get_task(connection, &input.user_id, &id)?;
    if existing.is_some() {
        let changed = connection
            .execute(
                "UPDATE execution_tasks SET project_id=?1,parent_task_id=?2,title=?3,description=?4,status=?5,priority=?6,estimated_minutes=?7,actual_minutes=?8,due_at=?9,scheduled_start_at=?10,scheduled_end_at=?11,timezone=?12,context=?13,completed_at=?14,cancelled_at=?15,updated_at=?16,version=version+1 WHERE id=?17 AND user_id=?18 AND deleted_at IS NULL",
                params![
                    input.project_id,
                    input.parent_task_id,
                    input.title,
                    input.description,
                    input.status,
                    input.priority,
                    input.estimated_minutes,
                    input.actual_minutes,
                    input.due_at,
                    input.scheduled_start_at,
                    input.scheduled_end_at,
                    input.timezone,
                    input.context,
                    input.completed_at,
                    input.cancelled_at,
                    stamp,
                    id,
                    input.user_id
                ],
            )
            .map_err(|error| error.to_string())?;
        if changed != 1 {
            return Err("任务更新失败".to_owned());
        }
    } else {
        connection
            .execute(
                "INSERT INTO execution_tasks(id,user_id,project_id,parent_task_id,title,description,status,priority,estimated_minutes,actual_minutes,due_at,scheduled_start_at,scheduled_end_at,timezone,context,completed_at,cancelled_at,created_at,updated_at,version) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?18,1)",
                params![
                    id,
                    input.user_id,
                    input.project_id,
                    input.parent_task_id,
                    input.title,
                    input.description,
                    input.status,
                    input.priority,
                    input.estimated_minutes,
                    input.actual_minutes,
                    input.due_at,
                    input.scheduled_start_at,
                    input.scheduled_end_at,
                    input.timezone,
                    input.context,
                    input.completed_at,
                    input.cancelled_at,
                    stamp
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    get_task(connection, &input.user_id, &id)?
        .ok_or_else(|| "任务保存后读取失败".to_owned())
}

pub fn soft_delete_task(
    connection: &Connection,
    user_id: &str,
    id: &str,
) -> Result<bool, String> {
    let stamp = now();
    connection
        .execute(
            "UPDATE execution_tasks SET deleted_at=?1,updated_at=?1,version=version+1 WHERE id=?2 AND user_id=?3 AND deleted_at IS NULL",
            params![stamp, id, user_id],
        )
        .map(|changed| changed == 1)
        .map_err(|error| error.to_string())
}

pub fn task_has_children(
    connection: &Connection,
    user_id: &str,
    id: &str,
) -> Result<bool, String> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM execution_tasks WHERE user_id=?1 AND parent_task_id=?2 AND deleted_at IS NULL)",
            params![user_id, id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migration_runner::{run, MigrationContext};
    use crate::database::migrations::all;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_data_dir(label: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("lifetrace-execution-{label}-{unique}"));
        std::fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn database() -> (Connection, String) {
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run(
            &mut connection,
            &MigrationContext::new(unique_data_dir("repository")),
            &all(),
        )
        .unwrap();
        let user_id = crate::database::profile::active_profile_id(&connection).unwrap();
        (connection, user_id)
    }

    #[test]
    fn project_and_task_reads_are_profile_scoped() {
        let (connection, user_id) = database();
        let project = save_project(
            &connection,
            &ProjectWrite {
                id: None,
                user_id: user_id.clone(),
                name: "EPIC20".to_owned(),
                description: None,
                status: "active".to_owned(),
                color: None,
                icon: None,
                sort_order: 0,
            },
        )
        .unwrap();
        let task = save_task(
            &connection,
            &TaskWrite {
                id: None,
                user_id: user_id.clone(),
                project_id: Some(project.id.clone()),
                parent_task_id: None,
                title: "Build repository".to_owned(),
                description: None,
                status: "todo".to_owned(),
                priority: "high".to_owned(),
                estimated_minutes: Some(60),
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
        assert_eq!(list_projects(&connection, &user_id).unwrap().len(), 1);
        assert_eq!(list_tasks(&connection, &user_id, &TaskListFilter::default()).unwrap().len(), 1);
        assert_eq!(get_task(&connection, &user_id, &task.id).unwrap().unwrap().project_id, Some(project.id));
        assert!(get_task(&connection, "another-profile", &task.id).unwrap().is_none());
    }

    #[test]
    fn soft_delete_hides_records_without_physical_removal() {
        let (connection, user_id) = database();
        let project = save_project(
            &connection,
            &ProjectWrite {
                id: None,
                user_id: user_id.clone(),
                name: "Disposable".to_owned(),
                description: None,
                status: "active".to_owned(),
                color: None,
                icon: None,
                sort_order: 0,
            },
        )
        .unwrap();
        assert!(soft_delete_project(&connection, &user_id, &project.id).unwrap());
        assert!(get_project(&connection, &user_id, &project.id).unwrap().is_none());
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM execution_projects WHERE id=?1 AND deleted_at IS NOT NULL",
                [project.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
