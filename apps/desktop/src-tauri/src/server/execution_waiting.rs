use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use crate::{
    execution::{ExecutionError, ExecutionErrorKind},
    execution_waiting::{
        self, ConvertWaitingToTaskInput, ResolveWaitingInput, TaskToWaitingInput, WaitingItemInput,
        WaitingQuery,
    },
};

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

fn lock_error() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "SQLite 锁已损坏".to_owned(),
            code: "EXECUTION_DATABASE_LOCK_FAILURE",
        }),
    )
        .into_response()
}

pub async fn list_waiting_items(
    State(state): State<AppState>,
    Query(query): Query<WaitingQuery>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_waiting::list_waiting_items(&connection, query) {
        Ok(items) => Json(items).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn get_waiting_item(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_waiting::get_waiting_item(&connection, &id) {
        Ok(item) => Json(item).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn create_waiting_item(
    State(state): State<AppState>,
    Json(input): Json<WaitingItemInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_waiting::create_waiting_item(&connection, input) {
        Ok(item) => (StatusCode::CREATED, Json(item)).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn update_waiting_item(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<WaitingItemInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_waiting::update_waiting_item(&connection, &id, input) {
        Ok(item) => Json(item).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn resolve_waiting_item(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<ResolveWaitingInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_waiting::resolve_waiting_item(&connection, &id, input) {
        Ok(item) => Json(item).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn cancel_waiting_item(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_waiting::cancel_waiting_item(&connection, &id) {
        Ok(item) => Json(item).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn delete_waiting_item(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_waiting::delete_waiting_item(&connection, &id) {
        Ok(()) => Json(OkResponse { ok: true }).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn create_waiting_from_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Json(input): Json<TaskToWaitingInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_waiting::create_waiting_from_task(&connection, &task_id, input) {
        Ok(item) => (StatusCode::CREATED, Json(item)).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn convert_waiting_to_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<ConvertWaitingToTaskInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_waiting::convert_waiting_to_task(&connection, &id, input) {
        Ok(task) => Json(task).into_response(),
        Err(error) => execution_error(error),
    }
}
