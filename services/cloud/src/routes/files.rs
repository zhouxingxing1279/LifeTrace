//! EPIC-12 unified file metadata and object-storage routes.
//!
//! Metadata is committed before bytes are uploaded. Large bytes never enter
//! the sync JSON protocol; clients use short-lived signed object URLs instead.

use std::path::Path as StdPath;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Duration, Utc};
use lifetrace_contracts::{ErrorCode, UserId};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgRow;
use sqlx::Row;
use uuid::Uuid;

use crate::auth::AuthenticatedPrincipal;
use crate::error::ApiError;
use crate::object_storage::{ObjectStorage, ObjectStorageError, PresignedRequest};
use crate::state::AppState;

const DEFAULT_LIST_LIMIT: i64 = 100;
const MAX_LIST_LIMIT: i64 = 500;
const INTEGRITY_SCAN_LIMIT: i64 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileDomain {
    FinanceImport,
    NotesAttachment,
    EnglishAudio,
    Photo,
    WorkoutImport,
    Backup,
}

impl FileDomain {
    fn parse(value: &str) -> Result<Self, ApiError> {
        match value {
            "finance_import" => Ok(Self::FinanceImport),
            "notes_attachment" => Ok(Self::NotesAttachment),
            "english_audio" => Ok(Self::EnglishAudio),
            "photo" => Ok(Self::Photo),
            "workout_import" => Ok(Self::WorkoutImport),
            "backup" => Ok(Self::Backup),
            _ => Err(bad_request("unsupported file domain")),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::FinanceImport => "finance_import",
            Self::NotesAttachment => "notes_attachment",
            Self::EnglishAudio => "english_audio",
            Self::Photo => "photo",
            Self::WorkoutImport => "workout_import",
            Self::Backup => "backup",
        }
    }

    fn storage_prefix(self) -> &'static str {
        match self {
            Self::FinanceImport => "finance/imports",
            Self::NotesAttachment => "notes/attachments",
            Self::EnglishAudio => "english/audio",
            Self::Photo => "photos",
            Self::WorkoutImport => "workout/imports",
            Self::Backup => "backups",
        }
    }

    fn max_bytes(self) -> u64 {
        match self {
            Self::FinanceImport => 128 * 1024 * 1024,
            Self::NotesAttachment => 128 * 1024 * 1024,
            Self::EnglishAudio => 256 * 1024 * 1024,
            Self::Photo => 1024 * 1024 * 1024,
            Self::WorkoutImport => 128 * 1024 * 1024,
            Self::Backup => 2 * 1024 * 1024 * 1024,
        }
    }

    fn allows_mime(self, mime: &str) -> bool {
        let mime = mime.to_ascii_lowercase();
        match self {
            Self::FinanceImport => matches!(
                mime.as_str(),
                "text/csv"
                    | "application/csv"
                    | "application/vnd.ms-excel"
                    | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            ),
            Self::NotesAttachment => {
                let supported_attachment = matches!(
                mime.as_str(),
                "application/pdf"
                    | "text/plain"
                    | "text/markdown"
                    | "application/json"
                    | "application/msword"
                    | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
                    | "application/vnd.ms-excel"
                    | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
                    | "application/vnd.ms-powerpoint"
                    | "application/vnd.openxmlformats-officedocument.presentationml.presentation"
                    | "audio/mpeg"
                    | "audio/mp4"
                    | "audio/wav"
                    | "audio/ogg"
                    | "audio/webm"
            );
                supported_attachment || is_safe_image_mime(&mime)
            }
            Self::EnglishAudio => matches!(
                mime.as_str(),
                "audio/mpeg"
                    | "audio/mp4"
                    | "audio/wav"
                    | "audio/x-wav"
                    | "audio/ogg"
                    | "audio/webm"
            ),
            Self::Photo => is_safe_image_mime(&mime),
            Self::WorkoutImport => {
                matches!(
                    mime.as_str(),
                    "text/csv" | "application/json" | "application/zip"
                ) || is_safe_image_mime(&mime)
            }
            Self::Backup => matches!(
                mime.as_str(),
                "application/zip"
                    | "application/gzip"
                    | "application/x-gzip"
                    | "application/x-tar"
                    | "application/octet-stream"
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMetadata {
    pub id: Uuid,
    pub domain: String,
    pub original_name: String,
    pub sha256: String,
    pub size_bytes: i64,
    pub mime_type: String,
    pub status: String,
    pub upload_attempts: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub ready_at: Option<DateTime<Utc>>,
}

#[derive(Debug)]
struct FileRecord {
    metadata: FileMetadata,
    object_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadInitInput {
    domain: String,
    original_name: String,
    sha256: String,
    size_bytes: u64,
    mime_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UploadInitResponse {
    file: FileMetadata,
    upload: Option<PresignedRequest>,
    deduplicated: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadResponse {
    file: FileMetadata,
    download: PresignedRequest,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileList {
    items: Vec<FileMetadata>,
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    domain: Option<String>,
    status: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct IntegrityDiagnostics {
    stale_pending: Vec<FileMetadata>,
    missing_ready: Vec<FileMetadata>,
    cleanup_pending: Vec<FileMetadata>,
}

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/api/v1/files", get(list_files))
        .route("/api/v1/files/uploads", post(initialize_upload))
        .route("/api/v1/files/diagnostics", get(diagnostics))
        .route("/api/v1/files/{id}", get(get_file).delete(delete_file))
        .route("/api/v1/files/{id}/complete", post(complete_upload))
        .route("/api/v1/files/{id}/download", post(download_file))
}

async fn list_files(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Query(query): Query<ListQuery>,
) -> Result<Json<FileList>, ApiError> {
    principal.require_scope("files:read")?;
    ensure_database(&state)?;
    let user_id = user_uuid(&principal.user_id)?;
    let domain = query
        .domain
        .as_deref()
        .map(FileDomain::parse)
        .transpose()?
        .map(|value| value.as_str().to_owned());
    let status = query.status.as_deref().map(validate_status).transpose()?;
    let limit = query
        .limit
        .unwrap_or(DEFAULT_LIST_LIMIT)
        .clamp(1, MAX_LIST_LIMIT);
    let rows = sqlx::query(
        "SELECT id,domain,original_name,sha256,size_bytes,mime_type,status,upload_attempts,object_key,storage_cleanup_pending,created_at,updated_at,ready_at \
         FROM cloud_file_objects \
         WHERE user_id=$1 AND deleted_at IS NULL \
           AND ($2::text IS NULL OR domain=$2) \
           AND ($3::text IS NULL OR status=$3) \
         ORDER BY created_at DESC LIMIT $4",
    )
    .bind(user_id)
    .bind(domain)
    .bind(status)
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(database_error)?;
    Ok(Json(FileList {
        items: rows
            .iter()
            .map(row_to_record)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|record| record.metadata)
            .collect(),
    }))
}

async fn get_file(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<FileMetadata>, ApiError> {
    principal.require_scope("files:read")?;
    let record = load_owned(&state, &principal.user_id, id, false).await?;
    Ok(Json(record.metadata))
}

async fn initialize_upload(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(input): Json<UploadInitInput>,
) -> Result<Json<UploadInitResponse>, ApiError> {
    principal.require_scope("files:write")?;
    ensure_database(&state)?;
    let storage = storage()?;
    let user_id = user_uuid(&principal.user_id)?;
    let domain = FileDomain::parse(input.domain.trim())?;
    let sha256 = normalize_sha256(&input.sha256)?;
    let mime_type = input.mime_type.trim().to_ascii_lowercase();
    if !domain.allows_mime(&mime_type) {
        return Err(bad_request("file MIME type is not allowed for this domain"));
    }
    let max_bytes = domain.max_bytes().min(storage.max_file_bytes());
    if input.size_bytes > max_bytes {
        return Err(ApiError::new(
            ErrorCode::PayloadTooLarge,
            format!("file exceeds domain limit of {max_bytes} bytes"),
            StatusCode::PAYLOAD_TOO_LARGE,
        ));
    }
    let size_bytes = i64::try_from(input.size_bytes)
        .map_err(|_| bad_request("file size is outside supported range"))?;
    let original_name = clean_file_name(&input.original_name);
    let object_key = object_key(user_id, domain, &sha256);
    let id = Uuid::new_v4();

    let row = sqlx::query(
        "INSERT INTO cloud_file_objects \
         (id,user_id,domain,original_name,sha256,size_bytes,mime_type,object_key,status,upload_attempts,storage_cleanup_pending,last_error) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,'pending',1,FALSE,NULL) \
         ON CONFLICT (user_id,domain,sha256,size_bytes) WHERE deleted_at IS NULL DO UPDATE SET \
           original_name=EXCLUDED.original_name, \
           mime_type=EXCLUDED.mime_type, \
           updated_at=now(), \
           last_error=NULL, \
           status=CASE WHEN cloud_file_objects.status='ready' THEN 'ready' ELSE 'pending' END, \
           upload_attempts=CASE WHEN cloud_file_objects.status='ready' THEN cloud_file_objects.upload_attempts ELSE cloud_file_objects.upload_attempts+1 END \
         RETURNING id,domain,original_name,sha256,size_bytes,mime_type,status,upload_attempts,object_key,storage_cleanup_pending,created_at,updated_at,ready_at",
    )
    .bind(id)
    .bind(user_id)
    .bind(domain.as_str())
    .bind(original_name)
    .bind(&sha256)
    .bind(size_bytes)
    .bind(&mime_type)
    .bind(object_key)
    .fetch_one(&state.pool)
    .await
    .map_err(database_error)?;
    let record = row_to_record(&row)?;
    let deduplicated = record.metadata.status == "ready";
    let upload = if deduplicated {
        None
    } else {
        Some(
            storage
                .presign_upload(
                    &record.object_key,
                    &record.metadata.sha256,
                    &record.metadata.domain,
                    &record.metadata.mime_type,
                )
                .map_err(storage_error)?,
        )
    };
    Ok(Json(UploadInitResponse {
        file: record.metadata,
        upload,
        deduplicated,
    }))
}

async fn complete_upload(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<FileMetadata>, ApiError> {
    principal.require_scope("files:write")?;
    let storage = storage()?;
    let record = load_owned(&state, &principal.user_id, id, false).await?;
    if record.metadata.status == "ready" {
        return Ok(Json(record.metadata));
    }
    if record.metadata.status == "deleted" {
        return Err(not_found());
    }
    let head = storage
        .head_object(&record.object_key)
        .await
        .map_err(storage_error)?;
    let Some(head) = head else {
        mark_failed(&state, id, "object is missing after upload").await?;
        return Err(conflict("uploaded object was not found"));
    };
    let valid = head.size_bytes == record.metadata.size_bytes.max(0) as u64
        && head.sha256.as_deref() == Some(record.metadata.sha256.as_str())
        && head.domain.as_deref() == Some(record.metadata.domain.as_str())
        && head.mime_type.as_deref() == Some(record.metadata.mime_type.as_str());
    if !valid {
        mark_failed(
            &state,
            id,
            "object HEAD metadata does not match file declaration",
        )
        .await?;
        return Err(conflict(
            "uploaded object integrity metadata does not match",
        ));
    }
    let row = sqlx::query(
        "UPDATE cloud_file_objects SET status='ready',ready_at=COALESCE(ready_at,now()),updated_at=now(),last_error=NULL \
         WHERE id=$1 RETURNING id,domain,original_name,sha256,size_bytes,mime_type,status,upload_attempts,object_key,storage_cleanup_pending,created_at,updated_at,ready_at",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(database_error)?;
    Ok(Json(row_to_record(&row)?.metadata))
}

async fn download_file(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<DownloadResponse>, ApiError> {
    principal.require_scope("files:read")?;
    let storage = storage()?;
    let record = load_owned(&state, &principal.user_id, id, false).await?;
    if record.metadata.status != "ready" {
        return Err(conflict("file bytes are not ready for download"));
    }
    let download = storage
        .presign_download(&record.object_key)
        .map_err(storage_error)?;
    Ok(Json(DownloadResponse {
        file: record.metadata,
        download,
    }))
}

async fn delete_file(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    principal.require_scope("files:write")?;
    let storage = storage()?;
    let record = load_owned(&state, &principal.user_id, id, false).await?;
    sqlx::query(
        "UPDATE cloud_file_objects SET status='deleted',deleted_at=now(),updated_at=now(),storage_cleanup_pending=TRUE,last_error=NULL WHERE id=$1",
    )
    .bind(id)
    .execute(&state.pool)
    .await
    .map_err(database_error)?;
    let cleanup_error = match storage.delete_object(&record.object_key).await {
        Ok(()) => {
            sqlx::query("UPDATE cloud_file_objects SET storage_cleanup_pending=FALSE,last_error=NULL,updated_at=now() WHERE id=$1")
                .bind(id)
                .execute(&state.pool)
                .await
                .map_err(database_error)?;
            None
        }
        Err(error) => {
            let message = truncate_error(&error.to_string());
            sqlx::query("UPDATE cloud_file_objects SET last_error=$2,updated_at=now() WHERE id=$1")
                .bind(id)
                .bind(&message)
                .execute(&state.pool)
                .await
                .map_err(database_error)?;
            Some(message)
        }
    };
    Ok(Json(serde_json::json!({
        "deleted": true,
        "id": id,
        "storageCleanupPending": cleanup_error.is_some()
    })))
}

async fn diagnostics(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<Json<IntegrityDiagnostics>, ApiError> {
    principal.require_scope("files:read")?;
    ensure_database(&state)?;
    let storage = storage()?;
    let user_id = user_uuid(&principal.user_id)?;
    let stale_before = Utc::now() - Duration::hours(24);
    let stale_rows = sqlx::query(
        "SELECT id,domain,original_name,sha256,size_bytes,mime_type,status,upload_attempts,object_key,storage_cleanup_pending,created_at,updated_at,ready_at \
         FROM cloud_file_objects WHERE user_id=$1 AND deleted_at IS NULL AND status IN ('pending','failed') AND updated_at<$2 \
         ORDER BY updated_at ASC LIMIT $3",
    )
    .bind(user_id)
    .bind(stale_before)
    .bind(INTEGRITY_SCAN_LIMIT)
    .fetch_all(&state.pool)
    .await
    .map_err(database_error)?;
    let ready_rows = sqlx::query(
        "SELECT id,domain,original_name,sha256,size_bytes,mime_type,status,upload_attempts,object_key,storage_cleanup_pending,created_at,updated_at,ready_at \
         FROM cloud_file_objects WHERE user_id=$1 AND deleted_at IS NULL AND status='ready' ORDER BY updated_at ASC LIMIT $2",
    )
    .bind(user_id)
    .bind(INTEGRITY_SCAN_LIMIT)
    .fetch_all(&state.pool)
    .await
    .map_err(database_error)?;
    let cleanup_rows = sqlx::query(
        "SELECT id,domain,original_name,sha256,size_bytes,mime_type,status,upload_attempts,object_key,storage_cleanup_pending,created_at,updated_at,ready_at \
         FROM cloud_file_objects WHERE user_id=$1 AND storage_cleanup_pending=TRUE ORDER BY updated_at ASC LIMIT $2",
    )
    .bind(user_id)
    .bind(INTEGRITY_SCAN_LIMIT)
    .fetch_all(&state.pool)
    .await
    .map_err(database_error)?;

    let stale_pending = rows_to_metadata(&stale_rows)?;
    let cleanup_pending = rows_to_metadata(&cleanup_rows)?;
    let mut missing_ready = Vec::new();
    for row in &ready_rows {
        let record = row_to_record(row)?;
        if storage
            .head_object(&record.object_key)
            .await
            .map_err(storage_error)?
            .is_none()
        {
            missing_ready.push(record.metadata);
        }
    }
    Ok(Json(IntegrityDiagnostics {
        stale_pending,
        missing_ready,
        cleanup_pending,
    }))
}

async fn load_owned(
    state: &AppState,
    user_id: &UserId,
    id: Uuid,
    include_deleted: bool,
) -> Result<FileRecord, ApiError> {
    ensure_database(state)?;
    let owner = user_uuid(user_id)?;
    let row = sqlx::query(
        "SELECT id,domain,original_name,sha256,size_bytes,mime_type,status,upload_attempts,object_key,storage_cleanup_pending,created_at,updated_at,ready_at \
         FROM cloud_file_objects WHERE id=$1 AND user_id=$2 AND ($3 OR deleted_at IS NULL)",
    )
    .bind(id)
    .bind(owner)
    .bind(include_deleted)
    .fetch_optional(&state.pool)
    .await
    .map_err(database_error)?
    .ok_or_else(not_found)?;
    row_to_record(&row)
}

async fn mark_failed(state: &AppState, id: Uuid, message: &str) -> Result<(), ApiError> {
    sqlx::query(
        "UPDATE cloud_file_objects SET status='failed',updated_at=now(),last_error=$2 WHERE id=$1 AND deleted_at IS NULL",
    )
    .bind(id)
    .bind(truncate_error(message))
    .execute(&state.pool)
    .await
    .map_err(database_error)?;
    Ok(())
}

fn rows_to_metadata(rows: &[PgRow]) -> Result<Vec<FileMetadata>, ApiError> {
    rows.iter()
        .map(row_to_record)
        .map(|record| record.map(|value| value.metadata))
        .collect()
}

fn row_to_record(row: &PgRow) -> Result<FileRecord, ApiError> {
    Ok(FileRecord {
        metadata: FileMetadata {
            id: row.try_get("id").map_err(database_error)?,
            domain: row.try_get("domain").map_err(database_error)?,
            original_name: row.try_get("original_name").map_err(database_error)?,
            sha256: row.try_get("sha256").map_err(database_error)?,
            size_bytes: row.try_get("size_bytes").map_err(database_error)?,
            mime_type: row.try_get("mime_type").map_err(database_error)?,
            status: row.try_get("status").map_err(database_error)?,
            upload_attempts: row.try_get("upload_attempts").map_err(database_error)?,
            created_at: row.try_get("created_at").map_err(database_error)?,
            updated_at: row.try_get("updated_at").map_err(database_error)?,
            ready_at: row.try_get("ready_at").map_err(database_error)?,
        },
        object_key: row.try_get("object_key").map_err(database_error)?,
    })
}

fn storage() -> Result<ObjectStorage, ApiError> {
    ObjectStorage::from_env().map_err(storage_error)
}

fn ensure_database(state: &AppState) -> Result<(), ApiError> {
    if state.database_enabled {
        Ok(())
    } else {
        Err(ApiError::new(
            ErrorCode::TemporarilyUnavailable,
            "file service requires PostgreSQL",
            StatusCode::SERVICE_UNAVAILABLE,
        ))
    }
}

fn user_uuid(user_id: &UserId) -> Result<Uuid, ApiError> {
    Uuid::parse_str(user_id.as_str()).map_err(|_| {
        ApiError::new(
            ErrorCode::InvalidRequest,
            "current account cannot use the file service",
            StatusCode::BAD_REQUEST,
        )
    })
}

fn object_key(user_id: Uuid, domain: FileDomain, sha256: &str) -> String {
    format!(
        "users/{user_id}/{}/{}/{}",
        domain.storage_prefix(),
        &sha256[..2],
        sha256
    )
}

fn normalize_sha256(value: &str) -> Result<String, ApiError> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.len() == 64 && normalized.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(normalized)
    } else {
        Err(bad_request(
            "sha256 must be exactly 64 hexadecimal characters",
        ))
    }
}

fn clean_file_name(value: &str) -> String {
    let raw = StdPath::new(value.trim())
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("file");
    let cleaned: String = raw
        .chars()
        .filter(|character| !character.is_control())
        .take(240)
        .collect();
    if cleaned.trim().is_empty() {
        "file".to_owned()
    } else {
        cleaned
    }
}

fn validate_status(value: &str) -> Result<String, ApiError> {
    match value {
        "pending" | "ready" | "failed" => Ok(value.to_owned()),
        _ => Err(bad_request("unsupported file status filter")),
    }
}

fn is_safe_image_mime(value: &str) -> bool {
    matches!(
        value,
        "image/jpeg" | "image/png" | "image/webp" | "image/heic" | "image/heif" | "image/gif"
    )
}

fn truncate_error(value: &str) -> String {
    value.chars().take(500).collect()
}

fn bad_request(message: impl Into<String>) -> ApiError {
    ApiError::new(ErrorCode::InvalidRequest, message, StatusCode::BAD_REQUEST)
}

fn conflict(message: impl Into<String>) -> ApiError {
    ApiError::new(ErrorCode::InvalidRequest, message, StatusCode::CONFLICT)
}

fn not_found() -> ApiError {
    ApiError::new(
        ErrorCode::InvalidRequest,
        "file does not exist",
        StatusCode::NOT_FOUND,
    )
}

fn database_error(error: sqlx::Error) -> ApiError {
    ApiError::new(
        ErrorCode::TemporarilyUnavailable,
        format!("file metadata database operation failed: {error}"),
        StatusCode::SERVICE_UNAVAILABLE,
    )
}

fn storage_error(error: ObjectStorageError) -> ApiError {
    ApiError::new(
        ErrorCode::TemporarilyUnavailable,
        format!("object storage unavailable: {error}"),
        StatusCode::SERVICE_UNAVAILABLE,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_epic12_domains_have_stable_storage_prefixes() {
        let cases = [
            ("finance_import", "finance/imports"),
            ("notes_attachment", "notes/attachments"),
            ("english_audio", "english/audio"),
            ("photo", "photos"),
            ("workout_import", "workout/imports"),
            ("backup", "backups"),
        ];
        for (domain, prefix) in cases {
            assert_eq!(FileDomain::parse(domain).unwrap().storage_prefix(), prefix);
        }
    }

    #[test]
    fn domain_mime_allowlist_is_fail_closed() {
        assert!(FileDomain::FinanceImport.allows_mime("text/csv"));
        assert!(!FileDomain::FinanceImport.allows_mime("text/html"));
        assert!(FileDomain::EnglishAudio.allows_mime("audio/mpeg"));
        assert!(!FileDomain::EnglishAudio.allows_mime("video/mp4"));
        assert!(FileDomain::Photo.allows_mime("image/webp"));
        assert!(!FileDomain::Photo.allows_mime("image/svg+xml"));
    }

    #[test]
    fn sha256_is_normalized_and_path_is_server_generated() {
        let sha = "A".repeat(64);
        let normalized = normalize_sha256(&sha).unwrap();
        assert_eq!(normalized, "a".repeat(64));
        let key = object_key(
            Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
            FileDomain::NotesAttachment,
            &normalized,
        );
        assert_eq!(
            key,
            format!(
                "users/00000000-0000-0000-0000-000000000001/notes/attachments/aa/{}",
                "a".repeat(64)
            )
        );
    }

    #[test]
    fn metadata_shape_contains_no_raw_file_bytes() {
        let serialized = serde_json::to_string(&FileMetadata {
            id: Uuid::nil(),
            domain: "photo".to_owned(),
            original_name: "photo.jpg".to_owned(),
            sha256: "a".repeat(64),
            size_bytes: 1024,
            mime_type: "image/jpeg".to_owned(),
            status: "ready".to_owned(),
            upload_attempts: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ready_at: Some(Utc::now()),
        })
        .unwrap();
        assert!(!serialized.contains("content"));
        assert!(!serialized.contains("bytes"));
        assert!(serialized.contains("sha256"));
    }
}
