use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{
    header::{CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE},
    HeaderValue, StatusCode,
};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use lifetrace_contracts::ErrorCode;
use uuid::Uuid;

use crate::auth::AuthenticatedPrincipal;
use crate::error::ApiError;
use crate::mail::attachment::{AttachmentReadError, AttachmentReader};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::<AppState>::new().route(
        "/api/v1/mail/attachments/{id}/content",
        get(download_attachment),
    )
}

fn map_error(error: AttachmentReadError) -> ApiError {
    let (status, message) = match error {
        AttachmentReadError::DatabaseRequired => (
            StatusCode::SERVICE_UNAVAILABLE,
            "mail storage requires PostgreSQL",
        ),
        AttachmentReadError::InvalidUser | AttachmentReadError::InvalidPart => {
            (StatusCode::BAD_REQUEST, "invalid attachment request")
        }
        AttachmentReadError::NotFound => (StatusCode::NOT_FOUND, "mail attachment not found"),
        AttachmentReadError::TooLarge => (
            StatusCode::PAYLOAD_TOO_LARGE,
            "mail attachment exceeds the download limit",
        ),
        AttachmentReadError::Credential => (
            StatusCode::SERVICE_UNAVAILABLE,
            "mail credential store is unavailable",
        ),
        AttachmentReadError::Protocol => {
            (StatusCode::BAD_GATEWAY, "mail provider operation failed")
        }
        AttachmentReadError::Parse => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "mail message could not be parsed",
        ),
        AttachmentReadError::Database => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "mail storage operation failed",
        ),
    };
    let code = if status.is_server_error() {
        ErrorCode::TemporarilyUnavailable
    } else {
        ErrorCode::InvalidRequest
    };
    ApiError::new(code, message, status)
}

async fn download_attachment(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Response<Body>, ApiError> {
    principal.require_scope("mail:read")?;
    let content = AttachmentReader::new(state.pool.clone(), state.database_enabled)
        .read(&principal.user_id, id)
        .await
        .map_err(map_error)?;
    let content_length = content.bytes.len();

    let mut response = Response::new(Body::from(content.bytes));
    let headers = response.headers_mut();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_str(&content.mime_type)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    headers.insert(CONTENT_DISPOSITION, HeaderValue::from_static("attachment"));
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
    headers.insert(
        CONTENT_LENGTH,
        HeaderValue::from_str(&content_length.to_string())
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    );
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        "x-lifetrace-attachment-name",
        HeaderValue::from_str(&content.filename)
            .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
    );
    Ok(response)
}
