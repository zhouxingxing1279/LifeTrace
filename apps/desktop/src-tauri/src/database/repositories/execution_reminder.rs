use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReminderRecord {
    pub id: String,
    pub user_id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub trigger_at: String,
    pub timezone: Option<String>,
    pub status: String,
    pub snoozed_until: Option<String>,
    pub last_fired_at: Option<String>,
    pub fire_key: String,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct ReminderWrite {
    pub id: Option<String>,
    pub user_id: String,
    pub subject_type: String,
    pub subject_id: String,
    pub trigger_at: String,
    pub timezone: Option<String>,
    pub status: String,
    pub snoozed_until: Option<String>,
    pub last_fired_at: Option<String>,
    pub fire_key: String,
}

const COLUMNS: &str = "id,user_id,subject_type,subject_id,trigger_at,timezone,status,snoozed_until,last_fired_at,fire_key,version,created_at,updated_at";

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn from_row(row: &Row<'_>) -> rusqlite::Result<ReminderRecord> {
    Ok(ReminderRecord {
        id: row.get(0)?,
        user_id: row.get(1)?,
        subject_type: row.get(2)?,
        subject_id: row.get(3)?,
        trigger_at: row.get(4)?,
        timezone: row.get(5)?,
        status: row.get(6)?,
        snoozed_until: row.get(7)?,
        last_fired_at: row.get(8)?,
        fire_key: row.get(9)?,
        version: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

pub fn get(
    connection: &Connection,
    user_id: &str,
    id: &str,
) -> Result<Option<ReminderRecord>, String> {
    let sql = format!("SELECT {COLUMNS} FROM execution_reminders WHERE id=?1 AND user_id=?2 AND deleted_at IS NULL");
    connection
        .query_row(&sql, params![id, user_id], from_row)
        .optional()
        .map_err(|error| error.to_string())
}

pub fn find_by_fire_key(
    connection: &Connection,
    user_id: &str,
    fire_key: &str,
) -> Result<Option<ReminderRecord>, String> {
    let sql = format!("SELECT {COLUMNS} FROM execution_reminders WHERE user_id=?1 AND fire_key=?2 AND deleted_at IS NULL");
    connection
        .query_row(&sql, params![user_id, fire_key], from_row)
        .optional()
        .map_err(|error| error.to_string())
}

pub fn list_for_subject(
    connection: &Connection,
    user_id: &str,
    subject_type: &str,
    subject_id: &str,
) -> Result<Vec<ReminderRecord>, String> {
    let sql = format!(
        "SELECT {COLUMNS} FROM execution_reminders
         WHERE user_id=?1 AND subject_type=?2 AND subject_id=?3 AND deleted_at IS NULL
         ORDER BY trigger_at ASC, created_at ASC"
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(params![user_id, subject_type, subject_id], from_row)
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

pub fn list_due(
    connection: &Connection,
    user_id: &str,
    now_at: &str,
    limit: i64,
) -> Result<Vec<ReminderRecord>, String> {
    let sql = format!(
        "SELECT {COLUMNS} FROM execution_reminders
         WHERE user_id=?1 AND status='scheduled' AND deleted_at IS NULL
           AND COALESCE(snoozed_until,trigger_at) <= ?2
         ORDER BY COALESCE(snoozed_until,trigger_at) ASC, created_at ASC LIMIT ?3"
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(params![user_id, now_at, limit], from_row)
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

pub fn save(connection: &Connection, input: &ReminderWrite) -> Result<ReminderRecord, String> {
    let id = input
        .id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let stamp = now();
    if get(connection, &input.user_id, &id)?.is_some() {
        let changed = connection
            .execute(
                "UPDATE execution_reminders SET subject_type=?1,subject_id=?2,trigger_at=?3,timezone=?4,status=?5,
                 snoozed_until=?6,last_fired_at=?7,fire_key=?8,updated_at=?9,version=version+1
                 WHERE id=?10 AND user_id=?11 AND deleted_at IS NULL",
                params![input.subject_type,input.subject_id,input.trigger_at,input.timezone,input.status,input.snoozed_until,
                    input.last_fired_at,input.fire_key,stamp,id,input.user_id],
            )
            .map_err(|error| error.to_string())?;
        if changed != 1 {
            return Err("提醒更新失败".to_owned());
        }
    } else {
        connection
            .execute(
                "INSERT INTO execution_reminders(id,user_id,subject_type,subject_id,trigger_at,timezone,status,snoozed_until,
                 last_fired_at,fire_key,created_at,updated_at,version)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?11,1)",
                params![id,input.user_id,input.subject_type,input.subject_id,input.trigger_at,input.timezone,input.status,
                    input.snoozed_until,input.last_fired_at,input.fire_key,stamp],
            )
            .map_err(|error| error.to_string())?;
    }
    get(connection, &input.user_id, &id)?.ok_or_else(|| "提醒保存后读取失败".to_owned())
}

pub fn soft_delete(connection: &Connection, user_id: &str, id: &str) -> Result<bool, String> {
    let stamp = now();
    connection
        .execute(
            "UPDATE execution_reminders SET deleted_at=?1,updated_at=?1,version=version+1
             WHERE id=?2 AND user_id=?3 AND deleted_at IS NULL",
            params![stamp, id, user_id],
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

    fn db() -> (Connection, String) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("lifetrace-reminder-repo-{unique}"));
        std::fs::create_dir_all(&data_dir).unwrap();
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run(&mut connection, &MigrationContext::new(data_dir), &all()).unwrap();
        let user_id = crate::database::profile::active_profile_id(&connection).unwrap();
        (connection, user_id)
    }

    #[test]
    fn due_query_uses_snooze_and_fire_key_is_idempotent() {
        let (connection, user_id) = db();
        let input = ReminderWrite {
            id: None,
            user_id: user_id.clone(),
            subject_type: "memo".to_owned(),
            subject_id: "m1".to_owned(),
            trigger_at: "2026-08-08T09:00:00Z".to_owned(),
            timezone: Some("UTC".to_owned()),
            status: "scheduled".to_owned(),
            snoozed_until: Some("2026-08-08T11:00:00Z".to_owned()),
            last_fired_at: None,
            fire_key: "memo:m1:20260808T090000Z".to_owned(),
        };
        let saved = save(&connection, &input).unwrap();
        assert!(list_due(&connection, &user_id, "2026-08-08T10:00:00Z", 10)
            .unwrap()
            .is_empty());
        assert_eq!(
            list_due(&connection, &user_id, "2026-08-08T12:00:00Z", 10)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            find_by_fire_key(&connection, &user_id, &saved.fire_key)
                .unwrap()
                .unwrap()
                .id,
            saved.id
        );
    }
}
