use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use crate::{
    execution::{ExecutionError, ExecutionErrorKind, TaskInput},
    execution_structure::{
        self, DependencyInput, OccurrenceInput, OccurrenceStatusInput, OccurrenceUpdateInput,
        RecurrenceRuleInput,
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

pub async fn list_subtasks(State(state): State<AppState>, Path(task_id): Path<String>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_structure::list_subtasks(&connection, &task_id) {
        Ok(tasks) => Json(tasks).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn add_subtask(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Json(input): Json<TaskInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_structure::add_subtask(&connection, &task_id, input) {
        Ok(task) => (StatusCode::CREATED, Json(task)).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn list_dependencies(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_structure::list_dependencies(&connection, &task_id) {
        Ok(dependencies) => Json(dependencies).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn add_dependency(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Json(input): Json<DependencyInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_structure::add_dependency(&connection, &task_id, input) {
        Ok(dependency) => (StatusCode::CREATED, Json(dependency)).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn remove_dependency(
    State(state): State<AppState>,
    Path((task_id, prerequisite_id)): Path<(String, String)>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_structure::remove_dependency(&connection, &task_id, &prerequisite_id) {
        Ok(()) => Json(OkResponse { ok: true }).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn list_blockers(State(state): State<AppState>, Path(task_id): Path<String>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_structure::list_blockers(&connection, &task_id) {
        Ok(blockers) => Json(blockers).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn get_recurrence(State(state): State<AppState>, Path(task_id): Path<String>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_structure::get_task_recurrence(&connection, &task_id) {
        Ok(recurrence) => Json(recurrence).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn set_recurrence(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Json(input): Json<RecurrenceRuleInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_structure::set_task_recurrence(&connection, &task_id, input) {
        Ok(recurrence) => Json(recurrence).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn clear_recurrence(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_structure::clear_task_recurrence(&connection, &task_id) {
        Ok(()) => Json(OkResponse { ok: true }).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn list_occurrences(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_structure::list_occurrences(&connection, &task_id) {
        Ok(occurrences) => Json(occurrences).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn materialize_occurrence(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Json(input): Json<OccurrenceInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_structure::materialize_occurrence(&connection, &task_id, input) {
        Ok(occurrence) => (StatusCode::CREATED, Json(occurrence)).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn update_occurrence(
    State(state): State<AppState>,
    Path((task_id, occurrence_id)): Path<(String, String)>,
    Json(input): Json<OccurrenceUpdateInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_structure::update_occurrence(&connection, &task_id, &occurrence_id, input) {
        Ok(occurrence) => Json(occurrence).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn change_occurrence_status(
    State(state): State<AppState>,
    Path((task_id, occurrence_id)): Path<(String, String)>,
    Json(input): Json<OccurrenceStatusInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_structure::change_occurrence_status(
        &connection,
        &task_id,
        &occurrence_id,
        input,
    ) {
        Ok(occurrence) => Json(occurrence).into_response(),
        Err(error) => execution_error(error),
    }
}
