//! Stock BeeCount attachment HTTP surface under the internal cutover prefix.

use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Multipart, Path, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use lifetrace_contracts::sync::v1::AppId;
use lifetrace_contracts::ErrorCode;

use crate::auth::AuthenticatedPrincipal;
use crate::beecount_attachments::{
    AttachmentBatchExistsRequest, AttachmentBatchExistsResponse, AttachmentUploadOut,
    BeeCountAttachmentService,
};
use crate::error::ApiError;
use crate::state::AppState;

const PREFIX: &str = "/api/v1/integrations/beecount/compat/attachments";
const MULTIPART_OVERHEAD_BYTES: usize = 1024 * 1024;

pub fn router(max_upload_bytes: usize) -> Router<AppState> {
    Router::<AppState>::new()
        .route(&format!("{PREFIX}/upload"), post(upload))
        .route(&format!("{PREFIX}/batch-exists"), post(batch_exists))
        .route(
            &format!("{PREFIX}/category-icons/upload"),
            post(upload_category_icon),
        )
        .route(&format!("{PREFIX}/{{file_id}}"), get(download))
        .layer(DefaultBodyLimit::max(
            max_upload_bytes.saturating_add(MULTIPART_OVERHEAD_BYTES),
        ))
}

#[derive(Default)]
struct UploadedFile {
    file_name: String,
    mime_type: Option<String>,
    content: Vec<u8>,
}

async fn upload(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    multipart: Multipart,
) -> Result<Json<AttachmentUploadOut>, ApiError> {
    authorize(&principal, true)?;
    let (ledger_id, file) = parse_multipart(
        multipart,
        true,
        state.config.beecount_attachment_max_upload_bytes,
    )
    .await?;
    BeeCountAttachmentService::new(state.pool.clone())
        .upload_transaction(
            &principal.user_id,
            principal.device_id.as_str(),
            ledger_id.as_deref().unwrap_or_default(),
            &file.file_name,
            file.mime_type.as_deref(),
            file.content,
        )
        .await
        .map(Json)
}

async fn upload_category_icon(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    multipart: Multipart,
) -> Result<Json<AttachmentUploadOut>, ApiError> {
    authorize(&principal, true)?;
    let (_, file) = parse_multipart(
        multipart,
        false,
        state.config.beecount_attachment_max_upload_bytes,
    )
    .await?;
    BeeCountAttachmentService::new(state.pool.clone())
        .upload_category_icon(
            &principal.user_id,
            principal.device_id.as_str(),
            &file.file_name,
            file.mime_type.as_deref(),
            file.content,
        )
        .await
        .map(Json)
}

async fn batch_exists(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(request): Json<AttachmentBatchExistsRequest>,
) -> Result<Json<AttachmentBatchExistsResponse>, ApiError> {
    authorize(&principal, true)?;
    BeeCountAttachmentService::new(state.pool.clone())
        .batch_exists(&principal.user_id, request)
        .await
        .map(Json)
}

async fn download(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(file_id): Path<String>,
) -> Result<Response, ApiError> {
    authorize(&principal, false)?;
    let file = BeeCountAttachmentService::new(state.pool.clone())
        .download(&principal.user_id, &file_id)
        .await?;
    let content_type = file
        .mime_type
        .as_deref()
        .and_then(|value| HeaderValue::from_str(value).ok())
        .unwrap_or_else(|| HeaderValue::from_static("application/octet-stream"));
    let content_disposition = HeaderValue::from_str(&content_disposition(&file.file_name))
        .unwrap_or_else(|_| HeaderValue::from_static("attachment; filename=\"attachment.bin\""));
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CONTENT_DISPOSITION, content_disposition)
        .header(header::CONTENT_LENGTH, file.content.len().to_string())
        .body(Body::from(file.content))
        .map_err(internal)
}

async fn parse_multipart(
    mut multipart: Multipart,
    ledger_required: bool,
    max_upload_bytes: usize,
) -> Result<(Option<String>, UploadedFile), ApiError> {
    let mut ledger_id = None;
    let mut uploaded_file = None;
    while let Some(field) = multipart.next_field().await.map_err(invalid)? {
        match field.name() {
            Some("ledger_id") if ledger_required => {
                let value = field.text().await.map_err(invalid)?;
                ledger_id = Some(value.trim().to_owned());
            }
            Some("file") => {
                let file_name = field.file_name().unwrap_or("attachment.bin").to_owned();
                let mime_type = field.content_type().map(str::to_owned);
                let content = field.bytes().await.map_err(invalid)?.to_vec();
                if content.is_empty() {
                    return Err(invalid("Attachment file is empty"));
                }
                if content.len() > max_upload_bytes {
                    return Err(ApiError::new(
                        ErrorCode::InvalidRequest,
                        "Attachment upload too large",
                        StatusCode::PAYLOAD_TOO_LARGE,
                    ));
                }
                uploaded_file = Some(UploadedFile {
                    file_name,
                    mime_type,
                    content,
                });
            }
            _ => {}
        }
    }
    if ledger_required && ledger_id.as_deref().is_none_or(str::is_empty) {
        return Err(invalid("ledger_id is required"));
    }
    let uploaded_file = uploaded_file.ok_or_else(|| invalid("file is required"))?;
    Ok((ledger_id, uploaded_file))
}

fn authorize(principal: &AuthenticatedPrincipal, write: bool) -> Result<(), ApiError> {
    if principal.app_id.as_str() != AppId::BEECOUNT {
        return Err(ApiError::new(
            ErrorCode::AuthInvalid,
            "BeeCount session required",
            StatusCode::UNAUTHORIZED,
        ));
    }
    principal.require_scope(if write { "files:write" } else { "files:read" })
}

fn content_disposition(file_name: &str) -> String {
    let ascii = file_name
        .chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() || matches!(value, '.' | '-' | '_') {
                value
            } else {
                '_'
            }
        })
        .collect::<String>();
    let encoded = file_name
        .as_bytes()
        .iter()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(*byte, b'.' | b'-' | b'_') {
                (*byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect::<String>();
    format!(
        "attachment; filename=\"{}\"; filename*=UTF-8''{}",
        if ascii.is_empty() {
            "attachment.bin"
        } else {
            &ascii
        },
        encoded
    )
}

fn invalid(error: impl std::fmt::Display) -> ApiError {
    ApiError::new(
        ErrorCode::InvalidRequest,
        error.to_string(),
        StatusCode::BAD_REQUEST,
    )
}

fn internal(error: impl std::fmt::Display) -> ApiError {
    ApiError::new(
        ErrorCode::InternalError,
        error.to_string(),
        StatusCode::INTERNAL_SERVER_ERROR,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_header_has_ascii_and_utf8_names() {
        let value = content_disposition("午餐 收据.jpg");
        assert!(value.contains("filename=\"_____.jpg\""));
        assert!(value.contains("filename*=UTF-8''%E5%8D%88"));
        assert!(!value.contains('\n'));
    }
}
