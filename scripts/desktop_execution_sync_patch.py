from pathlib import Path


def read(path):
    return Path(path).read_text(encoding="utf-8")


def write(path, text):
    Path(path).write_text(text, encoding="utf-8")


def replace_once(path, old, new):
    text = read(path)
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected 1 match, got {count}\n{old[:400]}")
    write(path, text.replace(old, new, 1))


def replace_all(path, old, new, minimum=1):
    text = read(path)
    count = text.count(old)
    if count < minimum:
        raise SystemExit(f"{path}: expected >= {minimum} matches, got {count}\n{old[:400]}")
    write(path, text.replace(old, new))


# ---------------------------------------------------------------------------
# Shared Execution Outbox helpers: always serialize the exact persisted row.
# ---------------------------------------------------------------------------
path = "apps/desktop/src-tauri/src/sync/outbox.rs"
marker = '''fn enqueue(
    connection: &Connection,
    entity_type: &str,
'''
insert = '''/// Queue a persisted Execution entity using the canonical SQLite-to-wire adapter.
/// This keeps local business services from hand-serializing partial DTOs.
pub fn enqueue_execution_upsert(
    connection: &Connection,
    entity_type: &str,
    entity_id: &str,
) -> Result<Option<String>, String> {
    let profile_id = profile::active_profile_id(connection)?;
    let value = super::execution::load_local_entity(connection, &profile_id, entity_type, entity_id)?
        .ok_or_else(|| format!("execution entity missing after local write: {entity_type}/{entity_id}"))?;
    enqueue_upsert(
        connection,
        entity_type,
        &value,
        None,
        MutationOrigin::Local,
    )
}

/// Queue an Execution tombstone from a local domain mutation.
pub fn enqueue_execution_delete(
    connection: &Connection,
    entity_type: &str,
    entity_id: &str,
) -> Result<Option<String>, String> {
    enqueue_delete(
        connection,
        entity_type,
        entity_id,
        None,
        MutationOrigin::Local,
    )
}

fn enqueue(
    connection: &Connection,
    entity_type: &str,
'''
replace_once(path, marker, insert)


# ---------------------------------------------------------------------------
# Core project/task domain owns Outbox so internal conversions are covered.
# ---------------------------------------------------------------------------
path = "apps/desktop/src-tauri/src/execution.rs"
replace_once(
    path,
    '    sync::outbox::{enqueue_delete, MutationOrigin},\n',
    '    sync::outbox::{enqueue_execution_delete, enqueue_execution_upsert},\n',
)
replace_once(
    path,
    '''fn active_user(connection: &Connection) -> ExecutionResult<String> {
    profile::active_profile_id(connection).map_err(ExecutionError::storage)
}
''',
    '''fn active_user(connection: &Connection) -> ExecutionResult<String> {
    profile::active_profile_id(connection).map_err(ExecutionError::storage)
}

pub(crate) fn local_transaction<T>(
    connection: &Connection,
    operation: impl FnOnce(&Connection) -> ExecutionResult<T>,
) -> ExecutionResult<T> {
    if !connection.is_autocommit() {
        return operation(connection);
    }
    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| ExecutionError::storage(error.to_string()))?;
    let result = operation(&transaction)?;
    transaction
        .commit()
        .map_err(|error| ExecutionError::storage(error.to_string()))?;
    Ok(result)
}

fn enqueue_saved(connection: &Connection, entity_type: &str, entity_id: &str) -> ExecutionResult<()> {
    enqueue_execution_upsert(connection, entity_type, entity_id)
        .map(|_| ())
        .map_err(ExecutionError::storage)
}

fn enqueue_removed(connection: &Connection, entity_type: &str, entity_id: &str) -> ExecutionResult<()> {
    enqueue_execution_delete(connection, entity_type, entity_id)
        .map(|_| ())
        .map_err(ExecutionError::storage)
}
''',
)
replace_once(
    path,
    '''    let user_id = active_user(connection)?;
    let input = normalize_project_input(user_id, None, input, "active")?;
    repository::save_project(connection, &input).map_err(ExecutionError::storage)
}''',
    '''    local_transaction(connection, |connection| {
        let user_id = active_user(connection)?;
        let input = normalize_project_input(user_id, None, input, "active")?;
        let project = repository::save_project(connection, &input).map_err(ExecutionError::storage)?;
        enqueue_saved(connection, EntityType::EXECUTION_PROJECT, &project.id)?;
        Ok(project)
    })
}''',
)
replace_once(
    path,
    '''    let user_id = active_user(connection)?;
    let current = repository::get_project(connection, &user_id, id)
        .map_err(ExecutionError::storage)?
        .ok_or_else(|| ExecutionError::not_found("项目不存在"))?;
    let input = normalize_project_input(user_id, Some(id.to_owned()), input, &current.status)?;
    repository::save_project(connection, &input).map_err(ExecutionError::storage)
}''',
    '''    local_transaction(connection, |connection| {
        let user_id = active_user(connection)?;
        let current = repository::get_project(connection, &user_id, id)
            .map_err(ExecutionError::storage)?
            .ok_or_else(|| ExecutionError::not_found("项目不存在"))?;
        let input = normalize_project_input(user_id, Some(id.to_owned()), input, &current.status)?;
        let project = repository::save_project(connection, &input).map_err(ExecutionError::storage)?;
        enqueue_saved(connection, EntityType::EXECUTION_PROJECT, &project.id)?;
        Ok(project)
    })
}''',
)
replace_once(
    path,
    '''    if repository::soft_delete_project(connection, &user_id, id).map_err(ExecutionError::storage)? {
        Ok(())
    } else {
        Err(ExecutionError::not_found("项目不存在"))
    }
}''',
    '''    local_transaction(connection, |connection| {
        if repository::soft_delete_project(connection, &user_id, id)
            .map_err(ExecutionError::storage)?
        {
            enqueue_removed(connection, EntityType::EXECUTION_PROJECT, id)?;
            Ok(())
        } else {
            Err(ExecutionError::not_found("项目不存在"))
        }
    })
}''',
)
replace_once(
    path,
    '''    let user_id = active_user(connection)?;
    let input = normalize_task_input(user_id.clone(), None, input, None)?;
    ensure_project_reference(connection, &user_id, input.project_id.as_deref())?;
    repository::save_task(connection, &input).map_err(ExecutionError::storage)
}''',
    '''    local_transaction(connection, |connection| {
        let user_id = active_user(connection)?;
        let input = normalize_task_input(user_id.clone(), None, input, None)?;
        ensure_project_reference(connection, &user_id, input.project_id.as_deref())?;
        let task = repository::save_task(connection, &input).map_err(ExecutionError::storage)?;
        enqueue_saved(connection, EntityType::EXECUTION_TASK, &task.id)?;
        Ok(task)
    })
}''',
)
replace_once(
    path,
    '''    let user_id = active_user(connection)?;
    let current = repository::get_task(connection, &user_id, id)
        .map_err(ExecutionError::storage)?
        .ok_or_else(|| ExecutionError::not_found("任务不存在"))?;
    let input = normalize_task_input(user_id.clone(), Some(id.to_owned()), input, Some(&current))?;
    ensure_project_reference(connection, &user_id, input.project_id.as_deref())?;
    repository::save_task(connection, &input).map_err(ExecutionError::storage)
}''',
    '''    local_transaction(connection, |connection| {
        let user_id = active_user(connection)?;
        let current = repository::get_task(connection, &user_id, id)
            .map_err(ExecutionError::storage)?
            .ok_or_else(|| ExecutionError::not_found("任务不存在"))?;
        let input = normalize_task_input(user_id.clone(), Some(id.to_owned()), input, Some(&current))?;
        ensure_project_reference(connection, &user_id, input.project_id.as_deref())?;
        let task = repository::save_task(connection, &input).map_err(ExecutionError::storage)?;
        enqueue_saved(connection, EntityType::EXECUTION_TASK, &task.id)?;
        Ok(task)
    })
}''',
)
replace_once(
    path,
    '''    } else if current.status == "done" {
        crate::execution_relation::clear_completion_for_task(connection, user_id, &saved.id)?;
    }
    Ok(saved)
}''',
    '''    } else if current.status == "done" {
        crate::execution_relation::clear_completion_for_task(connection, user_id, &saved.id)?;
    }
    enqueue_saved(connection, EntityType::EXECUTION_TASK, &saved.id)?;
    Ok(saved)
}''',
)
replace_once(
    path,
    '''    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| ExecutionError::storage(error.to_string()))?;
    if !repository::soft_delete_task(&transaction, &user_id, id).map_err(ExecutionError::storage)? {
        return Err(ExecutionError::not_found("任务不存在"));
    }
    crate::execution_relation::clear_completion_for_task(&transaction, &user_id, id)?;
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
        .map_err(|error| ExecutionError::storage(error.to_string()))?;
    Ok(())''',
    '''    local_transaction(connection, |connection| {
        if !repository::soft_delete_task(connection, &user_id, id)
            .map_err(ExecutionError::storage)?
        {
            return Err(ExecutionError::not_found("任务不存在"));
        }
        crate::execution_relation::clear_completion_for_task(connection, &user_id, id)?;
        enqueue_removed(connection, EntityType::EXECUTION_TASK, id)?;
        Ok(())
    })''',
)

# Server keeps HTTP mapping only; domain now owns sync semantics.
path = "apps/desktop/src-tauri/src/server/execution.rs"
replace_once(path, 'use lifetrace_contracts::registry::EntityType;\nuse rusqlite::Connection;\n', '')
replace_once(path, 'use crate::sync::outbox::{enqueue_delete, enqueue_upsert, MutationOrigin};\n\n', '')
start = '''fn enqueue_record<T: Serialize>(
    connection: &Connection,
    entity_type: &str,
    record: &T,
) -> Result<(), String> {
    let value = serde_json::to_value(record).map_err(|error| error.to_string())?;
    enqueue_upsert(connection, entity_type, &value, None, MutationOrigin::Local)?;
    Ok(())
}

'''
replace_once(path, start, '')
for block in [
'''    if let Err(error) = enqueue_record(&transaction, EntityType::EXECUTION_PROJECT, &project) {
        return storage_error(error);
    }
''',
'''    if let Err(error) = enqueue_delete(
        &transaction,
        EntityType::EXECUTION_PROJECT,
        &id,
        None,
        MutationOrigin::Local,
    ) {
        return storage_error(error);
    }
''',
'''    if let Err(error) = enqueue_record(&transaction, EntityType::EXECUTION_TASK, &task) {
        return storage_error(error);
    }
''']:
    replace_all(path, block, '', minimum=1)


# ---------------------------------------------------------------------------
# Completion results + entity links.
# ---------------------------------------------------------------------------
path = "apps/desktop/src-tauri/src/execution_relation.rs"
replace_once(path, 'use rusqlite::{params, Connection};\n', 'use lifetrace_contracts::registry::EntityType;\nuse rusqlite::{params, Connection};\n')
replace_once(
    path,
    '''    execution::{ExecutionError, ExecutionErrorKind, ExecutionResult},
};''',
    '''    execution::{self, ExecutionError, ExecutionErrorKind, ExecutionResult},
    sync::outbox::{enqueue_execution_delete, enqueue_execution_upsert},
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

fn queue_upsert(connection: &Connection, entity_type: &str, entity_id: &str) -> ExecutionResult<()> {
    enqueue_execution_upsert(connection, entity_type, entity_id)
        .map(|_| ())
        .map_err(storage)
}

fn queue_delete(connection: &Connection, entity_type: &str, entity_id: &str) -> ExecutionResult<()> {
    enqueue_execution_delete(connection, entity_type, entity_id)
        .map(|_| ())
        .map_err(storage)
}

fn save_completion_local(
    connection: &Connection,
    input: &CompletionResultWrite,
) -> ExecutionResult<CompletionResultRecord> {
    let result = repository::save_completion_result(connection, input).map_err(storage)?;
    queue_upsert(connection, EntityType::EXECUTION_COMPLETION_RESULT, &result.id)?;
    Ok(result)
}

fn save_link_local(
    connection: &Connection,
    input: &EntityLinkWrite,
) -> ExecutionResult<EntityLinkRecord> {
    let link = repository::save_link(connection, input).map_err(storage)?;
    queue_upsert(connection, EntityType::EXECUTION_ENTITY_LINK, &link.id)?;
    Ok(link)
}
''',
)
replace_once(
    path,
    '''    repository::save_completion_result(
        connection,
        &CompletionResultWrite {
            user_id,
            task_id: task.id,
            summary: clean_optional(input.summary, "完成总结", 20_000)?,
            completed_at,
            actual_minutes: input.actual_minutes.or(task.actual_minutes),
        },
    )
    .map_err(storage)''',
    '''    execution::local_transaction(connection, |connection| {
        save_completion_local(
            connection,
            &CompletionResultWrite {
                user_id,
                task_id: task.id,
                summary: clean_optional(input.summary, "完成总结", 20_000)?,
                completed_at,
                actual_minutes: input.actual_minutes.or(task.actual_minutes),
            },
        )
    })''',
)
replace_once(
    path,
    '''    repository::save_completion_result(
        connection,
        &CompletionResultWrite {
            user_id: user_id.to_owned(),
            task_id: task_id.to_owned(),
            summary,
            completed_at: completed_at.to_owned(),
            actual_minutes,
        },
    )
    .map_err(storage)''',
    '''    save_completion_local(
        connection,
        &CompletionResultWrite {
            user_id: user_id.to_owned(),
            task_id: task_id.to_owned(),
            summary,
            completed_at: completed_at.to_owned(),
            actual_minutes,
        },
    )''',
)
replace_once(
    path,
    '''    repository::soft_delete_completion_result(connection, user_id, task_id)
        .map(|_| ())
        .map_err(storage)''',
    '''    if let Some(existing) = repository::get_completion_result(connection, user_id, task_id)
        .map_err(storage)?
    {
        if repository::soft_delete_completion_result(connection, user_id, task_id).map_err(storage)? {
            queue_delete(
                connection,
                EntityType::EXECUTION_COMPLETION_RESULT,
                &existing.id,
            )?;
        }
    }
    Ok(())''',
)
replace_once(
    path,
    '''    repository::save_link(
        connection,
        &EntityLinkWrite {
            user_id,
            source_type,
            source_id,
            relation_type: validate_relation_type(&input.relation_type)?,
            target_type,
            target_id,
        },
    )
    .map_err(storage)''',
    '''    execution::local_transaction(connection, |connection| {
        save_link_local(
            connection,
            &EntityLinkWrite {
                user_id,
                source_type,
                source_id,
                relation_type: validate_relation_type(&input.relation_type)?,
                target_type,
                target_id,
            },
        )
    })''',
)
replace_once(
    path,
    '''    if repository::soft_delete_link(connection, &user_id, link_id).map_err(storage)? {
        Ok(())
    } else {
        Err(not_found("关联不存在"))
    }''',
    '''    execution::local_transaction(connection, |connection| {
        if repository::soft_delete_link(connection, &user_id, link_id).map_err(storage)? {
            queue_delete(connection, EntityType::EXECUTION_ENTITY_LINK, link_id)?;
            Ok(())
        } else {
            Err(not_found("关联不存在"))
        }
    })''',
)


# ---------------------------------------------------------------------------
# Waiting: all saves/deletes, including conversions, queue exact persisted rows.
# ---------------------------------------------------------------------------
path = "apps/desktop/src-tauri/src/execution_waiting.rs"
replace_once(path, 'use rusqlite::Connection;\n', 'use lifetrace_contracts::registry::EntityType;\nuse rusqlite::Connection;\n')
replace_once(
    path,
    '''    execution::{
        self, ExecutionError, ExecutionErrorKind, ExecutionResult, TaskInput, TaskStatusInput,
    },
};''',
    '''    execution::{
        self, ExecutionError, ExecutionErrorKind, ExecutionResult, TaskInput, TaskStatusInput,
    },
    execution_relation::{self, EntityLinkInput},
    sync::outbox::{enqueue_execution_delete, enqueue_execution_upsert},
};''',
)
# Replace repository calls before helper insertion.
replace_all(path, 'repository::save_waiting_item(', 'save_waiting_local(', minimum=5)
replace_all(path, 'repository::soft_delete_waiting_item(', 'delete_waiting_local(', minimum=1)
replace_once(
    path,
    '''fn active_user(connection: &Connection) -> ExecutionResult<String> {
    profile::active_profile_id(connection).map_err(storage)
}
''',
    '''fn active_user(connection: &Connection) -> ExecutionResult<String> {
    profile::active_profile_id(connection).map_err(storage)
}

fn save_waiting_local(
    connection: &Connection,
    write: &WaitingItemWrite,
) -> Result<WaitingItemRecord, String> {
    let operation = |connection: &Connection| -> Result<WaitingItemRecord, String> {
        let item = repository::save_waiting_item(connection, write)?;
        enqueue_execution_upsert(connection, EntityType::EXECUTION_WAITING_ITEM, &item.id)?;
        Ok(item)
    };
    if !connection.is_autocommit() {
        return operation(connection);
    }
    let transaction = connection.unchecked_transaction().map_err(|error| error.to_string())?;
    let item = operation(&transaction)?;
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(item)
}

fn delete_waiting_local(
    connection: &Connection,
    user_id: &str,
    id: &str,
) -> Result<bool, String> {
    let operation = |connection: &Connection| -> Result<bool, String> {
        let deleted = repository::soft_delete_waiting_item(connection, user_id, id)?;
        if deleted {
            enqueue_execution_delete(connection, EntityType::EXECUTION_WAITING_ITEM, id)?;
        }
        Ok(deleted)
    };
    if !connection.is_autocommit() {
        return operation(connection);
    }
    let transaction = connection.unchecked_transaction().map_err(|error| error.to_string())?;
    let deleted = operation(&transaction)?;
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(deleted)
}
''',
)
replace_once(
    path,
    '''    repository::create_conversion_links(&transaction, &user_id, waiting_item_id, &task.id)
        .map_err(storage)?;''',
    '''    execution_relation::create_link(
        &transaction,
        EntityLinkInput {
            source_type: "waiting_item".to_owned(),
            source_id: waiting_item_id.to_owned(),
            relation_type: "converted_to".to_owned(),
            target_type: "task".to_owned(),
            target_id: task.id.clone(),
        },
    )?;
    execution_relation::create_link(
        &transaction,
        EntityLinkInput {
            source_type: "task".to_owned(),
            source_id: task.id.clone(),
            relation_type: "derived_from".to_owned(),
            target_type: "waiting_item".to_owned(),
            target_id: waiting_item_id.to_owned(),
        },
    )?;''',
)


# ---------------------------------------------------------------------------
# Reminders: all lifecycle writes are atomic local+Outbox.
# ---------------------------------------------------------------------------
path = "apps/desktop/src-tauri/src/execution_reminder.rs"
replace_once(path, 'use rusqlite::{params, Connection};\n', 'use lifetrace_contracts::registry::EntityType;\nuse rusqlite::{params, Connection};\n')
replace_once(
    path,
    '''    execution::{ExecutionError, ExecutionErrorKind, ExecutionResult},
};''',
    '''    execution::{ExecutionError, ExecutionErrorKind, ExecutionResult},
    sync::outbox::{enqueue_execution_delete, enqueue_execution_upsert},
};''',
)
replace_all(path, 'repository::save(', 'save_reminder_local(', minimum=3)
replace_all(path, 'repository::soft_delete(', 'delete_reminder_local(', minimum=1)
replace_once(
    path,
    '''fn active_user(connection: &Connection) -> ExecutionResult<String> {
    profile::active_profile_id(connection).map_err(storage)
}
''',
    '''fn active_user(connection: &Connection) -> ExecutionResult<String> {
    profile::active_profile_id(connection).map_err(storage)
}

fn save_reminder_local(
    connection: &Connection,
    write: &ReminderWrite,
) -> Result<ReminderRecord, String> {
    let operation = |connection: &Connection| -> Result<ReminderRecord, String> {
        let reminder = repository::save(connection, write)?;
        enqueue_execution_upsert(connection, EntityType::EXECUTION_REMINDER, &reminder.id)?;
        Ok(reminder)
    };
    if !connection.is_autocommit() {
        return operation(connection);
    }
    let transaction = connection.unchecked_transaction().map_err(|error| error.to_string())?;
    let reminder = operation(&transaction)?;
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(reminder)
}

fn delete_reminder_local(
    connection: &Connection,
    user_id: &str,
    id: &str,
) -> Result<bool, String> {
    let operation = |connection: &Connection| -> Result<bool, String> {
        let deleted = repository::soft_delete(connection, user_id, id)?;
        if deleted {
            enqueue_execution_delete(connection, EntityType::EXECUTION_REMINDER, id)?;
        }
        Ok(deleted)
    };
    if !connection.is_autocommit() {
        return operation(connection);
    }
    let transaction = connection.unchecked_transaction().map_err(|error| error.to_string())?;
    let deleted = operation(&transaction)?;
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(deleted)
}
''',
)


# ---------------------------------------------------------------------------
# Memo + tags + tag-relations + conversion links.
# ---------------------------------------------------------------------------
path = "apps/desktop/src-tauri/src/execution_memo.rs"
replace_once(path, 'use chrono::Utc;\n', 'use std::collections::HashSet;\n\nuse chrono::Utc;\nuse lifetrace_contracts::registry::EntityType;\n')
replace_once(
    path,
    '''    execution_calendar::{self, CalendarEventInput, CalendarTimingInput},
    execution_waiting::{self, WaitingItemInput},
};''',
    '''    execution_calendar::{self, CalendarEventInput, CalendarTimingInput},
    execution_relation::{self, EntityLinkInput},
    execution_waiting::{self, WaitingItemInput},
    sync::outbox::{enqueue_execution_delete, enqueue_execution_upsert},
};''',
)
replace_all(path, 'repository::save(', 'save_memo_local(', minimum=5)
replace_all(path, 'repository::soft_delete(', 'delete_memo_local(', minimum=1)
replace_once(
    path,
    '''fn active_user(connection: &Connection) -> ExecutionResult<String> {
    profile::active_profile_id(connection).map_err(storage)
}
''',
    '''fn active_user(connection: &Connection) -> ExecutionResult<String> {
    profile::active_profile_id(connection).map_err(storage)
}

fn memo_tag_ids(connection: &Connection, memo_id: &str) -> Result<HashSet<String>, String> {
    let mut statement = connection
        .prepare("SELECT tag_id FROM execution_memo_tag_relations WHERE memo_id=?1")
        .map_err(|error| error.to_string())?;
    statement
        .query_map([memo_id], |row| row.get::<_, String>(0))
        .map_err(|error| error.to_string())?
        .collect::<rusqlite::Result<HashSet<_>>>()
        .map_err(|error| error.to_string())
}

fn sync_memo_graph(
    connection: &Connection,
    memo_id: &str,
    previous_tags: &HashSet<String>,
) -> Result<(), String> {
    enqueue_execution_upsert(connection, EntityType::EXECUTION_MEMO, memo_id)?;
    let current_tags = memo_tag_ids(connection, memo_id)?;
    for tag_id in &current_tags {
        enqueue_execution_upsert(connection, EntityType::EXECUTION_MEMO_TAG, tag_id)?;
        enqueue_execution_upsert(
            connection,
            EntityType::EXECUTION_MEMO_TAG_RELATION,
            &format!("{memo_id}:{tag_id}"),
        )?;
    }
    for tag_id in previous_tags.difference(&current_tags) {
        enqueue_execution_delete(
            connection,
            EntityType::EXECUTION_MEMO_TAG_RELATION,
            &format!("{memo_id}:{tag_id}"),
        )?;
    }
    Ok(())
}

fn save_memo_local(connection: &Connection, write: &MemoWrite) -> Result<MemoRecord, String> {
    let operation = |connection: &Connection| -> Result<MemoRecord, String> {
        let previous_tags = match write.id.as_deref() {
            Some(id) => memo_tag_ids(connection, id)?,
            None => HashSet::new(),
        };
        let memo = repository::save(connection, write)?;
        sync_memo_graph(connection, &memo.id, &previous_tags)?;
        Ok(memo)
    };
    if !connection.is_autocommit() {
        return operation(connection);
    }
    let transaction = connection.unchecked_transaction().map_err(|error| error.to_string())?;
    let memo = operation(&transaction)?;
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(memo)
}

fn delete_memo_local(connection: &Connection, user_id: &str, id: &str) -> Result<bool, String> {
    let operation = |connection: &Connection| -> Result<bool, String> {
        let tag_ids = memo_tag_ids(connection, id)?;
        let deleted = repository::soft_delete(connection, user_id, id)?;
        if deleted {
            enqueue_execution_delete(connection, EntityType::EXECUTION_MEMO, id)?;
            for tag_id in tag_ids {
                enqueue_execution_delete(
                    connection,
                    EntityType::EXECUTION_MEMO_TAG_RELATION,
                    &format!("{id}:{tag_id}"),
                )?;
            }
        }
        Ok(deleted)
    };
    if !connection.is_autocommit() {
        return operation(connection);
    }
    let transaction = connection.unchecked_transaction().map_err(|error| error.to_string())?;
    let deleted = operation(&transaction)?;
    transaction.commit().map_err(|error| error.to_string())?;
    Ok(deleted)
}
''',
)
# Replace conversion-link repository calls with sync-aware relation service.
replace_all(
    path,
    '''    repository::create_conversion_links(&transaction, &user_id, memo_id, "task", &target.id)
        .map_err(storage)?;''',
    '''    execution_relation::create_link(
        &transaction,
        EntityLinkInput {
            source_type: "memo".to_owned(),
            source_id: memo_id.to_owned(),
            relation_type: "converted_to".to_owned(),
            target_type: "task".to_owned(),
            target_id: target.id.clone(),
        },
    )?;
    execution_relation::create_link(
        &transaction,
        EntityLinkInput {
            source_type: "task".to_owned(),
            source_id: target.id.clone(),
            relation_type: "derived_from".to_owned(),
            target_type: "memo".to_owned(),
            target_id: memo_id.to_owned(),
        },
    )?;''',
)
replace_all(
    path,
    '''    repository::create_conversion_links(
        &transaction,
        &user_id,
        memo_id,
        "calendar_event",
        &target.id,
    )
    .map_err(storage)?;''',
    '''    execution_relation::create_link(
        &transaction,
        EntityLinkInput {
            source_type: "memo".to_owned(),
            source_id: memo_id.to_owned(),
            relation_type: "converted_to".to_owned(),
            target_type: "calendar_event".to_owned(),
            target_id: target.id.clone(),
        },
    )?;
    execution_relation::create_link(
        &transaction,
        EntityLinkInput {
            source_type: "calendar_event".to_owned(),
            source_id: target.id.clone(),
            relation_type: "derived_from".to_owned(),
            target_type: "memo".to_owned(),
            target_id: memo_id.to_owned(),
        },
    )?;''',
)
replace_all(
    path,
    '''    repository::create_conversion_links(
        &transaction,
        &user_id,
        memo_id,
        "waiting_item",
        &target.id,
    )
    .map_err(storage)?;''',
    '''    execution_relation::create_link(
        &transaction,
        EntityLinkInput {
            source_type: "memo".to_owned(),
            source_id: memo_id.to_owned(),
            relation_type: "converted_to".to_owned(),
            target_type: "waiting_item".to_owned(),
            target_id: target.id.clone(),
        },
    )?;
    execution_relation::create_link(
        &transaction,
        EntityLinkInput {
            source_type: "waiting_item".to_owned(),
            source_id: target.id.clone(),
            relation_type: "derived_from".to_owned(),
            target_type: "memo".to_owned(),
            target_id: memo_id.to_owned(),
        },
    )?;''',
)


# ---------------------------------------------------------------------------
# Task structure: subtasks, dependencies, recurrence, occurrences.
# ---------------------------------------------------------------------------
path = "apps/desktop/src-tauri/src/execution_structure.rs"
replace_once(path, 'use chrono::{DateTime, NaiveDate, Utc};\n', 'use chrono::{DateTime, NaiveDate, Utc};\nuse lifetrace_contracts::registry::EntityType;\n')
replace_once(
    path,
    '''    execution::{self, ExecutionError, ExecutionErrorKind, ExecutionResult, TaskInput},
};''',
    '''    execution::{self, ExecutionError, ExecutionErrorKind, ExecutionResult, TaskInput},
    sync::outbox::{enqueue_execution_delete, enqueue_execution_upsert},
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

fn queue_upsert(connection: &Connection, entity_type: &str, entity_id: &str) -> ExecutionResult<()> {
    enqueue_execution_upsert(connection, entity_type, entity_id)
        .map(|_| ())
        .map_err(storage)
}

fn queue_delete(connection: &Connection, entity_type: &str, entity_id: &str) -> ExecutionResult<()> {
    enqueue_execution_delete(connection, entity_type, entity_id)
        .map(|_| ())
        .map_err(storage)
}
''',
)
replace_once(
    path,
    '''    transaction
        .commit()
        .map_err(|error| storage(error.to_string()))?;
    execution::get_task(connection, &child.id)
}''',
    '''    let child = execution::get_task(&transaction, &child.id)?;
    queue_upsert(&transaction, EntityType::EXECUTION_TASK, &child.id)?;
    transaction
        .commit()
        .map_err(|error| storage(error.to_string()))?;
    Ok(child)
}''',
)
replace_once(
    path,
    '''    structure_repository::create_dependency(connection, &user_id, task_id, &prerequisite_id)
        .map_err(storage)
}''',
    '''    execution::local_transaction(connection, |connection| {
        let dependency = structure_repository::create_dependency(
            connection,
            &user_id,
            task_id,
            &prerequisite_id,
        )
        .map_err(storage)?;
        queue_upsert(
            connection,
            EntityType::EXECUTION_DEPENDENCY,
            &dependency.id,
        )?;
        Ok(dependency)
    })
}''',
)
replace_once(
    path,
    '''    if structure_repository::remove_dependency(connection, &user_id, task_id, prerequisite_id)
        .map_err(storage)?
    {
        Ok(())
    } else {
        Err(not_found("前置依赖不存在"))
    }
}''',
    '''    execution::local_transaction(connection, |connection| {
        let dependency = structure_repository::list_dependencies(connection, &user_id, task_id)
            .map_err(storage)?
            .into_iter()
            .find(|item| item.depends_on_task_id == prerequisite_id)
            .ok_or_else(|| not_found("前置依赖不存在"))?;
        if structure_repository::remove_dependency(connection, &user_id, task_id, prerequisite_id)
            .map_err(storage)?
        {
            queue_delete(
                connection,
                EntityType::EXECUTION_DEPENDENCY,
                &dependency.id,
            )?;
            Ok(())
        } else {
            Err(not_found("前置依赖不存在"))
        }
    })
}''',
)
replace_once(
    path,
    '''    transaction
        .commit()
        .map_err(|error| storage(error.to_string()))?;
    Ok(rule)
}''',
    '''    queue_upsert(
        &transaction,
        EntityType::EXECUTION_RECURRENCE_RULE,
        &rule.id,
    )?;
    queue_upsert(&transaction, EntityType::EXECUTION_TASK, task_id)?;
    transaction
        .commit()
        .map_err(|error| storage(error.to_string()))?;
    Ok(rule)
}''',
)
replace_once(
    path,
    '''    structure_repository::soft_delete_recurrence_rule(&transaction, &user_id, &rule_id)
        .map_err(storage)?;
    transaction
        .commit()''',
    '''    structure_repository::soft_delete_recurrence_rule(&transaction, &user_id, &rule_id)
        .map_err(storage)?;
    queue_delete(
        &transaction,
        EntityType::EXECUTION_RECURRENCE_RULE,
        &rule_id,
    )?;
    queue_upsert(&transaction, EntityType::EXECUTION_TASK, task_id)?;
    transaction
        .commit()''',
)
replace_once(
    path,
    '''    structure_repository::create_occurrence(connection, &user_id, &write).map_err(storage)
}''',
    '''    execution::local_transaction(connection, |connection| {
        let occurrence =
            structure_repository::create_occurrence(connection, &user_id, &write).map_err(storage)?;
        queue_upsert(
            connection,
            EntityType::EXECUTION_TASK_OCCURRENCE,
            &occurrence.id,
        )?;
        Ok(occurrence)
    })
}''',
)
replace_all(
    path,
    '''    structure_repository::update_occurrence(connection, &user_id, occurrence_id, &write)
        .map_err(storage)
}''',
    '''    execution::local_transaction(connection, |connection| {
        let occurrence = structure_repository::update_occurrence(
            connection,
            &user_id,
            occurrence_id,
            &write,
        )
        .map_err(storage)?;
        queue_upsert(
            connection,
            EntityType::EXECUTION_TASK_OCCURRENCE,
            &occurrence.id,
        )?;
        Ok(occurrence)
    })
}''',
    minimum=2,
)


# Strengthen a few existing tests so CI proves auxiliary writes are queued.
replace_once(
    path,
    '''        assert_eq!(child.parent_task_id.as_deref(), Some(parent.id.as_str()));
        assert_eq!(list_subtasks(&connection, &parent.id).unwrap().len(), 1);''',
    '''        assert_eq!(child.parent_task_id.as_deref(), Some(parent.id.as_str()));
        assert_eq!(list_subtasks(&connection, &parent.id).unwrap().len(), 1);
        let pending: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sync_outbox WHERE entity_type=?1 AND entity_id=?2 AND operation='upsert' AND status='pending'",
                [EntityType::EXECUTION_TASK, child.id.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(pending, 1);''',
)

print("Execution sync ownership patch applied")
