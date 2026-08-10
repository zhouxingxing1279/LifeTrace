use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use lifetrace_contracts::ErrorCode;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::AuthenticatedPrincipal;
use crate::error::ApiError;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::<AppState>::new().route("/api/v1/mail/messages", get(list_messages))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MessageListQuery {
    account_id: Option<Uuid>,
    folder_id: Option<Uuid>,
    q: Option<String>,
    unread_only: Option<bool>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct MailMessageSummary {
    id: Uuid,
    account_id: Uuid,
    folder_id: Uuid,
    thread_id: Uuid,
    subject: String,
    from_json: Value,
    to_json: Value,
    sent_at: Option<DateTime<Utc>>,
    received_at: DateTime<Utc>,
    is_read: bool,
    is_archived: bool,
    snippet: Option<String>,
    has_attachments: bool,
}

async fn list_messages(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Query(query): Query<MessageListQuery>,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:read")?;
    if !state.database_enabled {
        return Err(ApiError::new(
            ErrorCode::TemporarilyUnavailable,
            "mail storage requires PostgreSQL",
            StatusCode::SERVICE_UNAVAILABLE,
        ));
    }

    let user_id = Uuid::parse_str(principal.user_id.as_str()).map_err(|_| {
        ApiError::new(
            ErrorCode::InvalidRequest,
            "invalid authenticated user id",
            StatusCode::BAD_REQUEST,
        )
    })?;
    let q = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let limit = query.limit.unwrap_or(200).clamp(1, 500);
    let offset = query.offset.unwrap_or(0).max(0);

    let mut items = sqlx::query_as::<_, MailMessageSummary>(
        r#"
        SELECT m.id,m.account_id,m.folder_id,m.thread_id,m.subject,m.from_json,m.to_json,
               m.sent_at,m.received_at,m.is_read,m.is_archived,m.snippet,m.has_attachments
        FROM mail_messages m
        JOIN mail_folders f ON f.id=m.folder_id
        WHERE m.user_id=$1
          AND ($2::uuid IS NULL OR m.account_id=$2)
          AND (($3::uuid IS NULL AND f.normalized_role='inbox') OR m.folder_id=$3)
          AND m.received_at >= now() - interval '30 days'
          AND ($3::uuid IS NOT NULL OR m.is_archived=FALSE)
          AND ($4::text IS NULL
               OR m.subject ILIKE '%' || $4 || '%'
               OR coalesce(m.snippet,'') ILIKE '%' || $4 || '%'
               OR m.from_json::text ILIKE '%' || $4 || '%')
          AND ($5::boolean IS NULL OR ($5=TRUE AND m.is_read=FALSE) OR $5=FALSE)
        ORDER BY m.received_at DESC
        LIMIT $6 OFFSET $7
        "#,
    )
    .bind(user_id)
    .bind(query.account_id)
    .bind(query.folder_id)
    .bind(q)
    .bind(query.unread_only)
    .bind(limit + 1)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| {
        ApiError::new(
            ErrorCode::TemporarilyUnavailable,
            "mail storage operation failed",
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;

    let has_more = items.len() as i64 > limit;
    if has_more {
        items.truncate(limit as usize);
    }

    Ok(Json(json!({
        "items": items,
        "hasMore": has_more,
        "nextOffset": offset + items.len() as i64
    })))
}
