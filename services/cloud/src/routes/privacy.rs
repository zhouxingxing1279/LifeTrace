//! EPIC-17 privacy export, retention policy and account lifecycle endpoints.

use std::collections::BTreeSet;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get};
use axum::{Json, Router};
use chrono::Utc;
use lifetrace_contracts::ErrorCode;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use uuid::Uuid;

use crate::auth::AuthenticatedPrincipal;
use crate::error::ApiError;
use crate::state::AppState;

const EXPORT_VERSION: u32 = 1;
const DELETE_CONFIRMATION: &str = "DELETE MY ACCOUNT";
const ALL_MODULES: [&str; 5] = ["account", "devices", "grants", "sync", "mail"];

#[derive(Debug, Deserialize, Default)]
struct ExportQuery {
    modules: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeleteAccountRequest {
    confirmation: String,
}

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/api/v1/privacy/policy", get(policy))
        .route("/api/v1/privacy/export", get(export))
        .route("/api/v1/privacy/account", delete(delete_account))
}

async fn policy(
    principal: AuthenticatedPrincipal,
) -> Result<Json<Value>, ApiError> {
    // Authentication is intentionally required even though the policy itself
    // is non-secret. This keeps the whole privacy surface consistently
    // protected and covered by the anonymous-access regression matrix.
    let _ = principal;
    Ok(Json(retention_policy()))
}

async fn export(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Query(query): Query<ExportQuery>,
) -> Result<Json<Value>, ApiError> {
    require_database(&state)?;
    let user_id = principal_user_uuid(&principal)?;
    let modules = parse_modules(query.modules.as_deref())?;
    let mut data = Map::new();

    if modules.contains("account") {
        let account = sqlx::query_scalar::<_, Value>(
            r#"
            SELECT jsonb_build_object(
                'id', id::text,
                'email', email,
                'displayName', display_name,
                'status', status,
                'authState', auth_state,
                'emailVerifiedAt', email_verified_at,
                'passwordChangedAt', password_changed_at,
                'registrationSource', registration_source,
                'createdAt', created_at,
                'updatedAt', updated_at
            )
            FROM cloud_users
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(db_error)?
        .ok_or_else(|| {
            ApiError::new(
                ErrorCode::AuthInvalid,
                "authenticated user no longer exists",
                StatusCode::UNAUTHORIZED,
            )
        })?;
        data.insert("account".to_owned(), account);
    }

    if modules.contains("devices") {
        let devices = sqlx::query_scalar::<_, Value>(
            r#"
            SELECT COALESCE(jsonb_agg(jsonb_build_object(
                'id', id::text,
                'appId', app_id,
                'platform', platform,
                'clientVersion', client_version,
                'protocolVersion', protocol_version,
                'schemaVersion', schema_version,
                'status', status,
                'deviceGroupId', device_group_id,
                'deviceName', device_name,
                'firstSeenAt', first_seen_at,
                'lastSeenAt', last_seen_at,
                'lastSyncAt', last_sync_at,
                'lastLoginAt', last_login_at,
                'revokedAt', revoked_at,
                'revokedReason', revoked_reason
            ) ORDER BY first_seen_at), '[]'::jsonb)
            FROM cloud_devices
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&state.pool)
        .await
        .map_err(db_error)?;
        data.insert("devices".to_owned(), devices);
    }

    if modules.contains("grants") {
        let grants = sqlx::query_scalar::<_, Value>(
            r#"
            SELECT COALESCE(jsonb_agg(jsonb_build_object(
                'id', id::text,
                'appId', app_id,
                'scopes', scopes,
                'status', status,
                'grantedAt', granted_at,
                'revokedAt', revoked_at,
                'updatedAt', updated_at
            ) ORDER BY granted_at), '[]'::jsonb)
            FROM auth_app_grants
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&state.pool)
        .await
        .map_err(db_error)?;
        data.insert("grants".to_owned(), grants);
    }

    if modules.contains("sync") {
        let entities = sqlx::query_scalar::<_, Value>(
            r#"
            SELECT COALESCE(jsonb_agg(jsonb_build_object(
                'entityType', entity_type,
                'entityId', entity_id,
                'entitySchemaVersion', entity_schema_version,
                'serverVersion', server_version,
                'payload', payload,
                'isDeleted', is_deleted,
                'deletedAt', deleted_at,
                'createdAt', created_at,
                'serverModifiedAt', server_modified_at,
                'clientModifiedAt', client_modified_at,
                'lastCursor', last_cursor
            ) ORDER BY entity_type, entity_id), '[]'::jsonb)
            FROM sync_entities
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&state.pool)
        .await
        .map_err(db_error)?;
        data.insert("sync".to_owned(), json!({ "entities": entities }));
    }

    if modules.contains("mail") {
        let mail = sqlx::query_scalar::<_, Value>(
            r#"
            SELECT jsonb_build_object(
                'accounts', (
                    SELECT COALESCE(jsonb_agg(jsonb_build_object(
                        'id', id::text,
                        'provider', provider,
                        'emailAddress', email_address,
                        'displayName', display_name,
                        'imapHost', imap_host,
                        'imapPort', imap_port,
                        'imapSecurity', imap_security,
                        'smtpHost', smtp_host,
                        'smtpPort', smtp_port,
                        'smtpSecurity', smtp_security,
                        'username', username,
                        'status', status,
                        'idleSupported', idle_supported,
                        'lastValidatedAt', last_validated_at,
                        'lastSyncAt', last_sync_at,
                        'lastErrorCode', last_error_code,
                        'createdAt', created_at,
                        'updatedAt', updated_at,
                        'deletedAt', deleted_at
                    ) ORDER BY created_at), '[]'::jsonb)
                    FROM mail_accounts WHERE user_id = $1
                ),
                'folders', (
                    SELECT COALESCE(jsonb_agg(jsonb_build_object(
                        'id', id::text,
                        'accountId', account_id::text,
                        'remoteName', remote_name,
                        'normalizedRole', normalized_role,
                        'uidvalidity', uidvalidity,
                        'uidnext', uidnext,
                        'highestModseq', highest_modseq,
                        'lastSeenUid', last_seen_uid,
                        'lastSyncAt', last_sync_at,
                        'syncEnabled', sync_enabled,
                        'createdAt', created_at,
                        'updatedAt', updated_at
                    ) ORDER BY created_at), '[]'::jsonb)
                    FROM mail_folders WHERE user_id = $1
                ),
                'threads', (
                    SELECT COALESCE(jsonb_agg(jsonb_build_object(
                        'id', id::text,
                        'accountId', account_id::text,
                        'normalizedSubject', normalized_subject,
                        'latestMessageAt', latest_message_at,
                        'messageCount', message_count,
                        'unreadCount', unread_count,
                        'participantSummary', participant_summary,
                        'snippet', snippet,
                        'createdAt', created_at,
                        'updatedAt', updated_at
                    ) ORDER BY latest_message_at), '[]'::jsonb)
                    FROM mail_threads WHERE user_id = $1
                ),
                'messages', (
                    SELECT COALESCE(jsonb_agg(jsonb_build_object(
                        'id', id::text,
                        'accountId', account_id::text,
                        'folderId', folder_id::text,
                        'threadId', thread_id::text,
                        'remoteUid', remote_uid,
                        'uidvalidity', uidvalidity,
                        'messageId', message_id,
                        'inReplyTo', in_reply_to,
                        'references', references_json,
                        'subject', subject,
                        'from', from_json,
                        'to', to_json,
                        'cc', cc_json,
                        'bcc', bcc_json,
                        'replyTo', reply_to_json,
                        'sentAt', sent_at,
                        'receivedAt', received_at,
                        'flags', flags_json,
                        'isRead', is_read,
                        'isArchived', is_archived,
                        'sizeBytes', size_bytes,
                        'snippet', snippet,
                        'bodyText', body_text,
                        'bodyHtmlSanitized', body_html_sanitized,
                        'hasAttachments', has_attachments,
                        'createdAt', created_at,
                        'updatedAt', updated_at
                    ) ORDER BY received_at), '[]'::jsonb)
                    FROM mail_messages WHERE user_id = $1
                ),
                'attachments', (
                    SELECT COALESCE(jsonb_agg(jsonb_build_object(
                        'id', id::text,
                        'messageId', message_id::text,
                        'partId', part_id,
                        'filename', filename,
                        'mimeType', mime_type,
                        'sizeBytes', size_bytes,
                        'contentId', content_id,
                        'disposition', disposition,
                        'checksum', checksum,
                        'storageRef', storage_ref,
                        'downloadState', download_state,
                        'createdAt', created_at,
                        'updatedAt', updated_at
                    ) ORDER BY created_at), '[]'::jsonb)
                    FROM mail_attachments WHERE user_id = $1
                ),
                'drafts', (
                    SELECT COALESCE(jsonb_agg(jsonb_build_object(
                        'id', id::text,
                        'accountId', account_id::text,
                        'threadId', thread_id::text,
                        'inReplyToMessageId', in_reply_to_message_id::text,
                        'to', to_json,
                        'cc', cc_json,
                        'bcc', bcc_json,
                        'subject', subject,
                        'bodyText', body_text,
                        'state', state,
                        'createdAt', created_at,
                        'updatedAt', updated_at
                    ) ORDER BY created_at), '[]'::jsonb)
                    FROM mail_drafts WHERE user_id = $1
                ),
                'outbox', (
                    SELECT COALESCE(jsonb_agg(jsonb_build_object(
                        'id', id::text,
                        'accountId', account_id::text,
                        'draftId', draft_id::text,
                        'state', state,
                        'generatedMessageId', generated_message_id,
                        'attempt', attempt,
                        'nextRetryAt', next_retry_at,
                        'lastErrorCode', last_error_code,
                        'sentAt', sent_at,
                        'createdAt', created_at,
                        'updatedAt', updated_at
                    ) ORDER BY created_at), '[]'::jsonb)
                    FROM mail_outbox WHERE user_id = $1
                )
            )
            "#,
        )
        .bind(user_id)
        .fetch_one(&state.pool)
        .await
        .map_err(db_error)?;
        data.insert("mail".to_owned(), mail);
    }

    Ok(Json(json!({
        "schema": "lifetrace.user-data-export",
        "version": EXPORT_VERSION,
        "exportedAt": Utc::now(),
        "userId": principal.user_id.as_str(),
        "modules": modules.into_iter().collect::<Vec<_>>(),
        "data": data,
        "retentionPolicy": retention_policy(),
        "secretsExcluded": [
            "passwordHash",
            "accessToken",
            "refreshToken",
            "sessionSecret",
            "credentialCiphertext",
            "credentialNonce",
            "serverSecrets"
        ]
    })))
}

async fn delete_account(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(request): Json<DeleteAccountRequest>,
) -> Result<Json<Value>, ApiError> {
    require_database(&state)?;
    if request.confirmation != DELETE_CONFIRMATION {
        return Err(ApiError::new(
            ErrorCode::InvalidRequest,
            format!("confirmation must equal {DELETE_CONFIRMATION:?}"),
            StatusCode::BAD_REQUEST,
        ));
    }

    let user_id = principal_user_uuid(&principal)?;
    let mut tx = state.pool.begin().await.map_err(db_error)?;

    // EPIC-12 object storage is not yet a production dependency. If metadata
    // already points at an external object, fail closed instead of deleting
    // the database row and falsely claiming the object was erased.
    let external_object_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)::bigint
        FROM mail_attachments
        WHERE user_id = $1
          AND storage_ref IS NOT NULL
          AND btrim(storage_ref) <> ''
        "#,
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(db_error)?;

    if external_object_count > 0 {
        tx.rollback().await.map_err(db_error)?;
        return Err(ApiError::new(
            ErrorCode::TemporarilyUnavailable,
            "account has externally stored objects; object deletion integration must succeed before account deletion can continue",
            StatusCode::CONFLICT,
        ));
    }

    // Keep only the minimum security audit record: null the user/session/device
    // linkage before deleting the identity anchor. Business data is removed by
    // the FK cascade rooted at cloud_users.
    sqlx::query(
        r#"
        UPDATE auth_audit_log
        SET user_id = NULL, session_id = NULL, device_id = NULL,
            metadata = jsonb_build_object('accountDeleted', true)
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(db_error)?;

    let deleted = sqlx::query("DELETE FROM cloud_users WHERE id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;

    if deleted.rows_affected() != 1 {
        tx.rollback().await.map_err(db_error)?;
        return Err(ApiError::new(
            ErrorCode::AuthInvalid,
            "authenticated user no longer exists",
            StatusCode::UNAUTHORIZED,
        ));
    }

    tx.commit().await.map_err(db_error)?;

    Ok(Json(json!({
        "deleted": true,
        "onlineBusinessDataDeleted": true,
        "sessionsRevoked": true,
        "externalObjectsDeleted": true,
        "externalObjectCount": 0,
        "backupPolicy": "historical encrypted backups are not modified in place and expire according to the documented retention window"
    })))
}

fn parse_modules(raw: Option<&str>) -> Result<BTreeSet<&'static str>, ApiError> {
    if raw.map(str::trim).filter(|value| !value.is_empty()).is_none() {
        return Ok(ALL_MODULES.into_iter().collect());
    }

    let requested: BTreeSet<String> = raw
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
        .collect();

    let unknown: Vec<String> = requested
        .iter()
        .filter(|value| !ALL_MODULES.contains(&value.as_str()))
        .cloned()
        .collect();
    if !unknown.is_empty() {
        return Err(ApiError::new(
            ErrorCode::InvalidRequest,
            format!("unknown export modules: {}", unknown.join(", ")),
            StatusCode::BAD_REQUEST,
        ));
    }

    Ok(ALL_MODULES
        .into_iter()
        .filter(|module| requested.contains(*module))
        .collect())
}

fn principal_user_uuid(principal: &AuthenticatedPrincipal) -> Result<Uuid, ApiError> {
    Uuid::parse_str(principal.user_id.as_str()).map_err(|_| {
        ApiError::new(
            ErrorCode::AuthInvalid,
            "authenticated principal has an invalid cloud user id",
            StatusCode::UNAUTHORIZED,
        )
    })
}

fn require_database(state: &AppState) -> Result<(), ApiError> {
    if state.database_enabled {
        Ok(())
    } else {
        Err(ApiError::new(
            ErrorCode::TemporarilyUnavailable,
            "privacy lifecycle operations require the PostgreSQL cloud backend",
            StatusCode::SERVICE_UNAVAILABLE,
        ))
    }
}

fn db_error(error: sqlx::Error) -> ApiError {
    // Never echo SQL text, credentials or row contents to the client.
    eprintln!("privacy database operation failed: {}", database_error_kind(&error));
    ApiError::new(
        ErrorCode::InternalError,
        "privacy database operation failed",
        StatusCode::INTERNAL_SERVER_ERROR,
    )
}

fn database_error_kind(error: &sqlx::Error) -> &'static str {
    match error {
        sqlx::Error::RowNotFound => "row_not_found",
        sqlx::Error::PoolTimedOut => "pool_timed_out",
        sqlx::Error::PoolClosed => "pool_closed",
        sqlx::Error::Io(_) => "io",
        sqlx::Error::Tls(_) => "tls",
        sqlx::Error::Database(_) => "database",
        _ => "other",
    }
}

fn retention_policy() -> Value {
    json!({
        "version": 1,
        "onlineBusinessData": {
            "retention": "while account is active or until the user deletes the data",
            "accountDeletion": "online business rows are deleted transactionally through user-scoped foreign-key cascades"
        },
        "sessionsAndTokens": {
            "retention": "active only for their configured lifetime",
            "accountDeletion": "sessions and token records are deleted with the account and become unusable immediately"
        },
        "notificationRawText": {
            "retention": "minimum necessary for the user-facing feature; never written to ordinary diagnostic logs",
            "defaultRule": "prefer derived fields and discard raw text when the owning feature no longer needs it"
        },
        "importFiles": {
            "retention": "user-visible and user-deletable; no undocumented indefinite retention",
            "accountDeletion": "metadata is removed with account data; external object deletion must succeed before account deletion reports success"
        },
        "diagnosticLogs": {
            "retention": "bounded by EPIC-19 rotation and retention configuration",
            "content": "tokens, passwords, cookies, credentials and complete sensitive bodies are prohibited"
        },
        "securityAudit": {
            "retention": "minimum non-content security event metadata may be retained after account deletion",
            "accountDeletion": "user/session/device identifiers are detached and metadata is reduced to an account-deleted marker"
        },
        "backups": {
            "retention": "historical encrypted backups are immutable within their configured retention window and then expire",
            "accountDeletion": "online deletion is immediate; historical backup copies are not rewritten in place"
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_parser_defaults_to_complete_export() {
        let modules = parse_modules(None).unwrap();
        assert_eq!(modules.len(), ALL_MODULES.len());
        for module in ALL_MODULES {
            assert!(modules.contains(module));
        }
    }

    #[test]
    fn module_parser_rejects_unknown_modules() {
        let error = parse_modules(Some("account,secrets")).unwrap_err();
        assert_eq!(error.http_status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn retention_policy_prohibits_sensitive_diagnostic_content() {
        let policy = retention_policy();
        let content = policy["diagnosticLogs"]["content"].as_str().unwrap();
        for term in ["tokens", "passwords", "cookies", "credentials"] {
            assert!(content.contains(term));
        }
    }
}
