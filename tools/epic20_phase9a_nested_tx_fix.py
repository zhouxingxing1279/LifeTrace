from pathlib import Path

path = Path("apps/desktop/src-tauri/src/execution.rs")
text = path.read_text(encoding="utf-8")

helper_marker = "fn persist_task_status_change("
if helper_marker not in text:
    anchor = "pub fn change_task_status(\n"
    helper = r'''fn persist_task_status_change(
    connection: &Connection,
    user_id: &str,
    current: &TaskRecord,
    write: &TaskWrite,
) -> ExecutionResult<TaskRecord> {
    let saved = repository::save_task(connection, write).map_err(ExecutionError::storage)?;
    if saved.status == "done" {
        let completed_at = saved
            .completed_at
            .as_deref()
            .ok_or_else(|| ExecutionError::validation("已完成任务缺少 completedAt"))?;
        crate::execution_relation::ensure_completion_for_task(
            connection,
            user_id,
            &saved.id,
            completed_at,
            saved.actual_minutes,
        )?;
    } else if current.status == "done" {
        crate::execution_relation::clear_completion_for_task(connection, user_id, &saved.id)?;
    }
    Ok(saved)
}

'''
    if anchor not in text:
        raise SystemExit("missing task status function anchor")
    text = text.replace(anchor, helper + anchor, 1)

old = r'''    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| ExecutionError::storage(error.to_string()))?;
    let saved = repository::save_task(&transaction, &write).map_err(ExecutionError::storage)?;
    if saved.status == "done" {
        let completed_at = saved
            .completed_at
            .as_deref()
            .ok_or_else(|| ExecutionError::validation("已完成任务缺少 completedAt"))?;
        crate::execution_relation::ensure_completion_for_task(
            &transaction,
            &user_id,
            &saved.id,
            completed_at,
            saved.actual_minutes,
        )?;
    } else if current.status == "done" {
        crate::execution_relation::clear_completion_for_task(&transaction, &user_id, &saved.id)?;
    }
    transaction
        .commit()
        .map_err(|error| ExecutionError::storage(error.to_string()))?;
    repository::get_task(connection, &user_id, id)
        .map_err(ExecutionError::storage)?
        .ok_or_else(|| ExecutionError::not_found("任务不存在"))
}'''
new = r'''    if !connection.is_autocommit() {
        return persist_task_status_change(connection, &user_id, &current, &write);
    }

    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| ExecutionError::storage(error.to_string()))?;
    let saved = persist_task_status_change(&transaction, &user_id, &current, &write)?;
    transaction
        .commit()
        .map_err(|error| ExecutionError::storage(error.to_string()))?;
    Ok(saved)
}'''
if old not in text:
    if new not in text:
        raise SystemExit("missing phase9a transaction block")
else:
    text = text.replace(old, new, 1)

path.write_text(text, encoding="utf-8")
