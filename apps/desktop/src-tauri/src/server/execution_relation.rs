use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use crate::{
    execution::{ExecutionError, ExecutionErrorKind},
    execution_relation::{self, CompletionResultInput, EntityLinkInput, EntityLinksQuery},
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

pub async fn get_completion(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_relation::get_completion_result(&connection, &task_id) {
        Ok(result) => Json(result).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn save_completion(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Json(input): Json<CompletionResultInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_relation::save_completion_result(&connection, &task_id, input) {
        Ok(result) => Json(result).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn list_links(
    State(state): State<AppState>,
    Query(query): Query<EntityLinksQuery>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_relation::list_links(&connection, query) {
        Ok(links) => Json(links).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn create_link(
    State(state): State<AppState>,
    Json(input): Json<EntityLinkInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_relation::create_link(&connection, input) {
        Ok(link) => (StatusCode::CREATED, Json(link)).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn delete_link(State(state): State<AppState>, Path(link_id): Path<String>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_relation::delete_link(&connection, &link_id) {
        Ok(()) => Json(OkResponse { ok: true }).into_response(),
        Err(error) => execution_error(error),
    }
}
