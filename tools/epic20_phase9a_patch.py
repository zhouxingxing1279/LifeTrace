from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing patch anchor: {label}")
    return text.replace(old, new, 1)

# Repository registration.
path = Path("apps/desktop/src-tauri/src/database/repositories/mod.rs")
text = path.read_text(encoding="utf-8")
if "pub mod execution_relation;" not in text:
    text = replace_once(text, "pub mod execution_reminder;\n", "pub mod execution_reminder;\npub mod execution_relation;\n", "repository module")
path.write_text(text, encoding="utf-8")

# Migration registration.
path = Path("apps/desktop/src-tauri/src/database/migrations/mod.rs")
text = path.read_text(encoding="utf-8")
if "mod m0011_execution_completion_backfill;" not in text:
    text = replace_once(text, "mod m0010_execution_sync;\n", "mod m0010_execution_sync;\nmod m0011_execution_completion_backfill;\n", "migration module")
if "pub use m0011_execution_completion_backfill::M0011ExecutionCompletionBackfill;" not in text:
    text = replace_once(text, "pub use m0010_execution_sync::M0010ExecutionSync;\n", "pub use m0010_execution_sync::M0010ExecutionSync;\npub use m0011_execution_completion_backfill::M0011ExecutionCompletionBackfill;\n", "migration export")
if "Box::new(M0011ExecutionCompletionBackfill)," not in text:
    text = replace_once(text, "        Box::new(M0010ExecutionSync),\n", "        Box::new(M0010ExecutionSync),\n        Box::new(M0011ExecutionCompletionBackfill),\n", "migration registry")
path.write_text(text, encoding="utf-8")

# Crate module registration.
path = Path("apps/desktop/src-tauri/src/lib.rs")
text = path.read_text(encoding="utf-8")
if "mod execution_relation;" not in text:
    text = replace_once(text, "mod execution_reminder;\n", "mod execution_reminder;\nmod execution_relation;\n", "domain module")
path.write_text(text, encoding="utf-8")

# Server module + routes.
path = Path("apps/desktop/src-tauri/src/server.rs")
text = path.read_text(encoding="utf-8")
if "mod execution_relation;" not in text:
    text = replace_once(text, "mod execution_reminder;\n", "mod execution_reminder;\nmod execution_relation;\n", "server module")
completion_route = '''        .route(
            "/api/execution/tasks/{id}/completion-result",
            get(execution_relation::get_completion).put(execution_relation::save_completion),
        )
'''
if "/completion-result" not in text:
    anchor = '''        .route(
            "/api/execution/tasks/{id}/status",
            axum::routing::put(execution::change_task_status),
        )
'''
    text = replace_once(text, anchor, anchor + completion_route, "completion route")
links_routes = '''        .route(
            "/api/execution/entity-links",
            get(execution_relation::list_links).post(execution_relation::create_link),
        )
        .route(
            "/api/execution/entity-links/{id}",
            axum::routing::delete(execution_relation::delete_link),
        )
'''
if '"/api/execution/entity-links"' not in text:
    anchor = '''        .route(
            "/api/execution/waiting-items/{id}/convert-to-task",
            axum::routing::post(execution_waiting::convert_waiting_to_task),
        )
'''
    text = replace_once(text, anchor, anchor + links_routes, "entity link routes")
path.write_text(text, encoding="utf-8")

# Make task completion/result state atomic.
path = Path("apps/desktop/src-tauri/src/execution.rs")
text = path.read_text(encoding="utf-8")
start = text.index("pub fn change_task_status(")
end = text.index("\npub fn delete_task(", start)
new_status = r'''pub fn change_task_status(
    connection: &Connection,
    id: &str,
    input: TaskStatusInput,
) -> ExecutionResult<TaskRecord> {
    validate_task_status(&input.status)?;
    let user_id = active_user(connection)?;
    let current = repository::get_task(connection, &user_id, id)
        .map_err(ExecutionError::storage)?
        .ok_or_else(|| ExecutionError::not_found("任务不存在"))?;
    if current.status == input.status {
        if current.status == "done"
            && crate::database::repositories::execution_relation::get_completion_result(
                connection,
                &user_id,
                id,
            )
            .map_err(ExecutionError::storage)?
            .is_none()
        {
            let completed_at = current
                .completed_at
                .as_deref()
                .ok_or_else(|| ExecutionError::validation("已完成任务缺少 completedAt"))?;
            crate::execution_relation::ensure_completion_for_task(
                connection,
                &user_id,
                id,
                completed_at,
                current.actual_minutes,
            )?;
        }
        return Ok(current);
    }
    if !transition_allowed(&current.status, &input.status) {
        return Err(ExecutionError::conflict(format!(
            "不允许从 {} 切换到 {}",
            current.status, input.status
        )));
    }
    let stamp = Utc::now().to_rfc3339();
    let write = TaskWrite {
        id: Some(current.id.clone()),
        user_id: user_id.clone(),
        project_id: current.project_id.clone(),
        parent_task_id: current.parent_task_id.clone(),
        title: current.title.clone(),
        description: current.description.clone(),
        status: input.status.clone(),
        priority: current.priority.clone(),
        estimated_minutes: current.estimated_minutes,
        actual_minutes: current.actual_minutes,
        due_at: current.due_at.clone(),
        scheduled_start_at: current.scheduled_start_at.clone(),
        scheduled_end_at: current.scheduled_end_at.clone(),
        timezone: current.timezone.clone(),
        context: current.context.clone(),
        completed_at: if input.status == "done" {
            Some(stamp.clone())
        } else if current.status == "done" {
            None
        } else {
            current.completed_at.clone()
        },
        cancelled_at: if input.status == "cancelled" {
            Some(stamp)
        } else {
            current.cancelled_at.clone()
        },
    };

    let transaction = connection
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
}
'''
text = text[:start] + new_status + text[end:]

# Keep completion result lifecycle aligned with task soft-delete.
old_delete_tail = '''    if repository::soft_delete_task(connection, &user_id, id).map_err(ExecutionError::storage)? {
        Ok(())
    } else {
        Err(ExecutionError::not_found("任务不存在"))
    }
}'''
new_delete_tail = '''    let transaction = connection
        .unchecked_transaction()
        .map_err(|error| ExecutionError::storage(error.to_string()))?;
    if !repository::soft_delete_task(&transaction, &user_id, id).map_err(ExecutionError::storage)? {
        return Err(ExecutionError::not_found("任务不存在"));
    }
    crate::execution_relation::clear_completion_for_task(&transaction, &user_id, id)?;
    transaction
        .commit()
        .map_err(|error| ExecutionError::storage(error.to_string()))?;
    Ok(())
}'''
text = replace_once(text, old_delete_tail, new_delete_tail, "task delete completion cleanup")

# Extend existing status lifecycle test with completion-result assertions.
old_assert = '''        assert_eq!(done.status, "done");
        assert!(done.completed_at.is_some());
        let reopened = change_task_status('''
new_assert = '''        assert_eq!(done.status, "done");
        assert!(done.completed_at.is_some());
        assert!(crate::execution_relation::get_completion_result(&connection, &task.id)
            .unwrap()
            .is_some());
        let reopened = change_task_status('''
text = replace_once(text, old_assert, new_assert, "completion created assertion")
old_reopen = '''        assert_eq!(reopened.status, "todo");
        assert!(reopened.completed_at.is_none());'''
new_reopen = '''        assert_eq!(reopened.status, "todo");
        assert!(reopened.completed_at.is_none());
        assert!(crate::execution_relation::get_completion_result(&connection, &task.id)
            .unwrap()
            .is_none());'''
text = replace_once(text, old_reopen, new_reopen, "completion cleared assertion")
path.write_text(text, encoding="utf-8")

# Use EPIC-19 instrumented fetch and expose completion/link APIs to the renderer.
path = Path("apps/desktop/src/services/executionApi.ts")
text = path.read_text(encoding="utf-8")
if 'clientObservability' not in text:
    text = 'import { instrumentedFetch } from "@/src/services/clientObservability";\n\n' + text
if 'export type CompletionResult' not in text:
    anchor = 'export type TaskBlocker = { taskId: string; title: string; status: string };\n'
    addition = r'''
export type CompletionResult = {
  id: string;
  userId: string;
  taskId: string;
  summary?: string | null;
  completedAt: string;
  actualMinutes?: number | null;
  version: number;
  createdAt: string;
  updatedAt: string;
};

export type EntityLink = {
  id: string;
  userId: string;
  sourceType: string;
  sourceId: string;
  relationType: "related_to" | "derived_from" | "converted_to" | "attachment" | "reference";
  targetType: string;
  targetId: string;
  version: number;
  createdAt: string;
  updatedAt: string;
};

export type EntityLinkInput = Pick<EntityLink, "sourceType" | "sourceId" | "relationType" | "targetType" | "targetId">;
'''
    text = replace_once(text, anchor, anchor + addition, "renderer completion/link types")
old_request = '''async function request<T>(url: string, init?: RequestInit): Promise<T> {
  const response = await fetch(url, init);
'''
new_request = '''async function request<T>(url: string, init?: RequestInit): Promise<T> {
  const method = (init?.method || "GET").toUpperCase();
  const response = await instrumentedFetch(globalThis.fetch, url, init, {
    module: "execution",
    action: `${method} ${url.split("?", 1)[0]}`,
    userMessage: "执行服务请求失败",
  });
'''
text = replace_once(text, old_request, new_request, "execution instrumented fetch")
if 'completion: (id: string)' not in text:
    anchor = '''    schedule: (id: string, timing: CalendarTimingInput) =>
      request<CalendarEvent>(`/api/execution/tasks/${encodeURIComponent(id)}/schedule`, json("POST", { timing })),
'''
    addition = '''    completion: (id: string) =>
      request<CompletionResult | null>(`/api/execution/tasks/${encodeURIComponent(id)}/completion-result`),
    saveCompletion: (id: string, input: { summary?: string | null; actualMinutes?: number | null }) =>
      request<CompletionResult>(`/api/execution/tasks/${encodeURIComponent(id)}/completion-result`, json("PUT", input)),
'''
    text = replace_once(text, anchor, anchor + addition, "task completion APIs")
if 'relations: {' not in text:
    anchor = '  reminders: {\n'
    relations = '''  relations: {
    list: (entityType: string, entityId: string) =>
      request<EntityLink[]>(query("/api/execution/entity-links", { entityType, entityId })),
    create: (input: EntityLinkInput) =>
      request<EntityLink>("/api/execution/entity-links", json("POST", input)),
    remove: (id: string) =>
      request<{ ok: true }>(`/api/execution/entity-links/${encodeURIComponent(id)}`, { method: "DELETE" }),
  },
'''
    text = replace_once(text, anchor, relations + anchor, "renderer relation APIs")
path.write_text(text, encoding="utf-8")
