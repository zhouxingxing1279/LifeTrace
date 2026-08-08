use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEventRecord {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub description: Option<String>,
    pub is_all_day: bool,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub start_local_date: Option<String>,
    pub end_local_date: Option<String>,
    pub timezone: Option<String>,
    pub status: String,
    pub recurrence_rule_id: Option<String>,
    pub source_task_id: Option<String>,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct CalendarEventWrite {
    pub id: Option<String>,
    pub user_id: String,
    pub title: String,
    pub description: Option<String>,
    pub is_all_day: bool,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub start_local_date: Option<String>,
    pub end_local_date: Option<String>,
    pub timezone: Option<String>,
    pub status: String,
    pub recurrence_rule_id: Option<String>,
    pub source_task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CalendarOccurrenceRecord {
    pub id: String,
    pub event_id: String,
    pub occurrence_key: String,
    pub is_all_day: bool,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub start_local_date: Option<String>,
    pub end_local_date: Option<String>,
    pub status: String,
    pub title_override: Option<String>,
    pub description_override: Option<String>,
    pub version: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct CalendarOccurrenceWrite {
    pub event_id: String,
    pub occurrence_key: String,
    pub is_all_day: bool,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub start_local_date: Option<String>,
    pub end_local_date: Option<String>,
    pub status: String,
    pub title_override: Option<String>,
    pub description_override: Option<String>,
}

const EVENT_COLUMNS: &str = "id,user_id,title,description,is_all_day,start_at,end_at,start_local_date,end_local_date,timezone,status,recurrence_rule_id,source_task_id,version,created_at,updated_at";
const OCCURRENCE_COLUMNS: &str = "id,event_id,occurrence_key,is_all_day,start_at,end_at,start_local_date,end_local_date,status,title_override,description_override,version,created_at,updated_at";

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn event_from_row(row: &Row<'_>) -> rusqlite::Result<CalendarEventRecord> {
    Ok(CalendarEventRecord {
        id: row.get(0)?,
        user_id: row.get(1)?,
        title: row.get(2)?,
        description: row.get(3)?,
        is_all_day: row.get::<_, i64>(4)? != 0,
        start_at: row.get(5)?,
        end_at: row.get(6)?,
        start_local_date: row.get(7)?,
        end_local_date: row.get(8)?,
        timezone: row.get(9)?,
        status: row.get(10)?,
        recurrence_rule_id: row.get(11)?,
        source_task_id: row.get(12)?,
        version: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

fn occurrence_from_row(row: &Row<'_>) -> rusqlite::Result<CalendarOccurrenceRecord> {
    Ok(CalendarOccurrenceRecord {
        id: row.get(0)?,
        event_id: row.get(1)?,
        occurrence_key: row.get(2)?,
        is_all_day: row.get::<_, i64>(3)? != 0,
        start_at: row.get(4)?,
        end_at: row.get(5)?,
        start_local_date: row.get(6)?,
        end_local_date: row.get(7)?,
        status: row.get(8)?,
        title_override: row.get(9)?,
        description_override: row.get(10)?,
        version: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

pub fn get_event(
    connection: &Connection,
    user_id: &str,
    id: &str,
) -> Result<Option<CalendarEventRecord>, String> {
    let sql = format!(
        "SELECT {EVENT_COLUMNS} FROM execution_calendar_events
         WHERE id=?1 AND user_id=?2 AND deleted_at IS NULL"
    );
    connection
        .query_row(&sql, params![id, user_id], event_from_row)
        .optional()
        .map_err(|error| error.to_string())
}

pub fn find_scheduled_event_by_source_task(
    connection: &Connection,
    user_id: &str,
    task_id: &str,
) -> Result<Option<CalendarEventRecord>, String> {
    let sql = format!(
        "SELECT {EVENT_COLUMNS} FROM execution_calendar_events
         WHERE user_id=?1 AND source_task_id=?2 AND status='scheduled' AND deleted_at IS NULL
         ORDER BY updated_at DESC LIMIT 1"
    );
    connection
        .query_row(&sql, params![user_id, task_id], event_from_row)
        .optional()
        .map_err(|error| error.to_string())
}

pub fn list_events(
    connection: &Connection,
    user_id: &str,
    include_cancelled: bool,
) -> Result<Vec<CalendarEventRecord>, String> {
    let sql = format!(
        "SELECT {EVENT_COLUMNS} FROM execution_calendar_events
         WHERE user_id=?1 AND deleted_at IS NULL AND (?2=1 OR status='scheduled')
         ORDER BY COALESCE(start_at,start_local_date) ASC, updated_at DESC"
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(
            params![user_id, if include_cancelled { 1_i64 } else { 0_i64 }],
            event_from_row,
        )
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

pub fn save_event(
    connection: &Connection,
    input: &CalendarEventWrite,
) -> Result<CalendarEventRecord, String> {
    let id = input
        .id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let stamp = now();
    if get_event(connection, &input.user_id, &id)?.is_some() {
        let changed = connection
            .execute(
                "UPDATE execution_calendar_events SET
                   title=?1,description=?2,is_all_day=?3,start_at=?4,end_at=?5,
                   start_local_date=?6,end_local_date=?7,timezone=?8,status=?9,
                   recurrence_rule_id=?10,source_task_id=?11,updated_at=?12,version=version+1
                 WHERE id=?13 AND user_id=?14 AND deleted_at IS NULL",
                params![
                    input.title,
                    input.description,
                    if input.is_all_day { 1_i64 } else { 0_i64 },
                    input.start_at,
                    input.end_at,
                    input.start_local_date,
                    input.end_local_date,
                    input.timezone,
                    input.status,
                    input.recurrence_rule_id,
                    input.source_task_id,
                    stamp,
                    id,
                    input.user_id
                ],
            )
            .map_err(|error| error.to_string())?;
        if changed != 1 {
            return Err("日历事件更新失败".to_owned());
        }
    } else {
        connection
            .execute(
                "INSERT INTO execution_calendar_events(
                   id,user_id,title,description,is_all_day,start_at,end_at,start_local_date,end_local_date,
                   timezone,status,recurrence_rule_id,source_task_id,created_at,updated_at,version
                 ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?14,1)",
                params![
                    id,
                    input.user_id,
                    input.title,
                    input.description,
                    if input.is_all_day { 1_i64 } else { 0_i64 },
                    input.start_at,
                    input.end_at,
                    input.start_local_date,
                    input.end_local_date,
                    input.timezone,
                    input.status,
                    input.recurrence_rule_id,
                    input.source_task_id,
                    stamp
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    get_event(connection, &input.user_id, &id)?.ok_or_else(|| "日历事件保存后读取失败".to_owned())
}

pub fn soft_delete_event(connection: &Connection, user_id: &str, id: &str) -> Result<bool, String> {
    let stamp = now();
    connection
        .execute(
            "UPDATE execution_calendar_events
             SET deleted_at=?1,updated_at=?1,version=version+1
             WHERE id=?2 AND user_id=?3 AND deleted_at IS NULL",
            params![stamp, id, user_id],
        )
        .map(|changed| changed == 1)
        .map_err(|error| error.to_string())
}

pub fn set_event_recurrence_rule(
    connection: &Connection,
    user_id: &str,
    event_id: &str,
    recurrence_rule_id: Option<&str>,
) -> Result<bool, String> {
    let stamp = now();
    connection
        .execute(
            "UPDATE execution_calendar_events
             SET recurrence_rule_id=?1,updated_at=?2,version=version+1
             WHERE id=?3 AND user_id=?4 AND deleted_at IS NULL",
            params![recurrence_rule_id, stamp, event_id, user_id],
        )
        .map(|changed| changed == 1)
        .map_err(|error| error.to_string())
}

pub fn get_occurrence(
    connection: &Connection,
    user_id: &str,
    event_id: &str,
    occurrence_id: &str,
) -> Result<Option<CalendarOccurrenceRecord>, String> {
    let sql = format!(
        "SELECT {OCCURRENCE_COLUMNS} FROM execution_calendar_occurrences
         WHERE user_id=?1 AND event_id=?2 AND id=?3 AND deleted_at IS NULL"
    );
    connection
        .query_row(
            &sql,
            params![user_id, event_id, occurrence_id],
            occurrence_from_row,
        )
        .optional()
        .map_err(|error| error.to_string())
}

pub fn get_occurrence_by_key(
    connection: &Connection,
    user_id: &str,
    event_id: &str,
    occurrence_key: &str,
) -> Result<Option<CalendarOccurrenceRecord>, String> {
    let sql = format!(
        "SELECT {OCCURRENCE_COLUMNS} FROM execution_calendar_occurrences
         WHERE user_id=?1 AND event_id=?2 AND occurrence_key=?3 AND deleted_at IS NULL"
    );
    connection
        .query_row(
            &sql,
            params![user_id, event_id, occurrence_key],
            occurrence_from_row,
        )
        .optional()
        .map_err(|error| error.to_string())
}

pub fn list_occurrences(
    connection: &Connection,
    user_id: &str,
    event_id: &str,
) -> Result<Vec<CalendarOccurrenceRecord>, String> {
    let sql = format!(
        "SELECT {OCCURRENCE_COLUMNS} FROM execution_calendar_occurrences
         WHERE user_id=?1 AND event_id=?2 AND deleted_at IS NULL ORDER BY occurrence_key ASC"
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(params![user_id, event_id], occurrence_from_row)
        .map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())
}

pub fn create_occurrence(
    connection: &Connection,
    user_id: &str,
    input: &CalendarOccurrenceWrite,
) -> Result<CalendarOccurrenceRecord, String> {
    if let Some(existing) =
        get_occurrence_by_key(connection, user_id, &input.event_id, &input.occurrence_key)?
    {
        return Ok(existing);
    }
    let id = Uuid::new_v4().to_string();
    let stamp = now();
    connection
        .execute(
            "INSERT INTO execution_calendar_occurrences(
               id,user_id,event_id,occurrence_key,is_all_day,start_at,end_at,start_local_date,
               end_local_date,status,title_override,description_override,created_at,updated_at,version
             ) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?13,1)",
            params![
                id,
                user_id,
                input.event_id,
                input.occurrence_key,
                if input.is_all_day { 1_i64 } else { 0_i64 },
                input.start_at,
                input.end_at,
                input.start_local_date,
                input.end_local_date,
                input.status,
                input.title_override,
                input.description_override,
                stamp
            ],
        )
        .map_err(|error| error.to_string())?;
    get_occurrence(connection, user_id, &input.event_id, &id)?
        .ok_or_else(|| "日历 occurrence 创建后读取失败".to_owned())
}

pub fn update_occurrence(
    connection: &Connection,
    user_id: &str,
    occurrence_id: &str,
    input: &CalendarOccurrenceWrite,
) -> Result<CalendarOccurrenceRecord, String> {
    let stamp = now();
    let changed = connection
        .execute(
            "UPDATE execution_calendar_occurrences SET
               is_all_day=?1,start_at=?2,end_at=?3,start_local_date=?4,end_local_date=?5,status=?6,
               title_override=?7,description_override=?8,updated_at=?9,version=version+1
             WHERE id=?10 AND user_id=?11 AND event_id=?12 AND deleted_at IS NULL",
            params![
                if input.is_all_day { 1_i64 } else { 0_i64 },
                input.start_at,
                input.end_at,
                input.start_local_date,
                input.end_local_date,
                input.status,
                input.title_override,
                input.description_override,
                stamp,
                occurrence_id,
                user_id,
                input.event_id
            ],
        )
        .map_err(|error| error.to_string())?;
    if changed != 1 {
        return Err("日历 occurrence 不存在".to_owned());
    }
    get_occurrence(connection, user_id, &input.event_id, occurrence_id)?
        .ok_or_else(|| "日历 occurrence 更新后读取失败".to_owned())
}

pub fn create_task_schedule_link(
    connection: &Connection,
    user_id: &str,
    task_id: &str,
    event_id: &str,
) -> Result<(), String> {
    let stamp = now();
    connection
        .execute(
            "INSERT OR IGNORE INTO execution_entity_links(
               id,user_id,source_type,source_id,relation_type,target_type,target_id,created_at,updated_at,version
             ) VALUES(?1,?2,'task',?3,'related_to','calendar_event',?4,?5,?5,1)",
            params![Uuid::new_v4().to_string(), user_id, task_id, event_id, stamp],
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
        let data_dir = std::env::temp_dir().join(format!("lifetrace-calendar-repository-{unique}"));
        std::fs::create_dir_all(&data_dir).unwrap();
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run(&mut connection, &MigrationContext::new(data_dir), &all()).unwrap();
        let user_id = crate::database::profile::active_profile_id(&connection).unwrap();
        (connection, user_id)
    }

    #[test]
    fn timed_and_all_day_events_round_trip_without_mixing_time_models() {
        let (connection, user_id) = database();
        let timed = save_event(
            &connection,
            &CalendarEventWrite {
                id: None,
                user_id: user_id.clone(),
                title: "meeting".to_owned(),
                description: None,
                is_all_day: false,
                start_at: Some("2026-08-08T09:00:00+08:00".to_owned()),
                end_at: Some("2026-08-08T10:00:00+08:00".to_owned()),
                start_local_date: None,
                end_local_date: None,
                timezone: Some("Asia/Shanghai".to_owned()),
                status: "scheduled".to_owned(),
                recurrence_rule_id: None,
                source_task_id: None,
            },
        )
        .unwrap();
        let all_day = save_event(
            &connection,
            &CalendarEventWrite {
                id: None,
                user_id: user_id.clone(),
                title: "holiday".to_owned(),
                description: None,
                is_all_day: true,
                start_at: None,
                end_at: None,
                start_local_date: Some("2026-08-08".to_owned()),
                end_local_date: Some("2026-08-08".to_owned()),
                timezone: Some("America/Los_Angeles".to_owned()),
                status: "scheduled".to_owned(),
                recurrence_rule_id: None,
                source_task_id: None,
            },
        )
        .unwrap();
        assert!(timed.start_local_date.is_none());
        assert_eq!(all_day.start_local_date.as_deref(), Some("2026-08-08"));
        assert!(all_day.start_at.is_none());
        assert_eq!(list_events(&connection, &user_id, false).unwrap().len(), 2);
    }

    #[test]
    fn occurrences_are_idempotent_per_event_key() {
        let (connection, user_id) = database();
        let event = save_event(
            &connection,
            &CalendarEventWrite {
                id: None,
                user_id: user_id.clone(),
                title: "daily".to_owned(),
                description: None,
                is_all_day: false,
                start_at: Some("2026-08-08T09:00:00+08:00".to_owned()),
                end_at: Some("2026-08-08T09:30:00+08:00".to_owned()),
                start_local_date: None,
                end_local_date: None,
                timezone: Some("Asia/Shanghai".to_owned()),
                status: "scheduled".to_owned(),
                recurrence_rule_id: None,
                source_task_id: None,
            },
        )
        .unwrap();
        let write = CalendarOccurrenceWrite {
            event_id: event.id,
            occurrence_key: "2026-08-09".to_owned(),
            is_all_day: false,
            start_at: Some("2026-08-09T09:00:00+08:00".to_owned()),
            end_at: Some("2026-08-09T09:30:00+08:00".to_owned()),
            start_local_date: None,
            end_local_date: None,
            status: "scheduled".to_owned(),
            title_override: None,
            description_override: None,
        };
        let first = create_occurrence(&connection, &user_id, &write).unwrap();
        let second = create_occurrence(&connection, &user_id, &write).unwrap();
        assert_eq!(first.id, second.id);
    }
}
