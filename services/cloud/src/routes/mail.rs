use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use lifetrace_contracts::ErrorCode;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::AuthenticatedPrincipal;
use crate::error::ApiError;
use crate::mail::domain::{MailAccountInput, MailListQuery, SendMailInput};
use crate::mail::{MailService, MailServiceError};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/api/v1/mail/accounts", get(list_accounts).post(create_account))
        .route("/api/v1/mail/accounts/{id}", get(get_account).delete(disconnect_account))
        .route("/api/v1/mail/accounts/{id}/test", axum::routing::post(test_account))
        .route("/api/v1/mail/accounts/{id}/sync", axum::routing::post(sync_account))
        .route("/api/v1/mail/accounts/{id}/folders", get(list_folders))
        .route("/api/v1/mail/accounts/{id}/send", axum::routing::post(send_mail))
        .route("/api/v1/mail/threads", get(list_threads))
        .route("/api/v1/mail/threads/{id}/messages", get(thread_messages))
        .route("/api/v1/mail/messages/{id}", get(get_message))
        .route("/api/v1/mail/messages/{id}/attachments", get(message_attachments))
        .route("/api/v1/mail/messages/{id}/read", axum::routing::post(set_read))
        .route("/api/v1/mail/messages/{id}/archive", axum::routing::post(archive_message))
}

fn service(state: &AppState) -> MailService {
    MailService::new(state.pool.clone(), state.database_enabled)
}

fn map_error(error: MailServiceError) -> ApiError {
    let (status, message) = match error {
        MailServiceError::DatabaseRequired => (StatusCode::SERVICE_UNAVAILABLE, "mail storage requires PostgreSQL"),
        MailServiceError::InvalidUser | MailServiceError::InvalidAccount => (StatusCode::BAD_REQUEST, "invalid mail request"),
        MailServiceError::AccountNotFound => (StatusCode::NOT_FOUND, "mail account not found"),
        MailServiceError::MessageNotFound => (StatusCode::NOT_FOUND, "mail message not found"),
        MailServiceError::ThreadNotFound => (StatusCode::NOT_FOUND, "mail thread not found"),
        MailServiceError::ArchiveUnavailable => (StatusCode::CONFLICT, "archive folder is unavailable"),
        MailServiceError::Credential => (StatusCode::SERVICE_UNAVAILABLE, "mail credential store is unavailable"),
        MailServiceError::Protocol => (StatusCode::BAD_GATEWAY, "mail provider operation failed"),
        MailServiceError::Database => (StatusCode::INTERNAL_SERVER_ERROR, "mail storage operation failed"),
        MailServiceError::Parse => (StatusCode::UNPROCESSABLE_ENTITY, "mail message could not be parsed"),
        MailServiceError::SendInProgress => (StatusCode::CONFLICT, "mail send is already in progress"),
    };
    let code = if status.is_server_error() {
        ErrorCode::TemporarilyUnavailable
    } else {
        ErrorCode::InvalidRequest
    };
    ApiError::new(code, message, status)
}

async fn list_accounts(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:read")?;
    let items = service(&state)
        .list_accounts(&principal.user_id)
        .await
        .map_err(map_error)?;
    Ok(Json(json!({ "items": items })))
}

async fn get_account(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:read")?;
    let item = service(&state)
        .list_accounts(&principal.user_id)
        .await
        .map_err(map_error)?
        .into_iter()
        .find(|account| account.id == id)
        .ok_or_else(|| map_error(MailServiceError::AccountNotFound))?;
    Ok(Json(json!(item)))
}

async fn create_account(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(input): Json<MailAccountInput>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    principal.require_scope("mail:write")?;
    let account = service(&state)
        .create_account(&principal.user_id, input)
        .await
        .map_err(map_error)?;
    Ok((StatusCode::CREATED, Json(json!(account))))
}

async fn test_account(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:write")?;
    let result = service(&state)
        .test_account(&principal.user_id, id)
        .await
        .map_err(map_error)?;
    Ok(Json(json!(result)))
}

async fn disconnect_account(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:write")?;
    service(&state)
        .disconnect_account(&principal.user_id, id)
        .await
        .map_err(map_error)?;
    Ok(Json(json!({ "ok": true })))
}

async fn sync_account(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:write")?;
    let persisted = service(&state)
        .sync_account(&principal.user_id, id)
        .await
        .map_err(map_error)?;
    Ok(Json(json!({ "ok": true, "persisted": persisted })))
}

async fn list_folders(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:read")?;
    let items = service(&state)
        .list_folders(&principal.user_id, id)
        .await
        .map_err(map_error)?;
    Ok(Json(json!({ "items": items })))
}

async fn list_threads(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Query(query): Query<MailListQuery>,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:read")?;
    let items = service(&state)
        .list_threads(&principal.user_id, query)
        .await
        .map_err(map_error)?;
    Ok(Json(json!({ "items": items })))
}

async fn thread_messages(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:read")?;
    let items = service(&state)
        .thread_messages(&principal.user_id, id)
        .await
        .map_err(map_error)?;
    Ok(Json(json!({ "items": items })))
}

async fn get_message(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:read")?;
    let item = service(&state)
        .message(&principal.user_id, id)
        .await
        .map_err(map_error)?;
    Ok(Json(json!(item)))
}

async fn message_attachments(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:read")?;
    let items = service(&state)
        .attachments(&principal.user_id, id)
        .await
        .map_err(map_error)?;
    Ok(Json(json!({ "items": items })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadInput {
    read: bool,
}

async fn set_read(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
    Json(input): Json<ReadInput>,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:write")?;
    service(&state)
        .set_message_read(&principal.user_id, id, input.read)
        .await
        .map_err(map_error)?;
    Ok(Json(json!({ "ok": true, "read": input.read })))
}

async fn archive_message(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:write")?;
    service(&state)
        .archive_message(&principal.user_id, id)
        .await
        .map_err(map_error)?;
    Ok(Json(json!({ "ok": true })))
}

async fn send_mail(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
    Json(input): Json<SendMailInput>,
) -> Result<Json<Value>, ApiError> {
    principal.require_scope("mail:write")?;
    let message_id = service(&state)
        .send(&principal.user_id, id, input)
        .await
        .map_err(map_error)?;
    Ok(Json(json!({ "ok": true, "messageId": message_id })))
}
