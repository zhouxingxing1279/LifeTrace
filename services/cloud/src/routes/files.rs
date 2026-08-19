//! EPIC-12 unified file metadata and signed object-storage transfer API.

use axum::extract::{Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use lifetrace_contracts::{ErrorCode, UserId};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::auth::AuthenticatedPrincipal;
use crate::error::ApiError;
use crate::object_storage::{ObjectStorageConfig, PresignedRequest};
use crate::state::AppState;

const DEFAULT_MAX_FILE_BYTES: i64 = 256 * 1024 * 1024;
const MAX_LIST_LIMIT: i64 = 200;
const DOMAINS: &[&str] = &[
    "finance_imports",
    "notes_attachments",
    "english_audio",
    "photos",
    "workout_imports",
    "backups",
];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PrepareRequest {
    domain: String,
    original_name: String,
    mime_type: String,
    size_bytes: i64,
    sha256: String,
    entity_type: Option<String>,
    entity_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PrepareResponse {
    file: FileMetadata,
    deduplicated: bool,
    upload: Option<SignedTransfer>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SignedTransfer {
    url: String,
    required_headers: std::collections::BTreeMap<String, String>,
    expires_seconds: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileMetadata {
    id: String,
    domain: String,
    original_name: String,
    mime_type: String,
    size_bytes: i64,
    sha256: String,
    entity_type: Option<String>,
    entity_id: Option<String>,
    status: String,
    upload_attempts: i32,
    failure_reason: Option<String>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
    available_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileList {
    items: Vec<FileMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListQuery {
    domain: Option<String>,
    entity_type: Option<String>,
    entity_id: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FailureRequest {
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrphanQuery {
    older_than_hours: Option<i64>,
}

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/api/v1/files", get(list).post(prepare))
        .route("/api/v1/files/orphans", get(orphans))
        .route("/api/v1/files/{id}", get(metadata).delete(delete_metadata))
        .route("/api/v1/files/{id}/upload-url", post(refresh_upload_url))
        .route("/api/v1/files/{id}/complete", post(mark_complete))
        .route("/api/v1/files/{id}/fail", post(mark_failed))
        .route("/api/v1/files/{id}/download-url", post(download_url))
}

async fn prepare(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(mut input): Json<PrepareRequest>,
) -> Result<(StatusCode, Json<PrepareResponse>), ApiError> {
    principal.require_scope("files:write")?;
    ensure_database(&state)?;
    validate_prepare(&mut input)?;
    let storage = storage_config()?;
    let user_id = user_uuid(&principal.user_id)?;

    if let Some(row) = sqlx::query(
        "SELECT * FROM file_objects WHERE user_id=$1 AND domain=$2 AND sha256=$3 AND size_bytes=$4 AND deleted_at IS NULL",
    )
    .bind(user_id)
    .bind(&input.domain)
    .bind(&input.sha256)
    .bind(input.size_bytes)
    .fetch_optional(&state.pool)
    .await
    .map_err(database_error)?
    {
        let file = row_to_metadata(&row)?;
        let upload = if file.status == "available" {
            None
        } else {
            let key: String = row.try_get("storage_key").map_err(database_error)?;
            Some(signed_transfer(storage.presign_put(&key, &file.sha256, Utc::now()).map_err(storage_error)?))
        };
        return Ok((StatusCode::OK, Json(PrepareResponse { file, deduplicated: true, upload })));
    }

    let id = Uuid::new_v4();
    let storage_key = format!(
        "{}/{}/{}/{}",
        input.domain,
        user_id,
        &input.sha256[0..2],
        input.sha256
    );
    let row = sqlx::query(
        "INSERT INTO file_objects (id,user_id,domain,original_name,mime_type,size_bytes,sha256,storage_key,entity_type,entity_id,upload_attempts) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,1) RETURNING *",
    )
    .bind(id)
    .bind(user_id)
    .bind(&input.domain)
    .bind(&input.original_name)
    .bind(&input.mime_type)
    .bind(input.size_bytes)
    .bind(&input.sha256)
    .bind(&storage_key)
    .bind(&input.entity_type)
    .bind(&input.entity_id)
    .fetch_one(&state.pool)
    .await
    .map_err(database_error)?;
    let file = row_to_metadata(&row)?;
    let upload = storage
        .presign_put(&storage_key, &file.sha256, Utc::now())
        .map_err(storage_error)?;
    Ok((
        StatusCode::CREATED,
        Json(PrepareResponse {
            file,
            deduplicated: false,
            upload: Some(signed_transfer(upload)),
        }),
    ))
}

async fn list(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Query(query): Query<ListQuery>,
) -> Result<Json<FileList>, ApiError> {
    principal.require_scope("files:read")?;
    ensure_database(&state)?;
    if let Some(domain) = query.domain.as_deref() {
        validate_domain(domain)?;
    }
    if query.entity_type.is_some() != query.entity_id.is_some() {
        return Err(bad_request("entityType 与 entityId 必须同时提供"));
    }
    let owner = user_uuid(&principal.user_id)?;
    let limit = query.limit.unwrap_or(50).clamp(1, MAX_LIST_LIMIT);
    let rows = sqlx::query(
        "SELECT * FROM file_objects WHERE user_id=$1 AND deleted_at IS NULL \
         AND ($2::text IS NULL OR domain=$2) \
         AND ($3::text IS NULL OR entity_type=$3) \
         AND ($4::text IS NULL OR entity_id=$4) \
         ORDER BY created_at DESC LIMIT $5",
    )
    .bind(owner)
    .bind(query.domain)
    .bind(query.entity_type)
    .bind(query.entity_id)
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(database_error)?;
    Ok(Json(FileList { items: rows.iter().map(row_to_metadata).collect::<Result<_, _>>()? }))
}

async fn metadata(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    AxumPath(id): AxumPath<Uuid>,
) -> Result<Json<FileMetadata>, ApiError> {
    principal.require_scope("files:read")?;
    let row = owned_row(&state, &principal.user_id, id).await?;
    Ok(Json(row_to_metadata(&row)?))
}

async fn refresh_upload_url(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    AxumPath(id): AxumPath<Uuid>,
) -> Result<Json<SignedTransfer>, ApiError> {
    principal.require_scope("files:write")?;
    let row = owned_row(&state, &principal.user_id, id).await?;
    let status: String = row.try_get("status").map_err(database_error)?;
    if status == "available" {
        return Err(bad_request("文件已完成上传，无需重新签名"));
    }
    let key: String = row.try_get("storage_key").map_err(database_error)?;
    let sha256: String = row.try_get("sha256").map_err(database_error)?;
    sqlx::query("UPDATE file_objects SET upload_attempts=upload_attempts+1, status='pending', failure_reason=NULL, updated_at=now() WHERE id=$1")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(database_error)?;
    let signed = storage_config()?.presign_put(&key, &sha256, Utc::now()).map_err(storage_error)?;
    Ok(Json(signed_transfer(signed)))
}

async fn mark_complete(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    AxumPath(id): AxumPath<Uuid>,
) -> Result<Json<FileMetadata>, ApiError> {
    principal.require_scope("files:write")?;
    let owner = user_uuid(&principal.user_id)?;
    ensure_database(&state)?;
    let row = sqlx::query(
        "UPDATE file_objects SET status='available', available_at=COALESCE(available_at,now()), failure_reason=NULL, updated_at=now() \
         WHERE id=$1 AND user_id=$2 AND deleted_at IS NULL RETURNING *",
    )
    .bind(id)
    .bind(owner)
    .fetch_optional(&state.pool)
    .await
    .map_err(database_error)?
    .ok_or_else(not_found)?;
    Ok(Json(row_to_metadata(&row)?))
}

async fn mark_failed(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    AxumPath(id): AxumPath<Uuid>,
    Json(input): Json<FailureRequest>,
) -> Result<Json<FileMetadata>, ApiError> {
    principal.require_scope("files:write")?;
    let owner = user_uuid(&principal.user_id)?;
    ensure_database(&state)?;
    let reason = input.reason.unwrap_or_else(|| "client upload failed".to_owned());
    let reason: String = reason.trim().chars().filter(|c| !c.is_control()).take(300).collect();
    let row = sqlx::query(
        "UPDATE file_objects SET status='failed', failure_reason=$3, updated_at=now() \
         WHERE id=$1 AND user_id=$2 AND deleted_at IS NULL RETURNING *",
    )
    .bind(id)
    .bind(owner)
    .bind(reason)
    .fetch_optional(&state.pool)
    .await
    .map_err(database_error)?
    .ok_or_else(not_found)?;
    Ok(Json(row_to_metadata(&row)?))
}

async fn download_url(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    AxumPath(id): AxumPath<Uuid>,
) -> Result<Json<SignedTransfer>, ApiError> {
    principal.require_scope("files:read")?;
    let row = owned_row(&state, &principal.user_id, id).await?;
    let status: String = row.try_get("status").map_err(database_error)?;
    if status != "available" {
        return Err(bad_request("文件尚未完成上传"));
    }
    let key: String = row.try_get("storage_key").map_err(database_error)?;
    let signed = storage_config()?.presign_get(&key, Utc::now()).map_err(storage_error)?;
    Ok(Json(signed_transfer(signed)))
}

async fn delete_metadata(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    AxumPath(id): AxumPath<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    principal.require_scope("files:write")?;
    let owner = user_uuid(&principal.user_id)?;
    ensure_database(&state)?;
    let changed = sqlx::query(
        "UPDATE file_objects SET deleted_at=now(), updated_at=now() WHERE id=$1 AND user_id=$2 AND deleted_at IS NULL",
    )
    .bind(id)
    .bind(owner)
    .execute(&state.pool)
    .await
    .map_err(database_error)?;
    if changed.rows_affected() == 0 {
        return Err(not_found());
    }
    Ok(Json(serde_json::json!({"deleted": true, "id": id})))
}

async fn orphans(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Query(query): Query<OrphanQuery>,
) -> Result<Json<FileList>, ApiError> {
    principal.require_scope("files:read")?;
    ensure_database(&state)?;
    let owner = user_uuid(&principal.user_id)?;
    let hours = query.older_than_hours.unwrap_or(24).clamp(1, 24 * 365);
    let rows = sqlx::query(
        "SELECT * FROM file_objects WHERE user_id=$1 AND deleted_at IS NULL AND entity_type IS NULL \
         AND status IN ('pending','failed') AND created_at < now() - ($2::text || ' hours')::interval \
         ORDER BY created_at ASC LIMIT 200",
    )
    .bind(owner)
    .bind(hours.to_string())
    .fetch_all(&state.pool)
    .await
    .map_err(database_error)?;
    Ok(Json(FileList { items: rows.iter().map(row_to_metadata).collect::<Result<_, _>>()? }))
}

async fn owned_row(
    state: &AppState,
    user_id: &UserId,
    id: Uuid,
) -> Result<sqlx::postgres::PgRow, ApiError> {
    ensure_database(state)?;
    let owner = user_uuid(user_id)?;
    sqlx::query("SELECT * FROM file_objects WHERE id=$1 AND user_id=$2 AND deleted_at IS NULL")
        .bind(id)
        .bind(owner)
        .fetch_optional(&state.pool)
        .await
        .map_err(database_error)?
        .ok_or_else(not_found)
}

fn validate_prepare(input: &mut PrepareRequest) -> Result<(), ApiError> {
    validate_domain(&input.domain)?;
    input.original_name = clean_text(&input.original_name, 180, "file");
    input.mime_type = clean_text(&input.mime_type, 120, "application/octet-stream").to_ascii_lowercase();
    if input.size_bytes <= 0 || input.size_bytes > max_file_bytes() {
        return Err(bad_request(format!("文件大小必须在 1..={} bytes", max_file_bytes())));
    }
    input.sha256 = input.sha256.trim().to_ascii_lowercase();
    if input.sha256.len() != 64 || !input.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(bad_request("sha256 必须是 64 位十六进制字符串"));
    }
    if input.entity_type.is_some() != input.entity_id.is_some() {
        return Err(bad_request("entityType 与 entityId 必须同时提供"));
    }
    if !mime_allowed(&input.domain, &input.mime_type) {
        return Err(bad_request(format!("MIME 类型不允许用于 {}", input.domain)));
    }
    if let Some(value) = input.entity_type.as_mut() {
        *value = clean_text(value, 120, "");
    }
    if let Some(value) = input.entity_id.as_mut() {
        *value = clean_text(value, 180, "");
    }
    Ok(())
}

fn validate_domain(domain: &str) -> Result<(), ApiError> {
    if DOMAINS.contains(&domain) {
        Ok(())
    } else {
        Err(bad_request("文件 domain 不受支持"))
    }
}

fn mime_allowed(domain: &str, mime: &str) -> bool {
    match domain {
        "finance_imports" => matches!(mime, "text/csv" | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" | "application/vnd.ms-excel"),
        "notes_attachments" => mime.starts_with("image/") || mime.starts_with("audio/") || mime.starts_with("video/") || matches!(mime, "application/pdf" | "text/plain" | "text/markdown" | "application/zip"),
        "english_audio" => mime.starts_with("audio/"),
        "photos" => mime.starts_with("image/") || mime.starts_with("video/"),
        "workout_imports" => mime.starts_with("image/") || matches!(mime, "application/pdf" | "text/html" | "text/plain"),
        "backups" => matches!(mime, "application/zip" | "application/gzip" | "application/json" | "application/octet-stream"),
        _ => false,
    }
}

fn max_file_bytes() -> i64 {
    std::env::var("FILE_MAX_UPLOAD_BYTES")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_MAX_FILE_BYTES)
}

fn storage_config() -> Result<ObjectStorageConfig, ApiError> {
    ObjectStorageConfig::from_env().map_err(storage_error)
}

fn signed_transfer(value: PresignedRequest) -> SignedTransfer {
    SignedTransfer {
        url: value.url,
        required_headers: value.required_headers,
        expires_seconds: value.expires_seconds,
    }
}

fn row_to_metadata(row: &sqlx::postgres::PgRow) -> Result<FileMetadata, ApiError> {
    Ok(FileMetadata {
        id: row.try_get::<Uuid, _>("id").map_err(database_error)?.to_string(),
        domain: row.try_get("domain").map_err(database_error)?,
        original_name: row.try_get("original_name").map_err(database_error)?,
        mime_type: row.try_get("mime_type").map_err(database_error)?,
        size_bytes: row.try_get("size_bytes").map_err(database_error)?,
        sha256: row.try_get("sha256").map_err(database_error)?,
        entity_type: row.try_get("entity_type").map_err(database_error)?,
        entity_id: row.try_get("entity_id").map_err(database_error)?,
        status: row.try_get("status").map_err(database_error)?,
        upload_attempts: row.try_get("upload_attempts").map_err(database_error)?,
        failure_reason: row.try_get("failure_reason").map_err(database_error)?,
        created_at: row.try_get("created_at").map_err(database_error)?,
        updated_at: row.try_get("updated_at").map_err(database_error)?,
        available_at: row.try_get("available_at").map_err(database_error)?,
    })
}

fn ensure_database(state: &AppState) -> Result<(), ApiError> {
    if state.database_enabled {
        Ok(())
    } else {
        Err(ApiError::new(ErrorCode::TemporarilyUnavailable, "文件服务需要 PostgreSQL", StatusCode::SERVICE_UNAVAILABLE))
    }
}

fn user_uuid(user_id: &UserId) -> Result<Uuid, ApiError> {
    Uuid::parse_str(user_id.as_str()).map_err(|_| bad_request("当前账号不能使用文件服务"))
}

fn clean_text(value: &str, max: usize, fallback: &str) -> String {
    let value: String = value.trim().chars().filter(|character| !character.is_control()).take(max).collect();
    if value.is_empty() { fallback.to_owned() } else { value }
}

fn bad_request(message: impl Into<String>) -> ApiError {
    ApiError::new(ErrorCode::InvalidRequest, message, StatusCode::BAD_REQUEST)
}

fn not_found() -> ApiError {
    ApiError::new(ErrorCode::InvalidRequest, "文件不存在", StatusCode::NOT_FOUND)
}

fn database_error(error: sqlx::Error) -> ApiError {
    ApiError::new(ErrorCode::TemporarilyUnavailable, format!("文件数据库操作失败: {error}"), StatusCode::SERVICE_UNAVAILABLE)
}

fn storage_error(message: impl Into<String>) -> ApiError {
    ApiError::new(ErrorCode::TemporarilyUnavailable, message, StatusCode::SERVICE_UNAVAILABLE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domains_and_mime_allowlists_are_fail_closed() {
        assert!(mime_allowed("finance_imports", "text/csv"));
        assert!(!mime_allowed("finance_imports", "image/png"));
        assert!(mime_allowed("photos", "image/jpeg"));
        assert!(!mime_allowed("unknown", "image/jpeg"));
    }

    #[test]
    fn sha256_validation_rejects_non_hex_input() {
        let mut input = PrepareRequest {
            domain: "notes_attachments".to_owned(),
            original_name: "a.txt".to_owned(),
            mime_type: "text/plain".to_owned(),
            size_bytes: 12,
            sha256: "z".repeat(64),
            entity_type: None,
            entity_id: None,
        };
        assert!(validate_prepare(&mut input).is_err());
    }

    #[test]
    fn private_vault_is_not_a_supported_cloud_domain() {
        assert!(validate_domain("private_vault").is_err());
    }
}
