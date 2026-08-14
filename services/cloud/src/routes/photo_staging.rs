//! Reusable transient cloud relay for moving photos into a LifeTrace desktop library.
//!
//! This is deliberately not part of the normal sync entity store: photo bytes are
//! temporary delivery payloads. The desktop downloads an item, commits it to the
//! local photo library, then DELETEs the cloud item as its acknowledgement.

use std::str::FromStr;

use axum::body::Body;
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::Response;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Duration, Utc};
use lifetrace_contracts::{ErrorCode, UserId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

use crate::auth::AuthenticatedPrincipal;
use crate::error::ApiError;
use crate::state::AppState;

pub const MAX_STAGED_PHOTO_BYTES: usize = 64 * 1024 * 1024;
const DEFAULT_TTL_HOURS: i64 = 72;
const MAX_LIST_LIMIT: i64 = 200;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StagedPhoto {
    pub id: String,
    pub source: String,
    pub client_asset_id: Option<String>,
    pub sha256: String,
    pub original_name: String,
    pub media_type: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub captured_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StagedPhotoList {
    items: Vec<StagedPhoto>,
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    limit: Option<i64>,
}

#[derive(Debug)]
pub(crate) struct StageInput {
    pub source: String,
    pub client_asset_id: Option<String>,
    pub original_name: String,
    pub media_type: String,
    pub mime_type: String,
    pub captured_at: Option<DateTime<Utc>>,
    pub content: Vec<u8>,
}

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/api/v1/photo-staging", get(list).post(upload))
        .route(
            "/api/v1/photo-staging/{id}",
            get(metadata).delete(acknowledge),
        )
        .route("/api/v1/photo-staging/{id}/content", get(content))
        .layer(DefaultBodyLimit::max(MAX_STAGED_PHOTO_BYTES + 1024 * 1024))
}

async fn list(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Query(query): Query<ListQuery>,
) -> Result<Json<StagedPhotoList>, ApiError> {
    principal.require_scope("files:read")?;
    cleanup_expired(&state).await?;
    let user_id = user_uuid(&principal.user_id)?;
    let limit = query.limit.unwrap_or(50).clamp(1, MAX_LIST_LIMIT);
    let rows = sqlx::query(
        "SELECT id,source,client_asset_id,sha256,original_name,media_type,mime_type,size_bytes,captured_at,created_at,expires_at \
         FROM photo_staging_items WHERE user_id=$1 AND expires_at>now() ORDER BY created_at ASC LIMIT $2",
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(database_error)?;
    Ok(Json(StagedPhotoList {
        items: rows
            .iter()
            .map(row_to_metadata)
            .collect::<Result<_, _>>()?,
    }))
}

async fn metadata(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<StagedPhoto>, ApiError> {
    principal.require_scope("files:read")?;
    let user_id = user_uuid(&principal.user_id)?;
    let row = sqlx::query(
        "SELECT id,source,client_asset_id,sha256,original_name,media_type,mime_type,size_bytes,captured_at,created_at,expires_at \
         FROM photo_staging_items WHERE id=$1 AND user_id=$2 AND expires_at>now()",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(database_error)?
    .ok_or_else(not_found)?;
    Ok(Json(row_to_metadata(&row)?))
}

async fn upload(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    multipart: Multipart,
) -> Result<(StatusCode, Json<StagedPhoto>), ApiError> {
    principal.require_scope("files:write")?;
    let input = read_upload(multipart).await?;
    let staged = stage_for_user(&state, &principal.user_id, input).await?;
    Ok((StatusCode::CREATED, Json(staged)))
}

async fn content(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    principal.require_scope("files:read")?;
    let user_id = user_uuid(&principal.user_id)?;
    let row = sqlx::query(
        "SELECT original_name,mime_type,sha256,content FROM photo_staging_items \
         WHERE id=$1 AND user_id=$2 AND expires_at>now()",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(database_error)?
    .ok_or_else(not_found)?;
    let original_name: String = row.try_get("original_name").map_err(database_error)?;
    let mime_type: String = row.try_get("mime_type").map_err(database_error)?;
    let sha256: String = row.try_get("sha256").map_err(database_error)?;
    let bytes: Vec<u8> = row.try_get("content").map_err(database_error)?;
    let safe_name = original_name.replace(['\r', '\n', '"'], "");
    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&mime_type)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{safe_name}\""))
            .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
    );
    response.headers_mut().insert(
        "x-content-sha256",
        HeaderValue::from_str(&sha256).map_err(|_| bad_request("暂存照片哈希无效"))?,
    );
    Ok(response)
}

async fn acknowledge(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    principal.require_scope("files:write")?;
    let user_id = user_uuid(&principal.user_id)?;
    let result = sqlx::query("DELETE FROM photo_staging_items WHERE id=$1 AND user_id=$2")
        .bind(id)
        .bind(user_id)
        .execute(&state.pool)
        .await
        .map_err(database_error)?;
    if result.rows_affected() == 0 {
        return Err(not_found());
    }
    Ok(Json(serde_json::json!({ "deleted": true, "id": id })))
}

pub(crate) async fn stage_for_user(
    state: &AppState,
    user_id: &UserId,
    mut input: StageInput,
) -> Result<StagedPhoto, ApiError> {
    if !state.database_enabled {
        return Err(ApiError::new(
            ErrorCode::TemporarilyUnavailable,
            "照片云端暂存需要 PostgreSQL",
            StatusCode::SERVICE_UNAVAILABLE,
        ));
    }
    validate_input(&mut input)?;
    cleanup_expired(state).await?;
    let owner = user_uuid(user_id)?;
    let sha256 = hex::encode(Sha256::digest(&input.content));
    let id = Uuid::new_v4();
    let ttl_hours = std::env::var("PHOTO_STAGING_TTL_HOURS")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(DEFAULT_TTL_HOURS)
        .clamp(1, 24 * 30);
    let expires_at = Utc::now() + Duration::hours(ttl_hours);

    if let Some(client_asset_id) = input.client_asset_id.as_deref() {
        if let Some(row) = sqlx::query(
            "SELECT id,source,client_asset_id,sha256,original_name,media_type,mime_type,size_bytes,captured_at,created_at,expires_at \
             FROM photo_staging_items WHERE user_id=$1 AND source=$2 AND client_asset_id=$3 AND expires_at>now()",
        )
        .bind(owner)
        .bind(&input.source)
        .bind(client_asset_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(database_error)?
        {
            return row_to_metadata(&row);
        }
    }

    let row = sqlx::query(
        "INSERT INTO photo_staging_items \
         (id,user_id,source,client_asset_id,sha256,original_name,media_type,mime_type,size_bytes,captured_at,content,expires_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) \
         RETURNING id,source,client_asset_id,sha256,original_name,media_type,mime_type,size_bytes,captured_at,created_at,expires_at",
    )
    .bind(id)
    .bind(owner)
    .bind(&input.source)
    .bind(&input.client_asset_id)
    .bind(&sha256)
    .bind(&input.original_name)
    .bind(&input.media_type)
    .bind(&input.mime_type)
    .bind(input.content.len() as i64)
    .bind(input.captured_at)
    .bind(&input.content)
    .bind(expires_at)
    .fetch_one(&state.pool)
    .await
    .map_err(database_error)?;
    row_to_metadata(&row)
}

async fn read_upload(mut multipart: Multipart) -> Result<StageInput, ApiError> {
    let mut source = "manual-upload".to_owned();
    let mut client_asset_id = None;
    let mut captured_at = None;
    let mut file_name = None;
    let mut mime_type = None;
    let mut content = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| bad_request("上传表单无效"))?
    {
        let name = field.name().unwrap_or_default().to_owned();
        match name.as_str() {
            "source" => source = field.text().await.map_err(|_| bad_request("source 无效"))?,
            "clientAssetId" => {
                let value = field
                    .text()
                    .await
                    .map_err(|_| bad_request("clientAssetId 无效"))?;
                client_asset_id = (!value.trim().is_empty())
                    .then(|| value.trim().chars().take(180).collect());
            }
            "capturedAt" => {
                let value = field
                    .text()
                    .await
                    .map_err(|_| bad_request("capturedAt 无效"))?;
                if !value.trim().is_empty() {
                    captured_at = Some(
                        DateTime::parse_from_rfc3339(value.trim())
                            .map_err(|_| bad_request("capturedAt 必须是 RFC3339 时间"))?
                            .with_timezone(&Utc),
                    );
                }
            }
            "file" => {
                let incoming_name = field.file_name().unwrap_or("photo").to_owned();
                let incoming_mime = field
                    .content_type()
                    .unwrap_or("application/octet-stream")
                    .to_owned();
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|_| bad_request("读取上传照片失败"))?;
                if bytes.len() > MAX_STAGED_PHOTO_BYTES {
                    return Err(bad_request("照片超过 64 MiB 暂存限制"));
                }
                file_name = Some(incoming_name);
                mime_type = Some(incoming_mime);
                content = Some(bytes.to_vec());
            }
            _ => {}
        }
    }
    let content = content.ok_or_else(|| bad_request("缺少 file 字段"))?;
    Ok(StageInput {
        source,
        client_asset_id,
        original_name: file_name.unwrap_or_else(|| "photo".to_owned()),
        media_type: if mime_type
            .as_deref()
            .unwrap_or_default()
            .starts_with("video/")
        {
            "video"
        } else {
            "image"
        }
        .to_owned(),
        mime_type: mime_type.unwrap_or_else(|| "application/octet-stream".to_owned()),
        captured_at,
        content,
    })
}

fn validate_input(input: &mut StageInput) -> Result<(), ApiError> {
    if input.content.is_empty() || input.content.len() > MAX_STAGED_PHOTO_BYTES {
        return Err(bad_request("照片为空或超过 64 MiB 暂存限制"));
    }
    input.source = clean_text(&input.source, 80, "unknown");
    input.original_name = clean_file_name(&input.original_name);
    input.media_type = match input.media_type.as_str() {
        "image" => "image".to_owned(),
        "video" => "video".to_owned(),
        _ => return Err(bad_request("mediaType 仅支持 image 或 video")),
    };
    if input.media_type == "image" && !input.mime_type.starts_with("image/") {
        return Err(bad_request("图片 MIME 类型无效"));
    }
    if input.media_type == "video" && !input.mime_type.starts_with("video/") {
        return Err(bad_request("视频 MIME 类型无效"));
    }
    input.mime_type = clean_text(&input.mime_type, 120, "application/octet-stream");
    Ok(())
}

fn row_to_metadata(row: &sqlx::postgres::PgRow) -> Result<StagedPhoto, ApiError> {
    Ok(StagedPhoto {
        id: row
            .try_get::<Uuid, _>("id")
            .map_err(database_error)?
            .to_string(),
        source: row.try_get("source").map_err(database_error)?,
        client_asset_id: row.try_get("client_asset_id").map_err(database_error)?,
        sha256: row.try_get("sha256").map_err(database_error)?,
        original_name: row.try_get("original_name").map_err(database_error)?,
        media_type: row.try_get("media_type").map_err(database_error)?,
        mime_type: row.try_get("mime_type").map_err(database_error)?,
        size_bytes: row.try_get("size_bytes").map_err(database_error)?,
        captured_at: row.try_get("captured_at").map_err(database_error)?,
        created_at: row.try_get("created_at").map_err(database_error)?,
        expires_at: row.try_get("expires_at").map_err(database_error)?,
    })
}

async fn cleanup_expired(state: &AppState) -> Result<(), ApiError> {
    if !state.database_enabled {
        return Ok(());
    }
    sqlx::query("DELETE FROM photo_staging_items WHERE expires_at<=now()")
        .execute(&state.pool)
        .await
        .map_err(database_error)?;
    Ok(())
}

fn user_uuid(user_id: &UserId) -> Result<Uuid, ApiError> {
    Uuid::parse_str(user_id.as_str()).map_err(|_| {
        ApiError::new(
            ErrorCode::InvalidRequest,
            "当前账号不能使用照片云端暂存",
            StatusCode::BAD_REQUEST,
        )
    })
}

fn clean_file_name(value: &str) -> String {
    let raw = std::path::Path::new(value)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("photo");
    clean_text(raw, 180, "photo")
}

fn clean_text(value: &str, max: usize, fallback: &str) -> String {
    let value: String = value
        .trim()
        .chars()
        .filter(|character| !character.is_control())
        .take(max)
        .collect();
    if value.is_empty() {
        fallback.to_owned()
    } else {
        value
    }
}

fn bad_request(message: impl Into<String>) -> ApiError {
    ApiError::new(ErrorCode::InvalidRequest, message, StatusCode::BAD_REQUEST)
}

fn not_found() -> ApiError {
    ApiError::new(
        ErrorCode::InvalidRequest,
        "暂存照片不存在或已过期",
        StatusCode::NOT_FOUND,
    )
}

fn database_error(error: sqlx::Error) -> ApiError {
    ApiError::new(
        ErrorCode::TemporarilyUnavailable,
        format!("照片暂存数据库操作失败: {error}"),
        StatusCode::SERVICE_UNAVAILABLE,
    )
}
