use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use crate::{
    execution::{ExecutionError, ExecutionErrorKind},
    execution_reminder::{
        self, DueReminderQuery, ReminderInput, ReminderSnoozeInput, ReminderUpdateInput,
        SubjectReminderQuery,
    },
};

use super::AppState;

#[derive(Serialize)]
struct OkResponse { ok: bool }

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorResponse { error: String, code: &'static str }

fn execution_error(error: ExecutionError) -> Response {
    let (status, code) = match error.kind {
        ExecutionErrorKind::Validation => (StatusCode::BAD_REQUEST, "EXECUTION_VALIDATION"),
        ExecutionErrorKind::NotFound => (StatusCode::NOT_FOUND, "EXECUTION_NOT_FOUND"),
        ExecutionErrorKind::Conflict => (StatusCode::CONFLICT, "EXECUTION_CONFLICT"),
        ExecutionErrorKind::Storage => (StatusCode::INTERNAL_SERVER_ERROR, "EXECUTION_STORAGE_FAILURE"),
    };
    (status, Json(ErrorResponse { error: error.message, code })).into_response()
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

macro_rules! with_db {
    ($state:expr, $body:expr) => {{
        let connection = match $state.database.lock() {
            Ok(value) => value,
            Err(_) => return lock_error(),
        };
        match $body(&connection) {
            Ok(value) => value,
            Err(error) => return execution_error(error),
        }
    }};
}

pub async fn list_subject(
    State(state): State<AppState>,
    Query(query): Query<SubjectReminderQuery>,
) -> Response {
    let items = with_db!(state, |db| execution_reminder::list_subject_reminders(db, query));
    Json(items).into_response()
}

pub async fn list_due(
    State(state): State<AppState>,
    Query(query): Query<DueReminderQuery>,
) -> Response {
    let items = with_db!(state, |db| execution_reminder::list_due_reminders(db, query));
    Json(items).into_response()
}

pub async fn get(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let item = with_db!(state, |db| execution_reminder::get_reminder(db, &id));
    Json(item).into_response()
}

pub async fn create(State(state): State<AppState>, Json(input): Json<ReminderInput>) -> Response {
    let item = with_db!(state, |db| execution_reminder::create_reminder(db, input));
    (StatusCode::CREATED, Json(item)).into_response()
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<ReminderUpdateInput>,
) -> Response {
    let item = with_db!(state, |db| execution_reminder::update_reminder(db, &id, input));
    Json(item).into_response()
}

pub async fn fire(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let item = with_db!(state, |db| execution_reminder::mark_fired(db, &id));
    Json(item).into_response()
}

pub async fn snooze(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<ReminderSnoozeInput>,
) -> Response {
    let item = with_db!(state, |db| execution_reminder::snooze_reminder(db, &id, input));
    Json(item).into_response()
}

pub async fn dismiss(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let item = with_db!(state, |db| execution_reminder::dismiss_reminder(db, &id));
    Json(item).into_response()
}

pub async fn cancel(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let item = with_db!(state, |db| execution_reminder::cancel_reminder(db, &id));
    Json(item).into_response()
}

pub async fn delete(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    with_db!(state, |db| execution_reminder::delete_reminder(db, &id));
    Json(OkResponse { ok: true }).into_response()
}
