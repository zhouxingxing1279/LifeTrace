use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::database::{profile, repositories::analytics as analytics_repo};

use super::AppState;

fn failure(status: StatusCode, message: impl Into<String>) -> Response {
    (status, Json(json!({ "error": message.into() }))).into_response()
}

fn storage_error(message: impl Into<String>) -> Response {
    failure(StatusCode::INTERNAL_SERVER_ERROR, message)
}

fn user_id(connection: &rusqlite::Connection) -> Result<String, String> {
    profile::active_profile_id(connection)
}

pub async fn status(State(state): State<AppState>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return storage_error("SQLite 锁已损坏"),
    };
    let user_id = match user_id(&connection) {
        Ok(value) => value,
        Err(message) => return storage_error(message),
    };
    match analytics_repo::projection_status(&connection, &user_id) {
        Ok(value) => Json(value).into_response(),
        Err(message) => storage_error(message),
    }
}

pub async fn rebuild(State(state): State<AppState>) -> Response {
    let mut connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return storage_error("SQLite 锁已损坏"),
    };
    let user_id = match user_id(&connection) {
        Ok(value) => value,
        Err(message) => return storage_error(message),
    };
    match analytics_repo::rebuild(&mut connection, &user_id) {
        Ok(value) => Json(value).into_response(),
        Err(message) => storage_error(message),
    }
}

pub async fn timeline(
    State(state): State<AppState>,
    Query(query): Query<analytics_repo::TimelineQuery>,
) -> Response {
    let mut connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return storage_error("SQLite 锁已损坏"),
    };
    let user_id = match user_id(&connection) {
        Ok(value) => value,
        Err(message) => return storage_error(message),
    };
    if let Err(message) = analytics_repo::ensure_current(&mut connection, &user_id) {
        return storage_error(message);
    }
    match analytics_repo::timeline(&connection, &user_id, &query) {
        Ok(value) => Json(value).into_response(),
        Err(message) => failure(StatusCode::BAD_REQUEST, message),
    }
}

pub async fn search(
    State(state): State<AppState>,
    Query(query): Query<analytics_repo::SearchQuery>,
) -> Response {
    let mut connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return storage_error("SQLite 锁已损坏"),
    };
    let user_id = match user_id(&connection) {
        Ok(value) => value,
        Err(message) => return storage_error(message),
    };
    if let Err(message) = analytics_repo::ensure_current(&mut connection, &user_id) {
        return storage_error(message);
    }
    match analytics_repo::search(&connection, &user_id, &query) {
        Ok(value) => Json(value).into_response(),
        Err(message) => failure(StatusCode::BAD_REQUEST, message),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportQuery {
    report_type: String,
    period_start: String,
    period_end: String,
    timezone: Option<String>,
}

pub async fn report(State(state): State<AppState>, Query(query): Query<ReportQuery>) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return storage_error("SQLite 锁已损坏"),
    };
    let user_id = match user_id(&connection) {
        Ok(value) => value,
        Err(message) => return storage_error(message),
    };
    let timezone = query.timezone.as_deref().unwrap_or("UTC");
    match analytics_repo::generate_report(
        &connection,
        &user_id,
        &query.report_type,
        &query.period_start,
        &query.period_end,
        timezone,
    ) {
        Ok(value) => Json(value).into_response(),
        Err(message) => failure(StatusCode::BAD_REQUEST, message),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsightQuery {
    period_start: String,
    period_end: String,
}

pub async fn insights(
    State(state): State<AppState>,
    Query(query): Query<InsightQuery>,
) -> Response {
    let connection = match state.database.lock() {
        Ok(value) => value,
        Err(_) => return storage_error("SQLite 锁已损坏"),
    };
    let user_id = match user_id(&connection) {
        Ok(value) => value,
        Err(message) => return storage_error(message),
    };
    match analytics_repo::generate_insights(
        &connection,
        &user_id,
        &query.period_start,
        &query.period_end,
    ) {
        Ok(value) => Json(value).into_response(),
        Err(message) => failure(StatusCode::BAD_REQUEST, message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_query_uses_camel_case_contract() {
        let query: ReportQuery = serde_json::from_value(json!({
            "reportType": "weekly",
            "periodStart": "2026-08-03",
            "periodEnd": "2026-08-09",
            "timezone": "Asia/Shanghai"
        }))
        .unwrap();
        assert_eq!(query.report_type, "weekly");
        assert_eq!(query.period_start, "2026-08-03");
        assert_eq!(query.period_end, "2026-08-09");
    }
}
