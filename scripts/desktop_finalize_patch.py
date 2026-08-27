from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected exactly one match, found {count}\n--- pattern ---\n{old[:500]}")
    file.write_text(text.replace(old, new, 1), encoding="utf-8")


# English: local business writes are SQLite + Outbox in the same transaction.
path = "apps/desktop/src-tauri/src/server/english.rs"
replace_once(
    path,
    "use chrono::{Duration, Utc};\nuse regex::Regex;",
    "use chrono::{Duration, Utc};\nuse lifetrace_contracts::registry::EntityType;\nuse regex::Regex;",
)
replace_once(
    path,
    "use uuid::Uuid;\n\nuse super::AppState;",
    "use uuid::Uuid;\n\nuse crate::sync::outbox::{enqueue_delete, enqueue_upsert, MutationOrigin};\n\nuse super::AppState;",
)
replace_once(
    path,
    '''fn table(key: &str) -> Option<&'static str> {
    JSON_ENTITY_TABLES
        .iter()
        .find_map(|(name, table)| (*name == key).then_some(*table))
}

fn put(connection: &Connection, key: &str, value: &Value) -> Result<(), String> {
    let Some(table) = table(key) else {
        return crate::database::repositories::english::put(connection, key, value);
    };
    let entity_id = value
        .get("id")
        .or_else(|| value.get("taskId"))
        .and_then(Value::as_str)
        .ok_or_else(|| "数据缺少 id".to_owned())?;
    let stamp = value
        .get("updatedAt")
        .and_then(Value::as_str)
        .unwrap_or_else(|| "");
    let stamp = if stamp.is_empty() {
        now()
    } else {
        stamp.to_owned()
    };
    connection
        .execute(
            &format!(
                "INSERT INTO {table}(id,data_json,updated_at) VALUES(?1,?2,?3)\n                 ON CONFLICT(id) DO UPDATE SET data_json=excluded.data_json,updated_at=excluded.updated_at"
            ),
            params![entity_id, value.to_string(), stamp],
        )
        .map_err(|value| value.to_string())?;
    Ok(())
}
''',
    '''fn table(key: &str) -> Option<&'static str> {
    JSON_ENTITY_TABLES
        .iter()
        .find_map(|(name, table)| (*name == key).then_some(*table))
}

fn sync_entity_type(key: &str) -> Option<&'static str> {
    match key {
        "records" => Some(EntityType::ENGLISH_LEARNING_RECORD),
        "highlights" => Some(EntityType::ENGLISH_HIGHLIGHT),
        "notes" => Some(EntityType::ENGLISH_NOTE),
        "vocabulary" => Some(EntityType::ENGLISH_VOCABULARY),
        _ => None,
    }
}

fn put(connection: &Connection, key: &str, value: &Value) -> Result<(), String> {
    if let Some(table) = table(key) {
        let entity_id = value
            .get("id")
            .or_else(|| value.get("taskId"))
            .and_then(Value::as_str)
            .ok_or_else(|| "数据缺少 id".to_owned())?;
        let stamp = value
            .get("updatedAt")
            .and_then(Value::as_str)
            .unwrap_or_else(|| "");
        let stamp = if stamp.is_empty() {
            now()
        } else {
            stamp.to_owned()
        };
        connection
            .execute(
                &format!(
                    "INSERT INTO {table}(id,data_json,updated_at) VALUES(?1,?2,?3)\n                     ON CONFLICT(id) DO UPDATE SET data_json=excluded.data_json,updated_at=excluded.updated_at"
                ),
                params![entity_id, value.to_string(), stamp],
            )
            .map_err(|value| value.to_string())?;
        return Ok(());
    }

    let repository = &crate::database::repositories::english;
    let Some(entity_type) = sync_entity_type(key) else {
        return repository::put(connection, key, value);
    };
    let entity_id = value
        .get("id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "本地英语同步实体缺少 id".to_owned())?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    repository::put(&transaction, key, value)?;
    let stored = repository::get(&transaction, key, entity_id)?
        .ok_or_else(|| "英语实体写入后无法重新读取".to_owned())?;
    enqueue_upsert(
        &transaction,
        entity_type,
        &stored,
        None,
        MutationOrigin::Local,
    )?;
    transaction.commit().map_err(|error| error.to_string())
}
''',
)
replace_once(
    path,
    '''fn remove(connection: &Connection, key: &str, entity_id: &str) -> Result<bool, String> {
    let Some(table) = table(key) else {
        return crate::database::repositories::english::remove(connection, key, entity_id);
    };
    connection
        .execute(&format!("DELETE FROM {table} WHERE id=?1"), [entity_id])
        .map(|count| count > 0)
        .map_err(|value| value.to_string())
}
''',
    '''fn remove(connection: &Connection, key: &str, entity_id: &str) -> Result<bool, String> {
    if let Some(table) = table(key) {
        return connection
            .execute(&format!("DELETE FROM {table} WHERE id=?1"), [entity_id])
            .map(|count| count > 0)
            .map_err(|value| value.to_string());
    }
    let repository = &crate::database::repositories::english;
    let Some(entity_type) = sync_entity_type(key) else {
        return repository::remove(connection, key, entity_id);
    };
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| error.to_string())?;
    let removed = repository::remove(&transaction, key, entity_id)?;
    if removed {
        enqueue_delete(
            &transaction,
            entity_type,
            entity_id,
            None,
            MutationOrigin::Local,
        )?;
    }
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(removed)
}
''',
)
# Add focused local-first assertions at the end of the English server module.
english_tests = r'''

#[cfg(test)]
mod local_first_tests {
    use super::*;
    use crate::database::migration_runner::{run, MigrationContext};
    use crate::database::migrations::all;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn database() -> Connection {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("lifetrace-english-local-first-{unique}"));
        std::fs::create_dir_all(&directory).unwrap();
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        run(&mut connection, &MigrationContext::new(directory), &all()).unwrap();
        ensure_schema(&connection).unwrap();
        connection
    }

    fn pending_operation(connection: &Connection, entity_type: &str, entity_id: &str) -> String {
        connection
            .query_row(
                "SELECT operation FROM sync_outbox WHERE entity_type=?1 AND entity_id=?2 AND status='pending' ORDER BY created_at DESC LIMIT 1",
                params![entity_type, entity_id],
                |row| row.get(0),
            )
            .unwrap()
    }

    #[test]
    fn core_english_mutations_enqueue_local_sync_changes() {
        let connection = database();
        let record = reading(&connection, "local-healthy-habits", Some("start"), 12).unwrap();
        let record_id = record.get("id").and_then(Value::as_str).unwrap();
        assert_eq!(
            pending_operation(&connection, EntityType::ENGLISH_LEARNING_RECORD, record_id),
            "upsert"
        );

        let highlight = upsert_annotation(
            &connection,
            "highlights",
            json!({"articleId":"local-healthy-habits","text":"lasting change","color":"yellow"}),
        )
        .unwrap();
        let highlight_id = highlight.get("id").and_then(Value::as_str).unwrap();
        assert_eq!(
            pending_operation(&connection, EntityType::ENGLISH_HIGHLIGHT, highlight_id),
            "upsert"
        );

        let vocabulary = add_vocabulary(
            &connection,
            json!({"word":"lasting","selectedMeanings":["持续的"],"sourceArticleId":"local-healthy-habits"}),
        )
        .unwrap();
        let vocabulary_id = vocabulary.get("id").and_then(Value::as_str).unwrap();
        assert_eq!(
            pending_operation(&connection, EntityType::ENGLISH_VOCABULARY, vocabulary_id),
            "upsert"
        );
        assert!(remove(&connection, "vocabulary", vocabulary_id).unwrap());
        assert_eq!(
            pending_operation(&connection, EntityType::ENGLISH_VOCABULARY, vocabulary_id),
            "delete"
        );
    }
}
'''
file = Path(path)
text = file.read_text(encoding="utf-8")
if "mod local_first_tests" not in text:
    file.write_text(text + english_tests, encoding="utf-8")


# First profile binding should include categories and English notes as existing user data.
path = "apps/desktop/src-tauri/src/sync/outbox.rs"
replace_once(path, "let sources: [(&str, Vec<Value>); 10] = [", "let sources: [(&str, Vec<Value>); 12] = [")
replace_once(
    path,
    '''        (
            EntityType::FINANCE_TRANSACTION,
            crate::database::repositories::finance::list_transactions(connection)?,
        ),
        (
            EntityType::HABIT_ACTIVITY,''',
    '''        (
            EntityType::FINANCE_TRANSACTION,
            crate::database::repositories::finance::list_transactions(connection)?,
        ),
        (
            EntityType::FINANCE_CATEGORY,
            crate::database::repositories::finance::list_categories(connection)?,
        ),
        (
            EntityType::HABIT_ACTIVITY,''',
)
replace_once(
    path,
    '''        (
            EntityType::ENGLISH_HIGHLIGHT,
            crate::database::repositories::english::list(connection, "highlights")?,
        ),
        (
            EntityType::ENGLISH_VOCABULARY,''',
    '''        (
            EntityType::ENGLISH_HIGHLIGHT,
            crate::database::repositories::english::list(connection, "highlights")?,
        ),
        (
            EntityType::ENGLISH_NOTE,
            crate::database::repositories::english::list(connection, "notes")?,
        ),
        (
            EntityType::ENGLISH_VOCABULARY,''',
)


# Task deletion: keep task tombstone in the same SQLite transaction as the soft-delete.
path = "apps/desktop/src-tauri/src/execution.rs"
replace_once(
    path,
    "use chrono::{DateTime, Utc};\nuse rusqlite::Connection;",
    "use chrono::{DateTime, Utc};\nuse lifetrace_contracts::registry::EntityType;\nuse rusqlite::Connection;",
)
replace_once(
    path,
    '''use crate::database::{
    profile,
    repositories::execution::{
        self as repository, ProjectRecord, ProjectWrite, TaskListFilter, TaskRecord, TaskWrite,
    },
};''',
    '''use crate::{
    database::{
        profile,
        repositories::execution::{
            self as repository, ProjectRecord, ProjectWrite, TaskListFilter, TaskRecord, TaskWrite,
        },
    },
    sync::outbox::{enqueue_delete, MutationOrigin},
};''',
)
replace_once(
    path,
    '''    crate::execution_relation::clear_completion_for_task(&transaction, &user_id, id)?;
    transaction
        .commit()
        .map_err(|error| ExecutionError::storage(error.to_string()))?;''',
    '''    crate::execution_relation::clear_completion_for_task(&transaction, &user_id, id)?;
    enqueue_delete(
        &transaction,
        EntityType::EXECUTION_TASK,
        id,
        None,
        MutationOrigin::Local,
    )
    .map_err(ExecutionError::storage)?;
    transaction
        .commit()
        .map_err(|error| ExecutionError::storage(error.to_string()))?;''',
)
replace_once(
    path,
    '''        delete_task(&connection, &task.id).unwrap();
        delete_project(&connection, &project.id).unwrap();''',
    '''        delete_task(&connection, &task.id).unwrap();
        let operation: String = connection
            .query_row(
                "SELECT operation FROM sync_outbox WHERE entity_type=?1 AND entity_id=?2 AND status='pending'",
                [EntityType::EXECUTION_TASK, task.id.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(operation, "delete");
        delete_project(&connection, &project.id).unwrap();''',
)

# Server no longer needs a second post-commit task tombstone.
path = "apps/desktop/src-tauri/src/server/execution.rs"
replace_once(
    path,
    '''    // `execution::delete_task` currently owns its transaction because it also
    // clears completion relations. Queue the tombstone immediately afterwards;
    // a later service cleanup can fold both operations into one transaction.
    if let Err(error) = execution::delete_task(&connection, &id) {
        return execution_error(error);
    }
    if let Err(error) = enqueue_delete(
        &connection,
        EntityType::EXECUTION_TASK,
        &id,
        None,
        MutationOrigin::Local,
    ) {
        return storage_error(error);
    }
    Json(OkResponse { ok: true }).into_response()''',
    '''    if let Err(error) = execution::delete_task(&connection, &id) {
        return execution_error(error);
    }
    Json(OkResponse { ok: true }).into_response()''',
)


# Calendar service: local mutations enqueue sync records; Pull applies via sync/execution.rs and bypasses this layer.
path = "apps/desktop/src-tauri/src/execution_calendar.rs"
replace_once(
    path,
    "use chrono::{DateTime, NaiveDate, Utc};\nuse rusqlite::Connection;",
    "use chrono::{DateTime, NaiveDate, Utc};\nuse lifetrace_contracts::registry::EntityType;\nuse rusqlite::Connection;",
)
replace_once(
    path,
    '''    execution::{self, ExecutionError, ExecutionErrorKind, ExecutionResult},
    execution_structure::RecurrenceRuleInput,
};''',
    '''    execution::{self, ExecutionError, ExecutionErrorKind, ExecutionResult},
    execution_structure::RecurrenceRuleInput,
    sync::outbox::{enqueue_delete, enqueue_upsert, MutationOrigin},
};''',
)
replace_once(
    path,
    '''fn active_user(connection: &Connection) -> ExecutionResult<String> {
    profile::active_profile_id(connection).map_err(storage)
}
''',
    '''fn active_user(connection: &Connection) -> ExecutionResult<String> {
    profile::active_profile_id(connection).map_err(storage)
}

fn enqueue_record<T: Serialize>(
    connection: &Connection,
    entity_type: &str,
    record: &T,
) -> ExecutionResult<()> {
    let value = serde_json::to_value(record).map_err(|error| storage(error.to_string()))?;
    enqueue_upsert(connection, entity_type, &value, None, MutationOrigin::Local)
        .map_err(storage)?;
    Ok(())
}

fn enqueue_tombstone(
    connection: &Connection,
    entity_type: &str,
    entity_id: &str,
) -> ExecutionResult<()> {
    enqueue_delete(
        connection,
        entity_type,
        entity_id,
        None,
        MutationOrigin::Local,
    )
    .map_err(storage)?;
    Ok(())
}
''',
)
replace_once(
    path,
    '''    ensure_source_task(connection, &user_id, write.source_task_id.as_deref())?;
    repository::save_event(connection, &write).map_err(storage)
}''',
    '''    ensure_source_task(connection, &user_id, write.source_task_id.as_deref())?;
    let event = repository::save_event(connection, &write).map_err(storage)?;
    enqueue_record(connection, EntityType::EXECUTION_CALENDAR_EVENT, &event)?;
    Ok(event)
}''',
)
replace_once(
    path,
    '''    ensure_source_task(connection, &user_id, write.source_task_id.as_deref())?;
    repository::save_event(connection, &write).map_err(storage)
}

pub fn move_event''',
    '''    ensure_source_task(connection, &user_id, write.source_task_id.as_deref())?;
    let event = repository::save_event(connection, &write).map_err(storage)?;
    enqueue_record(connection, EntityType::EXECUTION_CALENDAR_EVENT, &event)?;
    Ok(event)
}

pub fn move_event''',
)
replace_once(
    path,
    '''    repository::save_event(connection, &write).map_err(storage)
}

pub fn cancel_event''',
    '''    let event = repository::save_event(connection, &write).map_err(storage)?;
    enqueue_record(connection, EntityType::EXECUTION_CALENDAR_EVENT, &event)?;
    Ok(event)
}

pub fn cancel_event''',
)
replace_once(
    path,
    '''    repository::save_event(connection, &write).map_err(storage)
}

pub fn delete_event''',
    '''    let event = repository::save_event(connection, &write).map_err(storage)?;
    enqueue_record(connection, EntityType::EXECUTION_CALENDAR_EVENT, &event)?;
    Ok(event)
}

pub fn delete_event''',
)
replace_once(
    path,
    '''    if repository::soft_delete_event(connection, &user_id, id).map_err(storage)? {
        Ok(())
    } else {''',
    '''    if repository::soft_delete_event(connection, &user_id, id).map_err(storage)? {
        enqueue_tombstone(connection, EntityType::EXECUTION_CALENDAR_EVENT, id)?;
        Ok(())
    } else {''',
)
replace_once(
    path,
    '''    repository::create_task_schedule_link(&transaction, &user_id, task_id, &event.id)
        .map_err(storage)?;
    transaction
        .commit()''',
    '''    repository::create_task_schedule_link(&transaction, &user_id, task_id, &event.id)
        .map_err(storage)?;
    enqueue_record(
        &transaction,
        EntityType::EXECUTION_CALENDAR_EVENT,
        &event,
    )?;
    transaction
        .commit()''',
)
replace_once(
    path,
    '''    repository::set_event_recurrence_rule(&transaction, &user_id, event_id, Some(&rule.id))
        .map_err(storage)?;
    transaction
        .commit()''',
    '''    repository::set_event_recurrence_rule(&transaction, &user_id, event_id, Some(&rule.id))
        .map_err(storage)?;
    enqueue_record(
        &transaction,
        EntityType::EXECUTION_RECURRENCE_RULE,
        &rule,
    )?;
    let updated_event = repository::get_event(&transaction, &user_id, event_id)
        .map_err(storage)?
        .ok_or_else(|| not_found("日历事件不存在"))?;
    enqueue_record(
        &transaction,
        EntityType::EXECUTION_CALENDAR_EVENT,
        &updated_event,
    )?;
    transaction
        .commit()''',
)
replace_once(
    path,
    '''    recurrence_repository::soft_delete_recurrence_rule(&transaction, &user_id, &rule_id)
        .map_err(storage)?;
    transaction
        .commit()''',
    '''    recurrence_repository::soft_delete_recurrence_rule(&transaction, &user_id, &rule_id)
        .map_err(storage)?;
    enqueue_tombstone(
        &transaction,
        EntityType::EXECUTION_RECURRENCE_RULE,
        &rule_id,
    )?;
    let updated_event = repository::get_event(&transaction, &user_id, event_id)
        .map_err(storage)?
        .ok_or_else(|| not_found("日历事件不存在"))?;
    enqueue_record(
        &transaction,
        EntityType::EXECUTION_CALENDAR_EVENT,
        &updated_event,
    )?;
    transaction
        .commit()''',
)
replace_once(
    path,
    '''    repository::create_occurrence(connection, &user_id, &write).map_err(storage)
}

pub fn update_occurrence''',
    '''    let occurrence = repository::create_occurrence(connection, &user_id, &write).map_err(storage)?;
    enqueue_record(
        connection,
        EntityType::EXECUTION_CALENDAR_OCCURRENCE,
        &occurrence,
    )?;
    Ok(occurrence)
}

pub fn update_occurrence''',
)
replace_once(
    path,
    '''    repository::update_occurrence(connection, &user_id, occurrence_id, &write).map_err(storage)
}

pub fn change_occurrence_status''',
    '''    let occurrence =
        repository::update_occurrence(connection, &user_id, occurrence_id, &write).map_err(storage)?;
    enqueue_record(
        connection,
        EntityType::EXECUTION_CALENDAR_OCCURRENCE,
        &occurrence,
    )?;
    Ok(occurrence)
}

pub fn change_occurrence_status''',
)
replace_once(
    path,
    '''    repository::update_occurrence(connection, &user_id, occurrence_id, &write).map_err(storage)
}

#[cfg(test)]''',
    '''    let occurrence =
        repository::update_occurrence(connection, &user_id, occurrence_id, &write).map_err(storage)?;
    enqueue_record(
        connection,
        EntityType::EXECUTION_CALENDAR_OCCURRENCE,
        &occurrence,
    )?;
    Ok(occurrence)
}

#[cfg(test)]''',
)
replace_once(
    path,
    '''        assert_eq!(event.start_local_date.as_deref(), Some("2026-11-01"));
        assert!(event.start_at.is_none());''',
    '''        assert_eq!(event.start_local_date.as_deref(), Some("2026-11-01"));
        assert!(event.start_at.is_none());
        let operation: String = connection
            .query_row(
                "SELECT operation FROM sync_outbox WHERE entity_type=?1 AND entity_id=?2 AND status='pending'",
                [EntityType::EXECUTION_CALENDAR_EVENT, event.id.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(operation, "upsert");''',
)

print("Desktop local-first finalize patch applied")
