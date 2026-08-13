//! BeeCount-compatible attachment persistence over LifeTrace PostgreSQL.

use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use lifetrace_contracts::{ErrorCode, UserId};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::error::ApiError;

const FILE_ENTITY_PREFIX: &str = "beecount-file:";
pub const MAX_BATCH_EXISTS_HASHES: usize = 1024;

#[derive(Debug, Clone, Serialize)]
pub struct AttachmentUploadOut {
    pub file_id: String,
    pub ledger_id: String,
    pub sha256: String,
    pub size: i64,
    pub mime_type: Option<String>,
    pub file_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AttachmentBatchExistsRequest {
    pub ledger_id: String,
    #[serde(default)]
    pub sha256_list: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttachmentExistsItem {
    pub sha256: String,
    pub exists: bool,
    pub file_id: Option<String>,
    pub size: Option<i64>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AttachmentBatchExistsResponse {
    pub items: Vec<AttachmentExistsItem>,
}

#[derive(Debug, Clone)]
pub struct AttachmentDownload {
    pub content: Vec<u8>,
    pub mime_type: Option<String>,
    pub file_name: String,
}

#[derive(Debug, Clone, Copy)]
enum AttachmentKind {
    Transaction,
    CategoryIcon,
}

impl AttachmentKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Transaction => "transaction_attachment",
            Self::CategoryIcon => "category_icon",
        }
    }
}

#[derive(Clone)]
pub struct BeeCountAttachmentService {
    pool: PgPool,
}

impl BeeCountAttachmentService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upload_transaction(
        &self,
        user_id: &UserId,
        device_id: &str,
        ledger_id: &str,
        file_name: &str,
        mime_type: Option<&str>,
        content: Vec<u8>,
    ) -> Result<AttachmentUploadOut, ApiError> {
        let actor_uuid = parse_uuid(user_id.as_str(), "invalid user identity")?;
        let access = crate::beecount_collaboration::resolve_ledger_access(
            &self.pool, actor_uuid, ledger_id, true,
        )
        .await?;
        let storage_user_id = UserId::new(access.storage_user_id.to_string());
        self.store(
            &storage_user_id,
            device_id,
            Some(ledger_id),
            AttachmentKind::Transaction,
            file_name,
            mime_type,
            content,
        )
        .await
    }

    pub async fn upload_category_icon(
        &self,
        user_id: &UserId,
        device_id: &str,
        file_name: &str,
        mime_type: Option<&str>,
        content: Vec<u8>,
    ) -> Result<AttachmentUploadOut, ApiError> {
        self.store(
            user_id,
            device_id,
            None,
            AttachmentKind::CategoryIcon,
            file_name,
            mime_type,
            content,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn store(
        &self,
        user_id: &UserId,
        device_id: &str,
        ledger_id: Option<&str>,
        kind: AttachmentKind,
        file_name: &str,
        mime_type: Option<&str>,
        content: Vec<u8>,
    ) -> Result<AttachmentUploadOut, ApiError> {
        if content.is_empty() {
            return Err(invalid("Attachment file is empty"));
        }
        let user_uuid = parse_uuid(user_id.as_str(), "invalid user identity")?;
        let device_uuid = parse_uuid(device_id, "invalid BeeCount device identity")?;
        let safe_name = safe_file_name(file_name);
        let mime_type = clean_mime_type(mime_type);
        let sha256 = hex::encode(Sha256::digest(&content));
        let mut tx = self.pool.begin().await.map_err(db_error)?;

        if let Some(existing) = find_existing(&mut tx, user_uuid, ledger_id, kind, &sha256).await? {
            tx.commit().await.map_err(db_error)?;
            return Ok(existing);
        }

        let file_uuid = Uuid::new_v4();
        let file_id = file_uuid.to_string();
        let file_entity_id = format!("{FILE_ENTITY_PREFIX}{file_id}");
        let created_at = Utc::now();
        let inserted = sqlx::query(
            "INSERT INTO cloud_file_blobs ( \
                id,user_id,file_entity_id,ledger_id,attachment_kind,sha256,size_bytes, \
                mime_type,file_name,content,created_by_device_id,created_at \
             ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) \
             ON CONFLICT DO NOTHING RETURNING id",
        )
        .bind(file_uuid)
        .bind(user_uuid)
        .bind(&file_entity_id)
        .bind(ledger_id)
        .bind(kind.as_str())
        .bind(&sha256)
        .bind(content.len() as i64)
        .bind(mime_type.as_deref())
        .bind(&safe_name)
        .bind(&content)
        .bind(device_uuid)
        .bind(created_at)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_error)?;

        if inserted.is_none() {
            let existing = find_existing(&mut tx, user_uuid, ledger_id, kind, &sha256)
                .await?
                .ok_or_else(|| internal("attachment deduplication failed"))?;
            tx.commit().await.map_err(db_error)?;
            return Ok(existing);
        }

        persist_file_metadata(
            &mut tx,
            user_uuid,
            device_uuid,
            device_id,
            &file_entity_id,
            &safe_name,
            mime_type.as_deref(),
            content.len() as i64,
            &sha256,
            created_at,
        )
        .await?;
        tx.commit().await.map_err(db_error)?;

        Ok(AttachmentUploadOut {
            file_id,
            ledger_id: ledger_id.unwrap_or_default().to_owned(),
            sha256,
            size: content.len() as i64,
            mime_type,
            file_name: Some(safe_name),
            created_at,
        })
    }

    pub async fn batch_exists(
        &self,
        user_id: &UserId,
        request: AttachmentBatchExistsRequest,
    ) -> Result<AttachmentBatchExistsResponse, ApiError> {
        if request.sha256_list.len() > MAX_BATCH_EXISTS_HASHES {
            return Err(invalid("too many attachment hashes"));
        }
        let actor_uuid = parse_uuid(user_id.as_str(), "invalid user identity")?;
        let access = crate::beecount_collaboration::resolve_ledger_access(
            &self.pool,
            actor_uuid,
            &request.ledger_id,
            false,
        )
        .await?;
        let user_uuid = access.storage_user_id;
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let wanted = request
            .sha256_list
            .iter()
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        if wanted.is_empty() {
            tx.commit().await.map_err(db_error)?;
            return Ok(AttachmentBatchExistsResponse { items: Vec::new() });
        }
        let query_hashes = wanted
            .iter()
            .filter(|value| valid_sha256(value))
            .cloned()
            .collect::<Vec<_>>();
        let rows = sqlx::query(
            "SELECT id::text,sha256,size_bytes,mime_type FROM cloud_file_blobs \
             WHERE user_id=$1 AND ledger_id=$2 \
               AND attachment_kind='transaction_attachment' AND sha256=ANY($3)",
        )
        .bind(user_uuid)
        .bind(&request.ledger_id)
        .bind(&query_hashes)
        .fetch_all(&mut *tx)
        .await
        .map_err(db_error)?;
        tx.commit().await.map_err(db_error)?;

        let items = wanted
            .into_iter()
            .map(|hash| {
                let row = rows
                    .iter()
                    .find(|row| row.try_get::<String, _>("sha256").ok().as_deref() == Some(&hash));
                AttachmentExistsItem {
                    sha256: hash,
                    exists: row.is_some(),
                    file_id: row.and_then(|value| value.try_get("id").ok()),
                    size: row.and_then(|value| value.try_get("size_bytes").ok()),
                    mime_type: row.and_then(|value| value.try_get("mime_type").ok()),
                }
            })
            .collect();
        Ok(AttachmentBatchExistsResponse { items })
    }

    pub async fn download(
        &self,
        user_id: &UserId,
        file_id: &str,
    ) -> Result<AttachmentDownload, ApiError> {
        let actor_uuid = parse_uuid(user_id.as_str(), "invalid user identity")?;
        let file_uuid = Uuid::parse_str(file_id).map_err(|_| not_found("Attachment not found"))?;
        let row = sqlx::query(
            "SELECT user_id,ledger_id,content,mime_type,file_name \
             FROM cloud_file_blobs WHERE id=$1",
        )
        .bind(file_uuid)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_error)?
        .ok_or_else(|| not_found("Attachment not found"))?;
        let storage_user_id: Uuid = row.try_get("user_id").map_err(internal)?;
        if storage_user_id != actor_uuid {
            let ledger_id: Option<String> = row.try_get("ledger_id").map_err(internal)?;
            let allowed = if let Some(ledger_id) = ledger_id {
                crate::beecount_collaboration::resolve_ledger_access(
                    &self.pool, actor_uuid, &ledger_id, false,
                )
                .await
                .is_ok_and(|access| access.storage_user_id == storage_user_id)
            } else {
                sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM beecount_ledger_members resource_owner \
                     JOIN beecount_ledger_members actor \
                       ON actor.ledger_id=resource_owner.ledger_id \
                     WHERE resource_owner.user_id=$1 AND actor.user_id=$2)",
                )
                .bind(storage_user_id)
                .bind(actor_uuid)
                .fetch_one(&self.pool)
                .await
                .map_err(db_error)?
            };
            if !allowed {
                return Err(not_found("Attachment not found"));
            }
        }
        Ok(AttachmentDownload {
            content: row.try_get("content").map_err(internal)?,
            mime_type: row.try_get("mime_type").map_err(internal)?,
            file_name: row.try_get("file_name").map_err(internal)?,
        })
    }
}

async fn find_existing(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    ledger_id: Option<&str>,
    kind: AttachmentKind,
    sha256: &str,
) -> Result<Option<AttachmentUploadOut>, ApiError> {
    let row = sqlx::query(
        "SELECT id::text,ledger_id,sha256,size_bytes,mime_type,file_name,created_at \
         FROM cloud_file_blobs WHERE user_id=$1 AND attachment_kind=$2 AND sha256=$3 \
           AND ledger_id IS NOT DISTINCT FROM $4 LIMIT 1",
    )
    .bind(user_id)
    .bind(kind.as_str())
    .bind(sha256)
    .bind(ledger_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db_error)?;
    row.map(upload_from_row).transpose()
}

fn upload_from_row(row: sqlx::postgres::PgRow) -> Result<AttachmentUploadOut, ApiError> {
    Ok(AttachmentUploadOut {
        file_id: row.try_get("id").map_err(internal)?,
        ledger_id: row
            .try_get::<Option<String>, _>("ledger_id")
            .map_err(internal)?
            .unwrap_or_default(),
        sha256: row.try_get("sha256").map_err(internal)?,
        size: row.try_get("size_bytes").map_err(internal)?,
        mime_type: row.try_get("mime_type").map_err(internal)?,
        file_name: row.try_get("file_name").map_err(internal)?,
        created_at: row.try_get("created_at").map_err(internal)?,
    })
}

#[allow(clippy::too_many_arguments)]
async fn persist_file_metadata(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    device_id: Uuid,
    external_device_id: &str,
    entity_id: &str,
    file_name: &str,
    mime_type: Option<&str>,
    size_bytes: i64,
    sha256: &str,
    now: DateTime<Utc>,
) -> Result<(), ApiError> {
    let payload = json!({
        "meta": {
            "id": entity_id,
            "userId": user_id.to_string(),
            "createdAt": now,
            "updatedAt": now,
            "deletedAt": null,
            "localVersion": 1,
            "serverVersion": "1",
            "modifiedByDevice": external_device_id,
        },
        "originalName": file_name,
        "mimeType": mime_type.unwrap_or("application/octet-stream"),
        "sizeBytes": size_bytes,
        "sha256": sha256,
        "storageState": "server_stored",
        "createdByDevice": external_device_id,
    });
    let payload_hash = hash_json(&payload)?;
    let cursor: i64 = sqlx::query_scalar(
        "INSERT INTO sync_change_log ( \
            user_id,entity_type,entity_id,operation,entity_schema_version,server_version, \
            payload,payload_hash,tombstone,origin_device_id,origin_device_external_id, \
            client_modified_at,server_modified_at \
         ) VALUES ($1,'file.metadata',$2,'upsert',1,1,$3,$4,NULL,$5,$6,$7,$7) \
         RETURNING cursor",
    )
    .bind(user_id)
    .bind(entity_id)
    .bind(&payload)
    .bind(&payload_hash)
    .bind(device_id)
    .bind(external_device_id)
    .bind(now)
    .fetch_one(&mut **tx)
    .await
    .map_err(db_error)?;
    sqlx::query(
        "INSERT INTO sync_entities ( \
            user_id,entity_type,entity_id,entity_schema_version,server_version,payload, \
            payload_hash,is_deleted,deleted_at,origin_device_id,origin_device_external_id, \
            created_at,server_modified_at,client_modified_at,last_cursor \
         ) VALUES ($1,'file.metadata',$2,1,1,$3,$4,FALSE,NULL,$5,$6,$7,$7,$7,$8)",
    )
    .bind(user_id)
    .bind(entity_id)
    .bind(payload)
    .bind(payload_hash)
    .bind(device_id)
    .bind(external_device_id)
    .bind(now)
    .bind(cursor)
    .execute(&mut **tx)
    .await
    .map_err(db_error)?;
    Ok(())
}

fn hash_json(value: &Value) -> Result<Vec<u8>, ApiError> {
    serde_json::to_vec(value)
        .map(|bytes| Sha256::digest(bytes).to_vec())
        .map_err(internal)
}

pub fn safe_file_name(raw: &str) -> String {
    let value = raw.rsplit(['/', '\\']).next().unwrap_or_default().trim();
    let mut output = value.chars().take(255).collect::<String>();
    if output.is_empty() || output == "." || output == ".." {
        output = "attachment.bin".to_owned();
    }
    output
}

fn clean_mime_type(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.len() <= 255)
        .map(str::to_owned)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn parse_uuid(value: &str, message: &'static str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(value).map_err(|_| internal(message))
}

fn invalid(message: impl Into<String>) -> ApiError {
    ApiError::new(ErrorCode::InvalidRequest, message, StatusCode::BAD_REQUEST)
}

fn not_found(message: impl Into<String>) -> ApiError {
    ApiError::new(ErrorCode::InvalidRequest, message, StatusCode::NOT_FOUND)
}

fn db_error(error: sqlx::Error) -> ApiError {
    ApiError::new(
        ErrorCode::TemporarilyUnavailable,
        format!("attachment database operation failed: {error}"),
        StatusCode::SERVICE_UNAVAILABLE,
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
    fn file_name_removes_client_paths_and_limits_length() {
        assert_eq!(safe_file_name("../../receipt.jpg"), "receipt.jpg");
        assert_eq!(safe_file_name(r"C:\\camera\\receipt.jpg"), "receipt.jpg");
        assert_eq!(safe_file_name(".."), "attachment.bin");
        assert_eq!(safe_file_name(&"a".repeat(300)).chars().count(), 255);
    }

    #[test]
    fn sha256_validation_is_strict() {
        assert!(valid_sha256(&"a".repeat(64)));
        assert!(!valid_sha256("not-a-hash"));
        assert!(!valid_sha256(&"g".repeat(64)));
    }
}
