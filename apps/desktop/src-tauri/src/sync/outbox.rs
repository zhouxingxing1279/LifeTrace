use chrono::Utc;
use lifetrace_contracts::registry::EntityType;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use uuid::Uuid;

use crate::database::profile;

use super::payload::legacy_to_wire;

/// 写入来源。当前实现只从本地写入路径入队；`Remote` 与 `Migration`
/// 保留用于未来的远端应用/迁移路径（EPIC-05 WriteOrigin 语义）。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationOrigin {
    Local,
    Remote,
    Migration,
}

pub fn enqueue_upsert(
    connection: &Connection,
    entity_type: &str,
    value: &Value,
    atomic_group_id: Option<&str>,
    origin: MutationOrigin,
) -> Result<Option<String>, String> {
    if origin != MutationOrigin::Local {
        return Ok(None);
    }
    let entity_id = value
        .get("id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "outbox entity is missing id".to_owned())?;
    enqueue(
        connection,
        entity_type,
        entity_id,
        "upsert",
        Some(value),
        atomic_group_id,
    )
}

/// Enqueue a local tombstone after the local repository has accepted a delete.
/// Remote/migration application must never call this as `Local`, otherwise a
/// pulled delete would be echoed back to the server.
pub fn enqueue_delete(
    connection: &Connection,
    entity_type: &str,
    entity_id: &str,
    atomic_group_id: Option<&str>,
    origin: MutationOrigin,
) -> Result<Option<String>, String> {
    if origin != MutationOrigin::Local {
        return Ok(None);
    }
    if entity_id.trim().is_empty() {
        return Err("outbox delete entity is missing id".to_owned());
    }
    enqueue(
        connection,
        entity_type,
        entity_id,
        "delete",
        None,
        atomic_group_id,
    )
}

fn enqueue(
    connection: &Connection,
    entity_type: &str,
    entity_id: &str,
    operation: &str,
    value: Option<&Value>,
    atomic_group_id: Option<&str>,
) -> Result<Option<String>, String> {
    if !super::payload::is_syncable(entity_type) {
        return Ok(None);
    }
    let profile_id = profile::active_profile_id(connection)?;
    let base_version: Option<String> = connection.query_row(
        "SELECT server_version FROM sync_metadata WHERE profile_id=?1 AND entity_type=?2 AND entity_id=?3",
        params![profile_id, entity_type, entity_id], |row| row.get(0)
    ).optional().map_err(|error| error.to_string())?.flatten();
    let payload = match value {
        Some(value) => Some(legacy_to_wire(
            entity_type,
            value,
            &profile_id,
            base_version.as_deref(),
        )?),
        None => None,
    };
    // A not-yet-sent mutation is safely coalesced. Leased rows are immutable
    // because the server may have accepted their changeId already.
    connection.execute(
        "DELETE FROM sync_outbox WHERE profile_id=?1 AND entity_type=?2 AND entity_id=?3 AND status='pending'",
        params![profile_id, entity_type, entity_id]
    ).map_err(|error| error.to_string())?;
    let change_id = Uuid::new_v4().to_string();
    let stamp = Utc::now().to_rfc3339();
    connection
        .execute(
            "INSERT INTO sync_outbox(
           change_id,profile_id,entity_type,entity_id,operation,base_server_version,
           entity_schema_version,payload_json,dependencies_json,atomic_group_id,status,
           retry_count,created_at,updated_at
         ) VALUES(?1,?2,?3,?4,?5,?6,1,?7,'[]',?8,'pending',0,?9,?9)",
            params![
                change_id,
                profile_id,
                entity_type,
                entity_id,
                operation,
                base_version.as_deref().unwrap_or("0"),
                payload.map(|value| value.to_string()),
                atomic_group_id,
                stamp
            ],
        )
        .map_err(|error| error.to_string())?;
    connection.execute(
        "INSERT INTO sync_audit_log(id,profile_id,event_type,entity_type,entity_id,details_json,created_at)
         VALUES(?1,?2,'outbox_enqueued',?3,?4,?5,?6)",
        params![Uuid::new_v4().to_string(), profile_id, entity_type, entity_id,
            serde_json::json!({"changeId": change_id, "operation": operation}).to_string(), stamp]
    ).map_err(|error| error.to_string())?;
    Ok(Some(change_id))
}

/// Queue all existing user-owned rows when the user explicitly chooses to bind
/// the current local profile. This is never called automatically on login.
pub fn enqueue_existing_profile(
    connection: &Connection,
    profile_id: &str,
) -> Result<usize, String> {
    let mut total = 0usize;
    let sources: [(&str, Vec<Value>); 10] = [
        (
            EntityType::FINANCE_ACCOUNT,
            crate::database::repositories::finance::list_accounts(connection)?,
        ),
        (
            EntityType::FINANCE_TRANSACTION,
            crate::database::repositories::finance::list_transactions(connection)?,
        ),
        (
            EntityType::HABIT_ACTIVITY,
            crate::database::repositories::habits::list_activities(connection)?,
        ),
        (
            EntityType::HABIT_LOG,
            crate::database::repositories::habits::list_activity_logs(connection)?,
        ),
        (
            EntityType::REVIEW_DAILY,
            crate::database::repositories::habits::list_daily_reviews(connection)?,
        ),
        (
            EntityType::WORKOUT_WORKOUT,
            crate::database::repositories::workouts::list_workouts(connection)?,
        ),
        (
            EntityType::WORKOUT_IMPORT,
            crate::database::repositories::workouts::list_imports(connection)?,
        ),
        (
            EntityType::ENGLISH_LEARNING_RECORD,
            crate::database::repositories::english::list(connection, "records")?,
        ),
        (
            EntityType::ENGLISH_HIGHLIGHT,
            crate::database::repositories::english::list(connection, "highlights")?,
        ),
        (
            EntityType::ENGLISH_VOCABULARY,
            crate::database::repositories::english::list(connection, "vocabulary")?,
        ),
    ];
    for (entity_type, values) in sources {
        for mut value in values {
            if value
                .get("userId")
                .and_then(Value::as_str)
                .is_some_and(|owner| owner != profile_id)
            {
                continue;
            }
            if let Some(object) = value.as_object_mut() {
                object.insert("userId".to_owned(), Value::String(profile_id.to_owned()));
            }
            if enqueue_upsert(connection, entity_type, &value, None, MutationOrigin::Local)?
                .is_some()
            {
                total += 1;
            }
        }
    }
    for (entity_type, value) in super::execution::existing_entities(connection, profile_id)? {
        if enqueue_upsert(connection, entity_type, &value, None, MutationOrigin::Local)?.is_some() {
            total += 1;
        }
    }
    let mut ids = connection
        .prepare("SELECT id FROM notes WHERE user_id=?1 AND deleted_at IS NULL")
        .map_err(|error| error.to_string())?;
    let note_ids = ids
        .query_map([profile_id], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|error| error.to_string())?;
    drop(ids);
    for id in note_ids {
        if let Some(note) = crate::database::repositories::notes::get_note(connection, &id)? {
            if enqueue_upsert(
                connection,
                EntityType::NOTE_NOTE,
                &note,
                None,
                MutationOrigin::Local,
            )?
            .is_some()
            {
                total += 1;
            }
        }
    }
    for (table, entity_type, columns) in [
        (
            "note_folders",
            EntityType::NOTE_FOLDER,
            "id,name,icon,color,sort_order,created_at,updated_at",
        ),
        (
            "note_tags",
            EntityType::NOTE_TAG,
            "id,name,'' AS icon,color,0 AS sort_order,created_at,updated_at",
        ),
    ] {
        let sql = format!("SELECT {columns} FROM {table} WHERE user_id=?1 AND deleted_at IS NULL");
        let mut statement = connection
            .prepare(&sql)
            .map_err(|error| error.to_string())?;
        let values = statement
            .query_map([profile_id], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_,String>(0)?, "name": row.get::<_,String>(1)?,
                    "icon": row.get::<_,String>(2)?, "color": row.get::<_,String>(3)?,
                    "sortOrder": row.get::<_,i64>(4)?, "createdAt": row.get::<_,String>(5)?,
                    "updatedAt": row.get::<_,String>(6)?, "userId": profile_id,
                }))
            })
            .map_err(|error| error.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?;
        for value in values {
            if enqueue_upsert(connection, entity_type, &value, None, MutationOrigin::Local)?
                .is_some()
            {
                total += 1;
            }
        }
    }
    Ok(total)
}
