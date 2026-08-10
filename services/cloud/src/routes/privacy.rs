//! EPIC-17 privacy export and account data-lifecycle endpoints.

use std::collections::BTreeMap;

use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get};
use axum::{Json, Router};
use lifetrace_contracts::registry::{EntityOwnership, REGISTRY};
use lifetrace_contracts::ErrorCode;
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

use crate::auth::scope;
use crate::auth::security::{clear_session_cookie, cookie_value};
use crate::auth::{AuthCredential, AuthenticatedPrincipal};
use crate::error::ApiError;
use crate::state::AppState;

const SUPPORTED_MODULES: &[&str] = &[
    "account",
    "devices",
    "sessions",
    "finance",
    "notes",
    "files",
    "english",
    "habits",
    "reviews",
    "workouts",
    "execution",
    "mail",
];

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/api/v1/privacy/export", get(export_all))
        .route("/api/v1/privacy/export/{module}", get(export_module))
        .route("/api/v1/privacy/policy", get(policy))
        .route("/api/v1/privacy/account", delete(delete_account))
}

fn api_error(code: ErrorCode, message: impl Into<String>, status: StatusCode) -> ApiError {
    ApiError::new(code, message, status)
}

fn db_error(error: sqlx::Error) -> ApiError {
    api_error(
        ErrorCode::TemporarilyUnavailable,
        format!("privacy database operation failed: {error}"),
        StatusCode::SERVICE_UNAVAILABLE,
    )
}

async fn read_principal(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AuthenticatedPrincipal, ApiError> {
    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    if authorization.is_some() {
        return state
            .auth
            .authenticate(AuthCredential::Bearer(authorization))
            .await;
    }
    let raw = cookie_value(headers, &state.config.auth_cookie_name);
    state
        .auth
        .authenticate(AuthCredential::WebSession(raw.as_deref()))
        .await
}

async fn write_principal(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<AuthenticatedPrincipal, ApiError> {
    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    if authorization.is_some() {
        return state
            .auth
            .authenticate(AuthCredential::Bearer(authorization))
            .await;
    }

    let raw_session = cookie_value(headers, &state.config.auth_cookie_name).unwrap_or_default();
    let csrf = headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let origin = headers.get("origin").and_then(|value| value.to_str().ok());
    state
        .auth_service
        .verify_web_csrf(&raw_session, csrf, origin)
        .await
}

fn module_for_entity(entity_type: &str) -> Option<&'static str> {
    scope::required_entity_scope(entity_type, false)
        .and_then(|value| value.split_once(':').map(|(module, _)| module))
}

fn module_read_scope(module: &str) -> Option<&'static str> {
    match module {
        "account" => Some("account:read"),
        "devices" => Some("devices:read"),
        "sessions" => Some("sessions:read"),
        "finance" => Some("finance:read"),
        "notes" => Some("notes:read"),
        "files" => Some("files:read"),
        "english" => Some("english:read"),
        "habits" => Some("habits:read"),
        "reviews" => Some("reviews:read"),
        "workouts" => Some("workouts:read"),
        "execution" => Some("execution:read"),
        "mail" => Some("mail:read"),
        _ => None,
    }
}

fn validate_module(module: &str) -> Result<(), ApiError> {
    if SUPPORTED_MODULES.contains(&module) {
        Ok(())
    } else {
        Err(api_error(
            ErrorCode::InvalidRequest,
            format!("unsupported export module: {module}"),
            StatusCode::BAD_REQUEST,
        ))
    }
}

fn database_user_id(principal: &AuthenticatedPrincipal) -> Option<Uuid> {
    Uuid::parse_str(principal.user_id.as_str()).ok()
}

async fn safe_account_profile(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
) -> Result<Value, ApiError> {
    if !state.database_enabled {
        return Ok(json!({"userId": principal.user_id.as_str()}));
    }
    let Some(user_id) = database_user_id(principal) else {
        return Ok(json!({"userId": principal.user_id.as_str()}));
    };
    let row = sqlx::query(
        "SELECT id,status,email,display_name,created_at,updated_at,email_verified_at,disabled_at,registration_source \
         FROM cloud_users WHERE id=$1",
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(db_error)?;
    let Some(row) = row else {
        return Ok(json!({"userId": principal.user_id.as_str()}));
    };
    Ok(json!({
        "userId": row.try_get::<Uuid, _>("id").ok().map(|value| value.to_string()),
        "status": row.try_get::<String, _>("status").ok(),
        "email": row.try_get::<Option<String>, _>("email").ok().flatten(),
        "displayName": row.try_get::<Option<String>, _>("display_name").ok().flatten(),
        "createdAt": row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at").ok(),
        "updatedAt": row.try_get::<chrono::DateTime<chrono::Utc>, _>("updated_at").ok(),
        "emailVerifiedAt": row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("email_verified_at").ok().flatten(),
        "disabledAt": row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("disabled_at").ok().flatten(),
        "registrationSource": row.try_get::<Option<String>, _>("registration_source").ok().flatten()
    }))
}

async fn jsonb_array(
    state: &AppState,
    sql: &str,
    user_id: Uuid,
) -> Result<Value, ApiError> {
    sqlx::query_scalar::<_, Value>(sql)
        .bind(user_id)
        .fetch_one(&state.pool)
        .await
        .map_err(db_error)
}

async fn database_section(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
    module: &str,
) -> Result<Option<Value>, ApiError> {
    if !state.database_enabled {
        return Ok(None);
    }
    let Some(user_id) = database_user_id(principal) else {
        return Ok(None);
    };
    match module {
        "devices" => Ok(Some(
            jsonb_array(
                state,
                "SELECT COALESCE(jsonb_agg(to_jsonb(d) ORDER BY d.first_seen_at), '[]'::jsonb) FROM cloud_devices d WHERE user_id=$1",
                user_id,
            )
            .await?,
        )),
        "sessions" => Ok(Some(
            jsonb_array(
                state,
                "SELECT COALESCE(jsonb_agg(to_jsonb(s) ORDER BY s.created_at), '[]'::jsonb) FROM auth_sessions s WHERE user_id=$1",
                user_id,
            )
            .await?,
        )),
        "mail" => {
            let accounts = jsonb_array(
                state,
                "SELECT COALESCE(jsonb_agg(to_jsonb(a) - ARRAY['credential_ciphertext','credential_nonce'] ORDER BY a.created_at), '[]'::jsonb) FROM mail_accounts a WHERE user_id=$1",
                user_id,
            )
            .await?;
            let messages = jsonb_array(
                state,
                "SELECT COALESCE(jsonb_agg(to_jsonb(m) ORDER BY m.received_at), '[]'::jsonb) FROM mail_messages m WHERE user_id=$1",
                user_id,
            )
            .await?;
            let attachments = jsonb_array(
                state,
                "SELECT COALESCE(jsonb_agg(to_jsonb(a) ORDER BY a.created_at), '[]'::jsonb) FROM mail_attachments a WHERE user_id=$1",
                user_id,
            )
            .await?;
            let drafts = jsonb_array(
                state,
                "SELECT COALESCE(jsonb_agg(to_jsonb(d) ORDER BY d.created_at), '[]'::jsonb) FROM mail_drafts d WHERE user_id=$1",
                user_id,
            )
            .await?;
            Ok(Some(json!({
                "accounts": accounts,
                "messages": messages,
                "attachments": attachments,
                "drafts": drafts
            })))
        }
        _ => Ok(None),
    }
}

async fn build_export(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
    requested_module: Option<&str>,
) -> Result<Value, ApiError> {
    if let Some(module) = requested_module {
        validate_module(module)?;
        if let Some(required) = module_read_scope(module) {
            principal.require_scope(required)?;
        }
    }

    let mut sections = BTreeMap::<String, Value>::new();
    for descriptor in REGISTRY {
        if descriptor.ownership != EntityOwnership::UserOwned {
            continue;
        }
        let Some(module) = module_for_entity(descriptor.entity_type) else {
            continue;
        };
        if requested_module.is_some_and(|requested| requested != module) {
            continue;
        }
        let Some(required) = module_read_scope(module) else {
            continue;
        };
        if !principal.scopes.contains(required) {
            continue;
        }
        let entities = state
            .store
            .list_entities(&principal.user_id, descriptor.entity_type)
            .await?;
        if entities.is_empty() {
            continue;
        }
        let value = serde_json::to_value(entities).map_err(|error| {
            api_error(
                ErrorCode::InternalError,
                format!("failed to serialize privacy export: {error}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;
        let section = sections
            .entry(module.to_owned())
            .or_insert_with(|| Value::Array(Vec::new()));
        if let (Value::Array(target), Value::Array(mut items)) = (section, value) {
            target.append(&mut items);
        }
    }

    let modules: Vec<&str> = if let Some(module) = requested_module {
        vec![module]
    } else {
        SUPPORTED_MODULES
            .iter()
            .copied()
            .filter(|module| {
                module_read_scope(module).is_some_and(|required| principal.scopes.contains(required))
            })
            .collect()
    };

    for module in modules {
        if module == "account" {
            sections.insert(
                "account".to_owned(),
                safe_account_profile(state, principal).await?,
            );
        }
        if let Some(value) = database_section(state, principal, module).await? {
            sections.insert(module.to_owned(), value);
        }
    }

    Ok(json!({
        "format": "lifetrace-privacy-export-v1",
        "exportedAt": chrono::Utc::now(),
        "userId": principal.user_id.as_str(),
        "requestedModule": requested_module,
        "sections": sections
    }))
}

async fn export_all(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let principal = read_principal(&state, &headers).await?;
    principal.require_scope("account:read")?;
    build_export(&state, &principal, None).await.map(Json)
}

async fn export_module(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(module): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let principal = read_principal(&state, &headers).await?;
    build_export(&state, &principal, Some(module.as_str()))
        .await
        .map(Json)
}

async fn policy(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let principal = read_principal(&state, &headers).await?;
    principal.require_scope("account:read")?;
    Ok(Json(json!({
        "policyVersion": 1,
        "onlinePrimaryData": "retained while the account is active; deleted by the account deletion workflow",
        "sessionsAndTokens": "revoked or deleted when the owning account is deleted",
        "mailRawContent": "retained while mail aggregation is enabled and deleted with the owning account",
        "importFiles": "raw import uploads are device-local unless a domain explicitly opts into cloud storage",
        "fileObjects": "the current cloud service stores metadata only; any non-null external storage reference blocks account deletion until an object cleanup provider is configured",
        "backupDeletion": "logical deletion is immediate; encrypted backup copies age out under the deployment backup-retention window and must be re-deleted if restored",
        "diagnosticLogs": "authentication secrets must be redacted before structured diagnostic metadata is written",
        "environment": state.config.environment
    })))
}

async fn delete_account(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let principal = write_principal(&state, &headers).await?;
    principal.require_scope("account:write")?;

    if !state.database_enabled {
        return Err(api_error(
            ErrorCode::TemporarilyUnavailable,
            "account deletion requires the persistent PostgreSQL cloud runtime",
            StatusCode::SERVICE_UNAVAILABLE,
        ));
    }
    let user_id = database_user_id(&principal).ok_or_else(|| {
        api_error(
            ErrorCode::InvalidRequest,
            "account id is not a persistent cloud UUID",
            StatusCode::BAD_REQUEST,
        )
    })?;

    // The current server has no object-byte store. If a mail attachment already
    // points at external storage, fail closed instead of claiming the object was
    // erased when no cleanup provider is available.
    let has_external_objects: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM mail_attachments WHERE user_id=$1 AND storage_ref IS NOT NULL)",
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(db_error)?;
    if has_external_objects {
        return Err(api_error(
            ErrorCode::TemporarilyUnavailable,
            "external file objects must be deleted before account deletion can complete",
            StatusCode::SERVICE_UNAVAILABLE,
        ));
    }

    let mut tx = state.pool.begin().await.map_err(db_error)?;
    sqlx::query(
        "UPDATE auth_sessions SET status='revoked',revoked_at=now(),revoked_reason='account_deleted' \
         WHERE user_id=$1 AND status='active'",
    )
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(db_error)?;

    let deleted = sqlx::query("DELETE FROM cloud_users WHERE id=$1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;
    if deleted.rows_affected() != 1 {
        return Err(api_error(
            ErrorCode::InvalidRequest,
            "account no longer exists",
            StatusCode::NOT_FOUND,
        ));
    }
    tx.commit().await.map_err(db_error)?;

    let mut response = StatusCode::NO_CONTENT.into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, clear_session_cookie(&state.config));
    Ok(response)
}
