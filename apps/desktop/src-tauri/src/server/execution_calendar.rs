use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use crate::{
    execution::{ExecutionError, ExecutionErrorKind},
    execution_calendar::{
        self, CalendarConflictInput, CalendarEventInput, CalendarOccurrenceInput,
        CalendarOccurrenceStatusInput, CalendarQuery, CalendarTimingInput, ScheduleTaskInput,
    },
    execution_structure::RecurrenceRuleInput,
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

pub async fn list_events(
    State(state): State<AppState>,
    Query(query): Query<CalendarQuery>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_calendar::list_events(&connection, query) {
        Ok(events) => Json(events).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn get_event(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_calendar::get_event(&connection, &id) {
        Ok(event) => Json(event).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn create_event(
    State(state): State<AppState>,
    Json(input): Json<CalendarEventInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_calendar::create_event(&connection, input) {
        Ok(event) => (StatusCode::CREATED, Json(event)).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn update_event(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<CalendarEventInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_calendar::update_event(&connection, &id, input) {
        Ok(event) => Json(event).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn move_event(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<CalendarTimingInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_calendar::move_event(&connection, &id, input) {
        Ok(event) => Json(event).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn cancel_event(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_calendar::cancel_event(&connection, &id) {
        Ok(event) => Json(event).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn delete_event(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_calendar::delete_event(&connection, &id) {
        Ok(()) => Json(OkResponse { ok: true }).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn find_conflicts(
    State(state): State<AppState>,
    Json(input): Json<CalendarConflictInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_calendar::find_conflicts(&connection, input) {
        Ok(conflicts) => Json(conflicts).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn schedule_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Json(input): Json<ScheduleTaskInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_calendar::schedule_task(&connection, &task_id, input) {
        Ok(event) => (StatusCode::CREATED, Json(event)).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn get_recurrence(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_calendar::get_event_recurrence(&connection, &id) {
        Ok(rule) => Json(rule).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn set_recurrence(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<RecurrenceRuleInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_calendar::set_event_recurrence(&connection, &id, input) {
        Ok(rule) => Json(rule).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn clear_recurrence(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_calendar::clear_event_recurrence(&connection, &id) {
        Ok(()) => Json(OkResponse { ok: true }).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn list_occurrences(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_calendar::list_occurrences(&connection, &id) {
        Ok(items) => Json(items).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn materialize_occurrence(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<CalendarOccurrenceInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_calendar::materialize_occurrence(&connection, &id, input) {
        Ok(item) => (StatusCode::CREATED, Json(item)).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn update_occurrence(
    State(state): State<AppState>,
    Path((event_id, occurrence_id)): Path<(String, String)>,
    Json(input): Json<CalendarOccurrenceInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_calendar::update_occurrence(&connection, &event_id, &occurrence_id, input) {
        Ok(item) => Json(item).into_response(),
        Err(error) => execution_error(error),
    }
}

pub async fn change_occurrence_status(
    State(state): State<AppState>,
    Path((event_id, occurrence_id)): Path<(String, String)>,
    Json(input): Json<CalendarOccurrenceStatusInput>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return lock_error(),
    };
    match execution_calendar::change_occurrence_status(
        &connection,
        &event_id,
        &occurrence_id,
        input,
    ) {
        Ok(item) => Json(item).into_response(),
        Err(error) => execution_error(error),
    }
}
