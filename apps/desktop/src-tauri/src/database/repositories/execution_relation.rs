use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompletionResultRecord {
    pub id: String,
    pub user_id: String,
    pub task_id: String,
    pub summary: Option<String>,
    pub completed_at: String,
    pub actual_minutes: Option<i64>,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct CompletionResultWrite {
    pub user_id: String,
    pub task_id: String,
    pub summary: Option<String>,
    pub completed_at: String,
    pub actual_minutes: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EntityLinkRecord {
    pub id: String,
    pub user_id: String,
    pub source_type: String,
    pub source_id: String,
    pub relation_type: String,
    pub target_type: String,
    pub target_id: String,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct EntityLinkWrite {
    pub user_id: String,
    pub source_type: String,
    pub source_id: String,
    pub relation_type: String,
    pub target_type: String,
    pub target_id: String,
}

const COMPLETION_COLUMNS: &str =
    "id,user_id,task_id,summary,completed_at,actual_minutes,version,created_at,updated_at";
const LINK_COLUMNS: &str =
    "id,user_id,source_type,source_id,relation_type,target_type,target_id,version,created_at,updated_at";

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn completion_from_row(row: &Row<'_>) -> rusqlite::Result<CompletionResultRecord> {
    let summary: String = row.get(3)?;
    Ok(CompletionResultRecord {
        id: row.get(0)?,
        user_id: row.get(1)?,
        task_id: row.get(2)?,
        summary: if summary.trim().is_empty() { None } else { Some(summary) },
        completed_at: row.get(4)?,
        actual_minutes: row.get(5)?,
        version: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn link_from_row(row: &Row<'_>) -> rusqlite::Result<EntityLinkRecord> {
    Ok(EntityLinkRecord {
        id: row.get(0)?,
        user_id: row.get(1)?,
        source_type: row.get(2)?,
        source_id: row.get(3)?,
        relation_type: row.get(4)?,
        target_type: row.get(5)?,
        target_id: row.get(6)?,
        version: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

pub fn get_completion_result(
    connection: &Connection,
    user_id: &str,
    task_id: &str,
) -> Result<Option<CompletionResultRecord>, String> {
    let sql = format!(
        "SELECT {COMPLETION_COLUMNS} FROM execution_completion_results
         WHERE user_id=?1 AND task_id=?2 AND deleted_at IS NULL
         ORDER BY updated_at DESC LIMIT 1"
    );
    connection
        .query_row(&sql, params![user_id, task_id], completion_from_row)
        .optional()
        .map_err(|error| error.to_string())
}

pub fn save_completion_result(
    connection: &Connection,
    input: &CompletionResultWrite,
) -> Result<CompletionResultRecord, String> {
    let stamp = now();
    let summary = input.summary.as_deref().unwrap_or("");
    if let Some(existing) = get_completion_result(connection, &input.user_id, &input.task_id)? {
        let changed = connection
            .execute(
                "UPDATE execution_completion_results
                 SET summary=?1,completed_at=?2,actual_minutes=?3,updated_at=?4,version=version+1
                 WHERE id=?5 AND user_id=?6 AND deleted_at IS NULL",
                params![
                    summary,
                    input.completed_at,
                    input.actual_minutes,
                    stamp,
                    existing.id,
                    input.user_id
                ],
            )
            .map_err(|error| error.to_string())?;
        if changed != 1 {
            return Err("完成结果更新失败".to_owned());
        }
    } else {
        connection
            .execute(
                "INSERT INTO execution_completion_results(
                   id,user_id,task_id,summary,completed_at,actual_minutes,created_at,updated_at,version
                 ) VALUES(?1,?2,?3,?4,?5,?6,?7,?7,1)",
                params![
                    Uuid::new_v4().to_string(),
                    input.user_id,
                    input.task_id,
                    summary,
                    input.completed_at,
                    input.actual_minutes,
                    stamp
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    get_completion_result(connection, &input.user_id, &input.task_id)?
        .ok_or_else(|| "完成结果保存后读取失败".to_owned())
}

pub fn soft_delete_completion_result(
    connection: &Connection,
    user_id: &str,
    task_id: &str,
) -> Result<bool, String> {
    let stamp = now();
    connection
        .execute(
            "UPDATE execution_completion_results
             SET deleted_at=?1,updated_at=?1,version=version+1
             WHERE user_id=?2 AND task_id=?3 AND deleted_at IS NULL",
            params![stamp, user_id, task_id],
        )
        .map(|changed| changed > 0)
        .map_err(|error| error.to_string())
}

pub fn list_links_for_entity(
    connection: &Connection,
    user_id: &str,
    entity_type: &str,
    entity_id: &str,
) -> Result<Vec<EntityLinkRecord>, String> {
    let sql = format!(
        "SELECT {LINK_COLUMNS} FROM execution_entity_links
         WHERE user_id=?1 AND deleted_at IS NULL
           AND ((source_type=?2 AND source_id=?3) OR (target_type=?2 AND target_id=?3))
         ORDER BY created_at ASC,id ASC"
    );
    let mut statement = connection.prepare(&sql).map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(params![user_id, entity_type, entity_id], link_from_row)
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

pub fn find_link(
    connection: &Connection,
    input: &EntityLinkWrite,
) -> Result<Option<EntityLinkRecord>, String> {
    let sql = format!(
        "SELECT {LINK_COLUMNS} FROM execution_entity_links
         WHERE user_id=?1 AND source_type=?2 AND source_id=?3 AND relation_type=?4
           AND target_type=?5 AND target_id=?6 AND deleted_at IS NULL
         LIMIT 1"
    );
    connection
        .query_row(
            &sql,
            params![
                input.user_id,
                input.source_type,
                input.source_id,
                input.relation_type,
                input.target_type,
                input.target_id
            ],
            link_from_row,
        )
        .optional()
        .map_err(|error| error.to_string())
}

pub fn save_link(
    connection: &Connection,
    input: &EntityLinkWrite,
) -> Result<EntityLinkRecord, String> {
    if let Some(existing) = find_link(connection, input)? {
        return Ok(existing);
    }
    let id = Uuid::new_v4().to_string();
    let stamp = now();
    connection
        .execute(
            "INSERT INTO execution_entity_links(
               id,user_id,source_type,source_id,relation_type,target_type,target_id,
               created_at,updated_at,version
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?8,1)",
            params![
                id,
                input.user_id,
                input.source_type,
                input.source_id,
                input.relation_type,
                input.target_type,
                input.target_id,
                stamp
            ],
        )
        .map_err(|error| error.to_string())?;
    find_link(connection, input)?.ok_or_else(|| "关联保存后读取失败".to_owned())
}

pub fn soft_delete_link(
    connection: &Connection,
    user_id: &str,
    link_id: &str,
) -> Result<bool, String> {
    let stamp = now();
    connection
        .execute(
            "UPDATE execution_entity_links
             SET deleted_at=?1,updated_at=?1,version=version+1
             WHERE id=?2 AND user_id=?3 AND deleted_at IS NULL",
            params![stamp, link_id, user_id],
        )
        .map(|changed| changed == 1)
        .map_err(|error| error.to_string())
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
        let directory = std::env::temp_dir().join(format!("lifetrace-execution-relation-{unique}"));
        std::fs::create_dir_all(&directory).unwrap();
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run(&mut connection, &MigrationContext::new(directory), &all()).unwrap();
        let user_id = crate::database::profile::active_profile_id(&connection).unwrap();
        (connection, user_id)
    }

    #[test]
    fn completion_result_upserts_and_soft_deletes() {
        let (connection, user_id) = database();
        connection.execute(
            "INSERT INTO execution_tasks(id,user_id,title,status,priority,created_at,updated_at,version)
             VALUES('task-1',?1,'Task','done','normal','2026-08-09T00:00:00Z','2026-08-09T00:00:00Z',1)",
            [&user_id],
        ).unwrap();
        let first = save_completion_result(&connection, &CompletionResultWrite {
            user_id: user_id.clone(),
            task_id: "task-1".to_owned(),
            summary: Some("完成".to_owned()),
            completed_at: "2026-08-09T00:00:00Z".to_owned(),
            actual_minutes: Some(30),
        }).unwrap();
        let second = save_completion_result(&connection, &CompletionResultWrite {
            user_id: user_id.clone(),
            task_id: "task-1".to_owned(),
            summary: Some("已交付".to_owned()),
            completed_at: "2026-08-09T01:00:00Z".to_owned(),
            actual_minutes: Some(45),
        }).unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(second.summary.as_deref(), Some("已交付"));
        assert!(soft_delete_completion_result(&connection, &user_id, "task-1").unwrap());
        assert!(get_completion_result(&connection, &user_id, "task-1").unwrap().is_none());
    }

    #[test]
    fn links_are_bidirectionally_queryable_and_idempotent() {
        let (connection, user_id) = database();
        let write = EntityLinkWrite {
            user_id: user_id.clone(),
            source_type: "task".to_owned(),
            source_id: "task-a".to_owned(),
            relation_type: "related_to".to_owned(),
            target_type: "note".to_owned(),
            target_id: "note-b".to_owned(),
        };
        let first = save_link(&connection, &write).unwrap();
        let second = save_link(&connection, &write).unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(list_links_for_entity(&connection, &user_id, "task", "task-a").unwrap().len(), 1);
        assert_eq!(list_links_for_entity(&connection, &user_id, "note", "note-b").unwrap().len(), 1);
        assert!(soft_delete_link(&connection, &user_id, &first.id).unwrap());
        assert!(list_links_for_entity(&connection, &user_id, "task", "task-a").unwrap().is_empty());
    }
}
