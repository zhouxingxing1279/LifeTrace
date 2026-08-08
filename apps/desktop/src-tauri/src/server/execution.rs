use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use crate::execution::{
    self, ExecutionError, ExecutionErrorKind, ProjectInput, TaskInput, TaskQuery, TaskStatusInput,
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
    match execution::create_project(&connection, input) {
        Ok(project) => (StatusCode::CREATED, Json(project)).into_response(),
        Err(error) => execution_error(error),
    }
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
    match execution::update_project(&connection, &id, input) {
        Ok(project) => Json(project).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn delete_project(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution::delete_project(&connection, &id) {
        Ok(()) => Json(OkResponse { ok: true }).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn list_tasks(
    State(state): State<AppState>,
    Query(query): Query<TaskQuery>,
) -> Response {
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
    match execution::create_task(&connection, input) {
        Ok(task) => (StatusCode::CREATED, Json(task)).into_response(),
        Err(error) => execution_error(error),
    }
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
    match execution::update_task(&connection, &id, input) {
        Ok(task) => Json(task).into_response(),
        Err(error) => execution_error(error),
    }
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
    match execution::change_task_status(&connection, &id, input) {
        Ok(task) => Json(task).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn delete_task(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution::delete_task(&connection, &id) {
        Ok(()) => Json(OkResponse { ok: true }).into_response(),
        Err(error) => execution_error(error),
    }
}
