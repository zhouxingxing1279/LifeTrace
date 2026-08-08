use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WaitingItemRecord {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub waiting_for: String,
    pub expected_at: Option<String>,
    pub follow_up_at: Option<String>,
    pub resolved_at: Option<String>,
    pub resolution_summary: Option<String>,
    pub source_task_id: Option<String>,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct WaitingItemWrite {
    pub id: Option<String>,
    pub user_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub waiting_for: String,
    pub expected_at: Option<String>,
    pub follow_up_at: Option<String>,
    pub resolved_at: Option<String>,
    pub resolution_summary: Option<String>,
    pub source_task_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct WaitingListFilter {
    pub status: Option<String>,
    pub source_task_id: Option<String>,
    pub expected_before: Option<String>,
    pub follow_up_before: Option<String>,
}

const WAITING_COLUMNS: &str = "id,user_id,title,description,status,waiting_for,expected_at,follow_up_at,resolved_at,resolution_summary,source_task_id,version,created_at,updated_at";

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn from_row(row: &Row<'_>) -> rusqlite::Result<WaitingItemRecord> {
    Ok(WaitingItemRecord {
        id: row.get(0)?,
        user_id: row.get(1)?,
        title: row.get(2)?,
        description: row.get(3)?,
        status: row.get(4)?,
        waiting_for: row.get(5)?,
        expected_at: row.get(6)?,
        follow_up_at: row.get(7)?,
        resolved_at: row.get(8)?,
        resolution_summary: row.get(9)?,
        source_task_id: row.get(10)?,
        version: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

pub fn list_waiting_items(
    connection: &Connection,
    user_id: &str,
    filter: &WaitingListFilter,
) -> Result<Vec<WaitingItemRecord>, String> {
    let sql = format!(
        "SELECT {WAITING_COLUMNS} FROM execution_waiting_items
         WHERE user_id=?1 AND deleted_at IS NULL
           AND (?2 IS NULL OR status=?2)
           AND (?3 IS NULL OR source_task_id=?3)
           AND (?4 IS NULL OR (expected_at IS NOT NULL AND expected_at < ?4))
           AND (?5 IS NULL OR (follow_up_at IS NOT NULL AND follow_up_at <= ?5))
         ORDER BY
           CASE status WHEN 'open' THEN 0 WHEN 'resolved' THEN 1 ELSE 2 END,
           CASE WHEN follow_up_at IS NULL THEN 1 ELSE 0 END,
           follow_up_at ASC,
           CASE WHEN expected_at IS NULL THEN 1 ELSE 0 END,
           expected_at ASC,
           updated_at DESC"
    );
    let mut statement = connection.prepare(&sql).map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(
            params![
                user_id,
                filter.status.as_deref(),
                filter.source_task_id.as_deref(),
                filter.expected_before.as_deref(),
                filter.follow_up_before.as_deref()
            ],
            from_row,
        )
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

pub fn get_waiting_item(
    connection: &Connection,
    user_id: &str,
    id: &str,
) -> Result<Option<WaitingItemRecord>, String> {
    let sql = format!(
        "SELECT {WAITING_COLUMNS} FROM execution_waiting_items
         WHERE id=?1 AND user_id=?2 AND deleted_at IS NULL"
    );
    connection
        .query_row(&sql, params![id, user_id], from_row)
        .optional()
        .map_err(|error| error.to_string())
}

pub fn find_open_by_source_task(
    connection: &Connection,
    user_id: &str,
    task_id: &str,
) -> Result<Option<WaitingItemRecord>, String> {
    let sql = format!(
        "SELECT {WAITING_COLUMNS} FROM execution_waiting_items
         WHERE user_id=?1 AND source_task_id=?2 AND status='open' AND deleted_at IS NULL
         ORDER BY updated_at DESC LIMIT 1"
    );
    connection
        .query_row(&sql, params![user_id, task_id], from_row)
        .optional()
        .map_err(|error| error.to_string())
}

pub fn save_waiting_item(
    connection: &Connection,
    input: &WaitingItemWrite,
) -> Result<WaitingItemRecord, String> {
    let id = input
        .id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let stamp = now();
    if get_waiting_item(connection, &input.user_id, &id)?.is_some() {
        let changed = connection
            .execute(
                "UPDATE execution_waiting_items
                 SET title=?1,description=?2,status=?3,waiting_for=?4,expected_at=?5,follow_up_at=?6,
                     resolved_at=?7,resolution_summary=?8,source_task_id=?9,updated_at=?10,version=version+1
                 WHERE id=?11 AND user_id=?12 AND deleted_at IS NULL",
                params![
                    input.title,
                    input.description,
                    input.status,
                    input.waiting_for,
                    input.expected_at,
                    input.follow_up_at,
                    input.resolved_at,
                    input.resolution_summary,
                    input.source_task_id,
                    stamp,
                    id,
                    input.user_id
                ],
            )
            .map_err(|error| error.to_string())?;
        if changed != 1 {
            return Err("等待事项更新失败".to_owned());
        }
    } else {
        connection
            .execute(
                "INSERT INTO execution_waiting_items(
                   id,user_id,title,description,status,waiting_for,expected_at,follow_up_at,
                   resolved_at,resolution_summary,source_task_id,created_at,updated_at,version
                 ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?12,1)",
                params![
                    id,
                    input.user_id,
                    input.title,
                    input.description,
                    input.status,
                    input.waiting_for,
                    input.expected_at,
                    input.follow_up_at,
                    input.resolved_at,
                    input.resolution_summary,
                    input.source_task_id,
                    stamp
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    get_waiting_item(connection, &input.user_id, &id)?
        .ok_or_else(|| "等待事项保存后读取失败".to_owned())
}

pub fn soft_delete_waiting_item(
    connection: &Connection,
    user_id: &str,
    id: &str,
) -> Result<bool, String> {
    let stamp = now();
    connection
        .execute(
            "UPDATE execution_waiting_items
             SET deleted_at=?1,updated_at=?1,version=version+1
             WHERE id=?2 AND user_id=?3 AND deleted_at IS NULL",
            params![stamp, id, user_id],
        )
        .map(|changed| changed == 1)
        .map_err(|error| error.to_string())
}

pub fn find_conversion_target_task_id(
    connection: &Connection,
    user_id: &str,
    waiting_item_id: &str,
) -> Result<Option<String>, String> {
    connection
        .query_row(
            "SELECT target_id FROM execution_entity_links
             WHERE user_id=?1 AND source_type='waiting_item' AND source_id=?2
               AND relation_type='converted_to' AND target_type='task' AND deleted_at IS NULL
             ORDER BY created_at ASC LIMIT 1",
            params![user_id, waiting_item_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())
}

pub fn create_conversion_links(
    connection: &Connection,
    user_id: &str,
    waiting_item_id: &str,
    task_id: &str,
) -> Result<(), String> {
    let stamp = now();
    connection
        .execute(
            "INSERT OR IGNORE INTO execution_entity_links(
               id,user_id,source_type,source_id,relation_type,target_type,target_id,
               created_at,updated_at,version
             ) VALUES(?1,?2,'waiting_item',?3,'converted_to','task',?4,?5,?5,1)",
            params![Uuid::new_v4().to_string(), user_id, waiting_item_id, task_id, stamp],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "INSERT OR IGNORE INTO execution_entity_links(
               id,user_id,source_type,source_id,relation_type,target_type,target_id,
               created_at,updated_at,version
             ) VALUES(?1,?2,'task',?3,'derived_from','waiting_item',?4,?5,?5,1)",
            params![Uuid::new_v4().to_string(), user_id, task_id, waiting_item_id, stamp],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migration_runner::{run, MigrationContext};
    use crate::database::migrations::all;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn database() -> (Connection, String) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("lifetrace-waiting-repository-{unique}"));
        std::fs::create_dir_all(&data_dir).unwrap();
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run(&mut connection, &MigrationContext::new(data_dir), &all()).unwrap();
        let user_id = crate::database::profile::active_profile_id(&connection).unwrap();
        (connection, user_id)
    }

    fn write(user_id: &str) -> WaitingItemWrite {
        WaitingItemWrite {
            id: None,
            user_id: user_id.to_owned(),
            title: "等待回复".to_owned(),
            description: None,
            status: "open".to_owned(),
            waiting_for: "Alice".to_owned(),
            expected_at: Some("2026-08-10T00:00:00Z".to_owned()),
            follow_up_at: Some("2026-08-09T00:00:00Z".to_owned()),
            resolved_at: None,
            resolution_summary: None,
            source_task_id: None,
        }
    }

    #[test]
    fn waiting_items_are_profile_scoped_and_soft_deleted() {
        let (connection, user_id) = database();
        let item = save_waiting_item(&connection, &write(&user_id)).unwrap();
        assert_eq!(list_waiting_items(&connection, &user_id, &WaitingListFilter::default()).unwrap().len(), 1);
        assert!(get_waiting_item(&connection, "other", &item.id).unwrap().is_none());
        assert!(soft_delete_waiting_item(&connection, &user_id, &item.id).unwrap());
        assert!(get_waiting_item(&connection, &user_id, &item.id).unwrap().is_none());
    }

    #[test]
    fn conversion_links_are_idempotent() {
        let (connection, user_id) = database();
        let item = save_waiting_item(&connection, &write(&user_id)).unwrap();
        connection
            .execute(
                "INSERT INTO execution_tasks(id,user_id,title,status,priority,created_at,updated_at,version)
                 VALUES('task-1',?1,'Follow up','todo','normal','2026-08-08','2026-08-08',1)",
                [user_id.as_str()],
            )
            .unwrap();
        create_conversion_links(&connection, &user_id, &item.id, "task-1").unwrap();
        create_conversion_links(&connection, &user_id, &item.id, "task-1").unwrap();
        assert_eq!(
            find_conversion_target_task_id(&connection, &user_id, &item.id).unwrap(),
            Some("task-1".to_owned())
        );
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM execution_entity_links WHERE user_id=?1 AND ((source_type='waiting_item' AND source_id=?2) OR (target_type='waiting_item' AND target_id=?2))",
                params![user_id, item.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
    }
}
