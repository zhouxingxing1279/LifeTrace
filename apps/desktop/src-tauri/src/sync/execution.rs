use std::collections::HashSet;

use rusqlite::{
    params, params_from_iter,
    types::{Value as SqlValue, ValueRef},
    Connection, OptionalExtension,
};
use serde_json::{Map, Value};

const MEMO_TAG_RELATION: &str = "execution.memo_tag_relation";

#[derive(Debug, Clone, Copy)]
struct TableSpec {
    entity_type: &'static str,
    table: &'static str,
    soft_delete: bool,
}

const TABLE_SPECS: &[TableSpec] = &[
    TableSpec {
        entity_type: "execution.goal",
        table: "execution_goals",
        soft_delete: true,
    },
    TableSpec {
        entity_type: "execution.project",
        table: "execution_projects",
        soft_delete: true,
    },
    TableSpec {
        entity_type: "execution.recurrence_rule",
        table: "execution_recurrence_rules",
        soft_delete: true,
    },
    TableSpec {
        entity_type: "execution.task",
        table: "execution_tasks",
        soft_delete: true,
    },
    TableSpec {
        entity_type: "execution.task_dependency",
        table: "execution_task_dependencies",
        soft_delete: false,
    },
    TableSpec {
        entity_type: "execution.task_occurrence",
        table: "execution_task_occurrences",
        soft_delete: true,
    },
    TableSpec {
        entity_type: "execution.waiting_item",
        table: "execution_waiting_items",
        soft_delete: true,
    },
    TableSpec {
        entity_type: "execution.calendar_event",
        table: "execution_calendar_events",
        soft_delete: true,
    },
    TableSpec {
        entity_type: "execution.calendar_occurrence",
        table: "execution_calendar_occurrences",
        soft_delete: true,
    },
    TableSpec {
        entity_type: "execution.memo",
        table: "execution_memos",
        soft_delete: true,
    },
    TableSpec {
        entity_type: "execution.memo_tag",
        table: "execution_memo_tags",
        soft_delete: true,
    },
    TableSpec {
        entity_type: "execution.reminder",
        table: "execution_reminders",
        soft_delete: true,
    },
    TableSpec {
        entity_type: "execution.completion_result",
        table: "execution_completion_results",
        soft_delete: true,
    },
    TableSpec {
        entity_type: "execution.entity_link",
        table: "execution_entity_links",
        soft_delete: true,
    },
];

pub const ENTITY_TYPES: &[&str] = &[
    "execution.goal",
    "execution.project",
    "execution.recurrence_rule",
    "execution.task",
    "execution.task_dependency",
    "execution.task_occurrence",
    "execution.waiting_item",
    "execution.calendar_event",
    "execution.calendar_occurrence",
    "execution.memo",
    "execution.memo_tag",
    MEMO_TAG_RELATION,
    "execution.reminder",
    "execution.completion_result",
    "execution.entity_link",
];

pub fn is_execution(entity_type: &str) -> bool {
    ENTITY_TYPES.contains(&entity_type)
}

fn spec_for(entity_type: &str) -> Option<TableSpec> {
    TABLE_SPECS
        .iter()
        .copied()
        .find(|spec| spec.entity_type == entity_type)
}

fn snake_to_camel(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut uppercase = false;
    for ch in value.chars() {
        if ch == '_' {
            uppercase = true;
        } else if uppercase {
            result.extend(ch.to_uppercase());
            uppercase = false;
        } else {
            result.push(ch);
        }
    }
    result
}

fn camel_to_snake(value: &str) -> String {
    let mut result = String::with_capacity(value.len() + 4);
    for ch in value.chars() {
        if ch.is_ascii_uppercase() {
            result.push('_');
            result.push(ch.to_ascii_lowercase());
        } else {
            result.push(ch);
        }
    }
    result
}

fn is_boolean_column(column: &str) -> bool {
    matches!(column, "is_all_day" | "is_pinned")
}

fn sqlite_value(value: ValueRef<'_>, column: &str) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(value) if is_boolean_column(column) => Value::Bool(value != 0),
        ValueRef::Integer(value) => Value::Number(value.into()),
        ValueRef::Real(value) => serde_json::Number::from_f64(value)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        ValueRef::Text(value) => Value::String(String::from_utf8_lossy(value).into_owned()),
        ValueRef::Blob(value) => Value::Array(
            value
                .iter()
                .map(|byte| Value::Number((*byte as u64).into()))
                .collect(),
        ),
    }
}

fn load_normal_entity(
    connection: &Connection,
    profile: &str,
    spec: TableSpec,
    entity_id: &str,
) -> Result<Option<Value>, String> {
    let sql = format!("SELECT * FROM {} WHERE id=?1 AND user_id=?2", spec.table);
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| error.to_string())?;
    let column_names = statement
        .column_names()
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<Vec<_>>();
    statement
        .query_row(params![entity_id, profile], |row| {
            let mut object = Map::new();
            for (index, column) in column_names.iter().enumerate() {
                object.insert(
                    snake_to_camel(column),
                    sqlite_value(row.get_ref(index)?, column),
                );
            }
            Ok(Value::Object(object))
        })
        .optional()
        .map_err(|error| error.to_string())
}

fn load_memo_tag_relation(
    connection: &Connection,
    profile: &str,
    entity_id: &str,
) -> Result<Option<Value>, String> {
    let Some((memo_id, tag_id)) = entity_id.split_once(':') else {
        return Ok(None);
    };
    connection
        .query_row(
            "SELECT r.memo_id,r.tag_id,r.created_at,m.user_id
             FROM execution_memo_tag_relations r
             JOIN execution_memos m ON m.id=r.memo_id
             WHERE r.memo_id=?1 AND r.tag_id=?2 AND m.user_id=?3",
            params![memo_id, tag_id, profile],
            |row| {
                let created_at: String = row.get(2)?;
                Ok(serde_json::json!({
                    "id": entity_id,
                    "userId": row.get::<_, String>(3)?,
                    "memoId": row.get::<_, String>(0)?,
                    "tagId": row.get::<_, String>(1)?,
                    "createdAt": created_at,
                    "updatedAt": created_at,
                    "version": 1
                }))
            },
        )
        .optional()
        .map_err(|error| error.to_string())
}

pub fn load_local_entity(
    connection: &Connection,
    profile: &str,
    entity_type: &str,
    entity_id: &str,
) -> Result<Option<Value>, String> {
    if entity_type == MEMO_TAG_RELATION {
        return load_memo_tag_relation(connection, profile, entity_id);
    }
    let Some(spec) = spec_for(entity_type) else {
        return Ok(None);
    };
    load_normal_entity(connection, profile, spec, entity_id)
}

pub fn existing_entities(
    connection: &Connection,
    profile: &str,
) -> Result<Vec<(&'static str, Value)>, String> {
    let mut result = Vec::new();
    for spec in TABLE_SPECS {
        let sql = if spec.soft_delete {
            format!(
                "SELECT id FROM {} WHERE user_id=?1 AND deleted_at IS NULL ORDER BY created_at,id",
                spec.table
            )
        } else {
            format!(
                "SELECT id FROM {} WHERE user_id=?1 ORDER BY created_at,id",
                spec.table
            )
        };
        let mut statement = connection
            .prepare(&sql)
            .map_err(|error| error.to_string())?;
        let ids = statement
            .query_map([profile], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?;
        drop(statement);
        for id in ids {
            if let Some(value) = load_normal_entity(connection, profile, *spec, &id)? {
                result.push((spec.entity_type, value));
            }
        }
    }

    let mut statement = connection
        .prepare(
            "SELECT r.memo_id,r.tag_id
             FROM execution_memo_tag_relations r
             JOIN execution_memos m ON m.id=r.memo_id
             JOIN execution_memo_tags t ON t.id=r.tag_id
             WHERE m.user_id=?1 AND m.deleted_at IS NULL AND t.deleted_at IS NULL
             ORDER BY r.created_at,r.memo_id,r.tag_id",
        )
        .map_err(|error| error.to_string())?;
    let relation_ids = statement
        .query_map([profile], |row| {
            Ok(format!(
                "{}:{}",
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?
            ))
        })
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    drop(statement);
    for id in relation_ids {
        if let Some(value) = load_memo_tag_relation(connection, profile, &id)? {
            result.push((MEMO_TAG_RELATION, value));
        }
    }
    Ok(result)
}

fn table_columns(connection: &Connection, table: &str) -> Result<Vec<String>, String> {
    let sql = format!("PRAGMA table_info({table})");
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| error.to_string())?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    Ok(columns)
}

fn json_to_sql(value: &Value, column: &str) -> SqlValue {
    match value {
        Value::Null => SqlValue::Null,
        Value::Bool(value) => SqlValue::Integer(if *value { 1 } else { 0 }),
        Value::Number(value) => value
            .as_i64()
            .map(SqlValue::Integer)
            .or_else(|| value.as_f64().map(SqlValue::Real))
            .unwrap_or(SqlValue::Null),
        Value::String(value) => SqlValue::Text(value.clone()),
        Value::Array(_) | Value::Object(_) if column.ends_with("_json") => {
            SqlValue::Text(value.to_string())
        }
        Value::Array(_) | Value::Object(_) => SqlValue::Text(value.to_string()),
    }
}

fn apply_normal_upsert(
    connection: &Connection,
    profile: &str,
    spec: TableSpec,
    legacy: &Value,
) -> Result<(), String> {
    let object = legacy
        .as_object()
        .ok_or_else(|| "execution sync payload must be an object".to_owned())?;
    let table_columns = table_columns(connection, spec.table)?;
    let allowed = table_columns.iter().cloned().collect::<HashSet<_>>();
    let mut values_by_column = Map::new();
    for (key, value) in object {
        let column = camel_to_snake(key);
        if allowed.contains(&column) {
            values_by_column.insert(column, value.clone());
        }
    }
    if allowed.contains("user_id") {
        values_by_column.insert("user_id".to_owned(), Value::String(profile.to_owned()));
    }
    let id = values_by_column
        .get("id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{} payload is missing id", spec.entity_type))?;
    let _ = id;

    let columns = table_columns
        .iter()
        .filter(|column| values_by_column.contains_key(*column))
        .cloned()
        .collect::<Vec<_>>();
    if columns.is_empty() {
        return Err(format!(
            "{} payload has no writable columns",
            spec.entity_type
        ));
    }
    let sql_values = columns
        .iter()
        .map(|column| json_to_sql(values_by_column.get(column).unwrap_or(&Value::Null), column))
        .collect::<Vec<_>>();
    let placeholders = (1..=columns.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(",");
    let updates = columns
        .iter()
        .filter(|column| column.as_str() != "id")
        .map(|column| format!("{column}=excluded.{column}"))
        .collect::<Vec<_>>()
        .join(",");
    let sql = if updates.is_empty() {
        format!(
            "INSERT OR IGNORE INTO {}({}) VALUES({})",
            spec.table,
            columns.join(","),
            placeholders
        )
    } else {
        format!(
            "INSERT INTO {}({}) VALUES({}) ON CONFLICT(id) DO UPDATE SET {}",
            spec.table,
            columns.join(","),
            placeholders,
            updates
        )
    };
    connection
        .execute(&sql, params_from_iter(sql_values.iter()))
        .map(|_| ())
        .map_err(|error| format!("apply {} upsert: {error}", spec.entity_type))
}

fn apply_relation_upsert(
    connection: &Connection,
    profile: &str,
    legacy: &Value,
) -> Result<(), String> {
    let object = legacy
        .as_object()
        .ok_or_else(|| "memo tag relation payload must be an object".to_owned())?;
    let memo_id = object
        .get("memoId")
        .and_then(Value::as_str)
        .ok_or_else(|| "memo tag relation is missing memoId".to_owned())?;
    let tag_id = object
        .get("tagId")
        .and_then(Value::as_str)
        .ok_or_else(|| "memo tag relation is missing tagId".to_owned())?;
    let created_at = object
        .get("createdAt")
        .and_then(Value::as_str)
        .unwrap_or("1970-01-01T00:00:00Z");
    let owner: Option<String> = connection
        .query_row(
            "SELECT user_id FROM execution_memos WHERE id=?1",
            [memo_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    if owner.as_deref() != Some(profile) {
        return Err("memo tag relation owner mismatch or memo missing".to_owned());
    }
    connection
        .execute(
            "INSERT INTO execution_memo_tag_relations(memo_id,tag_id,created_at)
             VALUES(?1,?2,?3)
             ON CONFLICT(memo_id,tag_id) DO UPDATE SET created_at=excluded.created_at",
            params![memo_id, tag_id, created_at],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

pub fn apply_upsert(
    connection: &Connection,
    profile: &str,
    entity_type: &str,
    legacy: &Value,
) -> Result<(), String> {
    if entity_type == MEMO_TAG_RELATION {
        return apply_relation_upsert(connection, profile, legacy);
    }
    let spec = spec_for(entity_type)
        .ok_or_else(|| format!("unsupported execution entity type: {entity_type}"))?;
    apply_normal_upsert(connection, profile, spec, legacy)
}

pub fn apply_delete(
    connection: &Connection,
    profile: &str,
    entity_type: &str,
    entity_id: &str,
) -> Result<(), String> {
    if entity_type == MEMO_TAG_RELATION {
        let Some((memo_id, tag_id)) = entity_id.split_once(':') else {
            return Ok(());
        };
        return connection
            .execute(
                "DELETE FROM execution_memo_tag_relations
                 WHERE memo_id=?1 AND tag_id=?2
                   AND EXISTS(SELECT 1 FROM execution_memos WHERE id=?1 AND user_id=?3)",
                params![memo_id, tag_id, profile],
            )
            .map(|_| ())
            .map_err(|error| error.to_string());
    }
    let spec = spec_for(entity_type)
        .ok_or_else(|| format!("unsupported execution entity type: {entity_type}"))?;
    if spec.soft_delete {
        connection
            .execute(
                &format!(
                    "UPDATE {} SET deleted_at=?1,updated_at=?1,version=version+1
                     WHERE id=?2 AND user_id=?3",
                    spec.table
                ),
                params![chrono::Utc::now().to_rfc3339(), entity_id, profile],
            )
            .map(|_| ())
            .map_err(|error| error.to_string())
    } else {
        connection
            .execute(
                &format!("DELETE FROM {} WHERE id=?1 AND user_id=?2", spec.table),
                params![entity_id, profile],
            )
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::migration_runner::{run, MigrationContext};
    use crate::database::migrations::all;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn db(label: &str) -> (Connection, String) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("lifetrace-sync-execution-{label}-{unique}"));
        std::fs::create_dir_all(&dir).unwrap();
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run(&mut connection, &MigrationContext::new(dir), &all()).unwrap();
        let profile = crate::database::profile::active_profile_id(&connection).unwrap();
        (connection, profile)
    }

    #[test]
    fn execution_payload_round_trips_to_real_tables() {
        let (source, source_profile) = db("source");
        let (target, target_profile) = db("target");
        source.execute(
            "INSERT INTO execution_goals(id,user_id,name,status,created_at,updated_at) VALUES('g1',?1,'Graduate','active','2026-08-09T00:00:00Z','2026-08-09T00:00:00Z')",
            [&source_profile],
        ).unwrap();
        source
            .execute(
                "INSERT INTO execution_projects(id,user_id,name,status,goal_id,created_at,updated_at)
             VALUES('p1',?1,'Launch','active','g1','2026-08-09T00:00:00Z','2026-08-09T00:00:00Z')",
                [&source_profile],
            )
            .unwrap();
        source.execute(
            "INSERT INTO execution_tasks(id,user_id,project_id,title,status,priority,created_at,updated_at)
             VALUES('t1',?1,'p1','Ship EPIC20','todo','high','2026-08-09T00:00:00Z','2026-08-09T00:00:00Z')",
            [&source_profile],
        ).unwrap();
        source.execute(
            "INSERT INTO execution_memos(id,user_id,content,plain_text,status,created_at,updated_at)
             VALUES('m1',?1,'Remember sync','Remember sync','active','2026-08-09T00:00:00Z','2026-08-09T00:00:00Z')",
            [&source_profile],
        ).unwrap();

        for (entity_type, entity_id) in [
            ("execution.goal", "g1"),
            ("execution.project", "p1"),
            ("execution.task", "t1"),
            ("execution.memo", "m1"),
        ] {
            let local = load_local_entity(&source, &source_profile, entity_type, entity_id)
                .unwrap()
                .unwrap();
            let wire =
                crate::sync::payload::legacy_to_wire(entity_type, &local, &source_profile, None)
                    .unwrap();
            let legacy = crate::sync::payload::wire_to_legacy(&wire).unwrap();
            apply_upsert(&target, &target_profile, entity_type, &legacy).unwrap();
        }

        let goal_name: String = target.query_row(
            "SELECT name FROM execution_goals WHERE id='g1' AND user_id=?1", [&target_profile], |row| row.get(0)
        ).unwrap();
        assert_eq!(goal_name, "Graduate");
        let project_goal: String = target.query_row(
            "SELECT goal_id FROM execution_projects WHERE id='p1' AND user_id=?1", [&target_profile], |row| row.get(0)
        ).unwrap();
        assert_eq!(project_goal, "g1");
        let title: String = target
            .query_row(
                "SELECT title FROM execution_tasks WHERE id='t1' AND user_id=?1",
                [&target_profile],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(title, "Ship EPIC20");
        let memo: String = target
            .query_row(
                "SELECT plain_text FROM execution_memos WHERE id='m1' AND user_id=?1",
                [&target_profile],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(memo, "Remember sync");
    }

    #[test]
    fn memo_tag_relation_and_soft_delete_round_trip() {
        let (connection, profile) = db("relation");
        connection.execute(
            "INSERT INTO execution_memos(id,user_id,content,plain_text,status,created_at,updated_at)
             VALUES('m1',?1,'memo','memo','active','2026-08-09T00:00:00Z','2026-08-09T00:00:00Z')",
            [&profile],
        ).unwrap();
        connection.execute(
            "INSERT INTO execution_memo_tags(id,user_id,name,normalized_name,created_at,updated_at)
             VALUES('tag1',?1,'Work','work','2026-08-09T00:00:00Z','2026-08-09T00:00:00Z')",
            [&profile],
        ).unwrap();
        let relation = serde_json::json!({
            "memoId":"m1","tagId":"tag1","createdAt":"2026-08-09T00:00:00Z"
        });
        apply_upsert(&connection, &profile, MEMO_TAG_RELATION, &relation).unwrap();
        assert!(
            load_local_entity(&connection, &profile, MEMO_TAG_RELATION, "m1:tag1")
                .unwrap()
                .is_some()
        );
        apply_delete(&connection, &profile, MEMO_TAG_RELATION, "m1:tag1").unwrap();
        assert!(
            load_local_entity(&connection, &profile, MEMO_TAG_RELATION, "m1:tag1")
                .unwrap()
                .is_none()
        );

        apply_delete(&connection, &profile, "execution.memo", "m1").unwrap();
        let deleted: Option<String> = connection
            .query_row(
                "SELECT deleted_at FROM execution_memos WHERE id='m1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(deleted.is_some());
    }

    #[test]
    fn offline_core_entities_sync_into_second_device_real_tables() {
        let (source, source_profile) = db("offline-source");
        let (target, target_profile) = db("offline-target");
        let stamp = "2026-08-09T00:00:00Z";

        source.execute("INSERT INTO execution_tasks(id,user_id,title,status,priority,created_at,updated_at) VALUES('task-sync',?1,'Offline Task','todo','normal',?2,?2)", params![source_profile, stamp]).unwrap();
        source.execute("INSERT INTO execution_calendar_events(id,user_id,title,is_all_day,start_at,end_at,status,created_at,updated_at) VALUES('event-sync',?1,'Focus',0,'2026-08-09T02:00:00Z','2026-08-09T03:00:00Z','scheduled',?2,?2)", params![source_profile, stamp]).unwrap();
        source.execute("INSERT INTO execution_waiting_items(id,user_id,title,status,waiting_for,source_task_id,created_at,updated_at) VALUES('waiting-sync',?1,'Waiting','open','Alice','task-sync',?2,?2)", params![source_profile, stamp]).unwrap();
        source.execute("INSERT INTO execution_memos(id,user_id,content,plain_text,is_pinned,status,created_at,updated_at) VALUES('memo-sync',?1,'Remember','Remember',0,'active',?2,?2)", params![source_profile, stamp]).unwrap();
        source.execute("INSERT INTO execution_reminders(id,user_id,subject_type,subject_id,trigger_at,status,fire_key,created_at,updated_at) VALUES('reminder-sync',?1,'task','task-sync','2026-08-10T00:00:00Z','scheduled','task-sync@2026-08-10',?2,?2)", params![source_profile, stamp]).unwrap();

        let queued: i64 = source.query_row("SELECT COUNT(*) FROM sync_outbox WHERE profile_id=?1 AND entity_type LIKE 'execution.%' AND status='pending'", [&source_profile], |row| row.get(0)).unwrap();
        assert_eq!(
            queued, 5,
            "offline writes must be captured before reconnect"
        );

        target
            .execute(
                "UPDATE sync_context SET origin='remote' WHERE singleton=1",
                [],
            )
            .unwrap();
        for (entity_type, entity_id) in [
            ("execution.task", "task-sync"),
            ("execution.calendar_event", "event-sync"),
            ("execution.waiting_item", "waiting-sync"),
            ("execution.memo", "memo-sync"),
            ("execution.reminder", "reminder-sync"),
        ] {
            let local = load_local_entity(&source, &source_profile, entity_type, entity_id)
                .unwrap()
                .unwrap();
            let wire = crate::sync::payload::legacy_to_wire(
                entity_type,
                &local,
                &source_profile,
                Some("1"),
            )
            .unwrap();
            let legacy = crate::sync::payload::wire_to_legacy(&wire).unwrap();
            apply_upsert(&target, &target_profile, entity_type, &legacy).unwrap();
        }
        let target_outbox: i64 = target
            .query_row(
                "SELECT COUNT(*) FROM sync_outbox WHERE entity_type LIKE 'execution.%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(target_outbox, 0, "remote pull must not echo into outbox");
        let task_status: String = target
            .query_row(
                "SELECT status FROM execution_tasks WHERE id='task-sync' AND user_id=?1",
                [&target_profile],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(task_status, "todo");
        let event_title: String = target
            .query_row(
                "SELECT title FROM execution_calendar_events WHERE id='event-sync' AND user_id=?1",
                [&target_profile],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(event_title, "Focus");
        let waiting_for: String = target.query_row("SELECT waiting_for FROM execution_waiting_items WHERE id='waiting-sync' AND user_id=?1", [&target_profile], |row| row.get(0)).unwrap();
        assert_eq!(waiting_for, "Alice");
        let memo: String = target
            .query_row(
                "SELECT content FROM execution_memos WHERE id='memo-sync' AND user_id=?1",
                [&target_profile],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(memo, "Remember");
        let reminder_status: String = target
            .query_row(
                "SELECT status FROM execution_reminders WHERE id='reminder-sync' AND user_id=?1",
                [&target_profile],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(reminder_status, "scheduled");

        source.execute("UPDATE execution_tasks SET status='done',completed_at='2026-08-09T01:00:00Z',updated_at='2026-08-09T01:00:00Z',version=version+1 WHERE id='task-sync'", []).unwrap();
        let task_local = load_local_entity(&source, &source_profile, "execution.task", "task-sync")
            .unwrap()
            .unwrap();
        let task_wire = crate::sync::payload::legacy_to_wire(
            "execution.task",
            &task_local,
            &source_profile,
            Some("2"),
        )
        .unwrap();
        let task_legacy = crate::sync::payload::wire_to_legacy(&task_wire).unwrap();
        apply_upsert(&target, &target_profile, "execution.task", &task_legacy).unwrap();
        let task_status: String = target
            .query_row(
                "SELECT status FROM execution_tasks WHERE id='task-sync'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(task_status, "done");

        source.execute("UPDATE execution_memos SET deleted_at='2026-08-09T02:00:00Z',updated_at='2026-08-09T02:00:00Z',version=version+1 WHERE id='memo-sync'", []).unwrap();
        let memo_operation: String = source.query_row("SELECT operation FROM sync_outbox WHERE profile_id=?1 AND entity_type='execution.memo' AND entity_id='memo-sync' AND status='pending'", [&source_profile], |row| row.get(0)).unwrap();
        assert_eq!(memo_operation, "delete");
        apply_delete(&target, &target_profile, "execution.memo", "memo-sync").unwrap();
        let deleted: Option<String> = target
            .query_row(
                "SELECT deleted_at FROM execution_memos WHERE id='memo-sync'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(deleted.is_some());
        target
            .execute(
                "UPDATE sync_context SET origin='local' WHERE singleton=1",
                [],
            )
            .unwrap();
    }
}
