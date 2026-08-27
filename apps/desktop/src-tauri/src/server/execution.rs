use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use lifetrace_contracts::registry::EntityType;
use rusqlite::Connection;
use serde::Serialize;

use crate::execution::{
    self, ExecutionError, ExecutionErrorKind, ProjectInput, TaskInput, TaskQuery, TaskStatusInput,
};
use crate::sync::outbox::{enqueue_delete, enqueue_upsert, MutationOrigin};

use super::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OkResponse {
    ok: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorResponse {
    error: String,
    code: &'static str,
}

fn execution_error(error: ExecutionError) -> Response {
    let (status, code) = match error.kind {
        ExecutionErrorKind::Validation => (StatusCode::BAD_REQUEST, "EXECUTION_VALIDATION"),
        ExecutionErrorKind::NotFound => (StatusCode::NOT_FOUND, "EXECUTION_NOT_FOUND"),
        ExecutionErrorKind::Conflict => (StatusCode::CONFLICT, "EXECUTION_CONFLICT"),
        ExecutionErrorKind::Storage => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "EXECUTION_STORAGE_FAILURE",
        ),
    };
    (
        status,
        Json(ErrorResponse {
            error: error.message,
            code,
        }),
    )
        .into_response()
}

fn storage_error(message: impl Into<String>) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: message.into(),
            code: "EXECUTION_STORAGE_FAILURE",
        }),
    )
        .into_response()
}

fn lock_error() -> Response {
    storage_error("SQLite 锁已损坏")
}

fn enqueue_record<T: Serialize>(
    connection: &Connection,
    entity_type: &str,
    record: &T,
) -> Result<(), String> {
    let value = serde_json::to_value(record).map_err(|error| error.to_string())?;
    enqueue_upsert(
        connection,
        entity_type,
        &value,
        None,
        MutationOrigin::Local,
    )?;
    Ok(())
}

pub async fn list_projects(State(state): State<AppState>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution::list_projects(&connection) {
        Ok(projects) => Json(projects).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn get_project(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution::get_project(&connection, &id) {
        Ok(project) => Json(project).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn create_project(
    State(state): State<AppState>,
    Json(input): Json<ProjectInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    let transaction = match connection.unchecked_transaction() {
        Ok(value) => value,
        Err(error) => return storage_error(error.to_string()),
    };
    let project = match execution::create_project(&transaction, input) {
        Ok(value) => value,
        Err(error) => return execution_error(error),
    };
    if let Err(error) = enqueue_record(&transaction, EntityType::EXECUTION_PROJECT, &project) {
        return storage_error(error);
    }
    if let Err(error) = transaction.commit() {
        return storage_error(error.to_string());
    }
    (StatusCode::CREATED, Json(project)).into_response()
}

pub async fn update_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<ProjectInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    let transaction = match connection.unchecked_transaction() {
        Ok(value) => value,
        Err(error) => return storage_error(error.to_string()),
    };
    let project = match execution::update_project(&transaction, &id, input) {
        Ok(value) => value,
        Err(error) => return execution_error(error),
    };
    if let Err(error) = enqueue_record(&transaction, EntityType::EXECUTION_PROJECT, &project) {
        return storage_error(error);
    }
    if let Err(error) = transaction.commit() {
        return storage_error(error.to_string());
    }
    Json(project).into_response()
}

pub async fn delete_project(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    let transaction = match connection.unchecked_transaction() {
        Ok(value) => value,
        Err(error) => return storage_error(error.to_string()),
    };
    if let Err(error) = execution::delete_project(&transaction, &id) {
        return execution_error(error);
    }
    if let Err(error) = enqueue_delete(
        &transaction,
        EntityType::EXECUTION_PROJECT,
        &id,
        None,
        MutationOrigin::Local,
    ) {
        return storage_error(error);
    }
    if let Err(error) = transaction.commit() {
        return storage_error(error.to_string());
    }
    Json(OkResponse { ok: true }).into_response()
}

pub async fn list_tasks(State(state): State<AppState>, Query(query): Query<TaskQuery>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution::list_tasks(&connection, query) {
        Ok(tasks) => Json(tasks).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn get_task(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution::get_task(&connection, &id) {
        Ok(task) => Json(task).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn create_task(State(state): State<AppState>, Json(input): Json<TaskInput>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    let transaction = match connection.unchecked_transaction() {
        Ok(value) => value,
        Err(error) => return storage_error(error.to_string()),
    };
    let task = match execution::create_task(&transaction, input) {
        Ok(value) => value,
        Err(error) => return execution_error(error),
    };
    if let Err(error) = enqueue_record(&transaction, EntityType::EXECUTION_TASK, &task) {
        return storage_error(error);
    }
    if let Err(error) = transaction.commit() {
        return storage_error(error.to_string());
    }
    (StatusCode::CREATED, Json(task)).into_response()
}

pub async fn update_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<TaskInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    let transaction = match connection.unchecked_transaction() {
        Ok(value) => value,
        Err(error) => return storage_error(error.to_string()),
    };
    let task = match execution::update_task(&transaction, &id, input) {
        Ok(value) => value,
        Err(error) => return execution_error(error),
    };
    if let Err(error) = enqueue_record(&transaction, EntityType::EXECUTION_TASK, &task) {
        return storage_error(error);
    }
    if let Err(error) = transaction.commit() {
        return storage_error(error.to_string());
    }
    Json(task).into_response()
}

pub async fn change_task_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<TaskStatusInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    let transaction = match connection.unchecked_transaction() {
        Ok(value) => value,
        Err(error) => return storage_error(error.to_string()),
    };
    let task = match execution::change_task_status(&transaction, &id, input) {
        Ok(value) => value,
        Err(error) => return execution_error(error),
    };
    if let Err(error) = enqueue_record(&transaction, EntityType::EXECUTION_TASK, &task) {
        return storage_error(error);
    }
    if let Err(error) = transaction.commit() {
        return storage_error(error.to_string());
    }
    Json(task).into_response()
}

pub async fn delete_task(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    // `execution::delete_task` currently owns its transaction because it also
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
    Json(OkResponse { ok: true }).into_response()
}
