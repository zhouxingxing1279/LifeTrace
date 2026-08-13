//! BeeCount wire-compatible endpoints mounted behind an internal namespace.
//!
//! Caddy will rewrite the stock BeeCount paths to these routes only during the
//! Phase 3 cutover. Keeping the namespace private avoids colliding with
//! LifeTrace's camelCase `/api/v1/auth/*` contract while both services coexist.

use std::collections::BTreeSet;

use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use lifetrace_contracts::auth::v1::{LoginRequestV1, RegisterRequestV1, TokenResponseV1};
use lifetrace_contracts::sync::v1::AppId;
use lifetrace_contracts::ErrorCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::security::{PeerAddr, RequestContext};
use crate::auth::AuthenticatedPrincipal;
use crate::beecount_compat::{
    BeeCountEntityKind, BeeCountReadLedgerOut, BeeCountScope, BeeCountSyncFullResponse,
    BeeCountSyncLedgerOut, BeeCountSyncPullResponse, BeeCountSyncPushRequest,
    BeeCountSyncPushResponse, USER_GLOBAL_LEDGER_SENTINEL,
};
use crate::beecount_sync::BeeCountSyncService;
use crate::error::ApiError;
use crate::state::AppState;

const PREFIX: &str = "/api/v1/integrations/beecount/compat";

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route(&format!("{PREFIX}/auth/register"), post(register))
        .route(&format!("{PREFIX}/auth/login"), post(login))
        .route(&format!("{PREFIX}/auth/refresh"), post(refresh))
        .route(&format!("{PREFIX}/auth/logout"), post(logout))
        .route(&format!("{PREFIX}/auth/2fa/status"), get(two_factor_status))
        .route(&format!("{PREFIX}/sync/push"), post(sync_push))
        .route(&format!("{PREFIX}/sync/pull"), get(sync_pull))
        .route(&format!("{PREFIX}/sync/ledgers"), get(sync_ledgers))
        .route(&format!("{PREFIX}/sync/full"), get(sync_full))
        .route(&format!("{PREFIX}/version"), get(version))
        .route(&format!("{PREFIX}/read/ledgers"), get(read_ledgers))
}

/// Stock BeeCount clients probe `GET /api/v1/version` on startup; mirror the
/// legacy BeeCount Cloud response so the compatibility check passes.
async fn version() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": "BeeCount Cloud",
        "version": "1.6.3",
    }))
}

/// Read-namespace ledger listing used by stock BeeCount clients
/// (`GET /api/v1/read/ledgers`).
async fn read_ledgers(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<Json<Vec<BeeCountReadLedgerOut>>, ApiError> {
    ensure_beecount_principal(&principal)?;
    principal.require_scope("sync:read")?;
    BeeCountSyncService::new(state.pool.clone())
        .read_ledgers(&principal.user_id)
        .await
        .map(Json)
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct BeeCountCredentials {
    email: String,
    password: String,
    device_id: String,
    device_name: String,
    platform: String,
    #[serde(default)]
    app_version: Option<String>,
    #[serde(default)]
    os_version: Option<String>,
    #[serde(default)]
    device_model: Option<String>,
    #[serde(default)]
    client_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BeeCountRefreshRequest {
    refresh_token: String,
}

#[derive(Debug, Deserialize)]
struct BeeCountLogoutRequest {
    #[allow(dead_code)]
    refresh_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BeeCountPullQuery {
    #[serde(default)]
    since: i64,
    device_id: Option<String>,
    #[serde(default = "default_pull_limit")]
    limit: i64,
}

#[derive(Debug, Deserialize)]
struct BeeCountFullQuery {
    ledger_id: String,
}

fn default_pull_limit() -> i64 {
    1000
}

#[derive(Debug, Serialize)]
struct BeeCountAuthUser {
    id: String,
    email: String,
    display_name: Option<String>,
}

#[derive(Debug, Serialize)]
struct BeeCountAuthResponse {
    access_token: String,
    refresh_token: String,
    token_type: String,
    expires_in: u64,
    user: BeeCountAuthUser,
    device_id: String,
}

#[derive(Debug, Serialize)]
struct BeeCountAcceptedResponse {
    accepted: bool,
}

#[derive(Debug, Serialize)]
struct BeeCountTwoFactorStatus {
    enabled: bool,
    enabled_at: Option<String>,
}

fn context(state: &AppState, headers: &HeaderMap, peer: PeerAddr) -> RequestContext {
    RequestContext::from_headers(headers, peer.0, &state.config)
}

fn requested_scopes() -> Vec<lifetrace_contracts::auth::v1::Scope> {
    crate::auth::scope::default_scopes(AppId::BEECOUNT)
        .into_iter()
        .map(lifetrace_contracts::auth::v1::Scope::new)
        .collect()
}

fn ensure_beecount_principal(principal: &AuthenticatedPrincipal) -> Result<(), ApiError> {
    if principal.app_id.as_str() != AppId::BEECOUNT {
        return Err(ApiError::new(
            ErrorCode::AuthInvalid,
            "BeeCount session required",
            StatusCode::UNAUTHORIZED,
        ));
    }
    Ok(())
}

fn auth_response(
    response: TokenResponseV1,
    device_id: String,
    beecount_user_id: String,
) -> Result<BeeCountAuthResponse, ApiError> {
    let refresh_token = response.refresh_token.ok_or_else(|| {
        ApiError::new(
            ErrorCode::InternalError,
            "BeeCount sessions require refresh tokens",
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;
    Ok(BeeCountAuthResponse {
        access_token: response.access_token,
        refresh_token,
        token_type: response.token_type,
        expires_in: response.expires_in,
        user: BeeCountAuthUser {
            id: beecount_user_id,
            email: response.user.email,
            display_name: response.user.display_name,
        },
        device_id,
    })
}

async fn ensure_identity_link(
    state: &AppState,
    response: &TokenResponseV1,
) -> Result<String, ApiError> {
    let user_id = Uuid::parse_str(response.user.id.as_str()).map_err(|_| {
        ApiError::new(
            ErrorCode::InternalError,
            "invalid LifeTrace user id",
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;
    let beecount_user_id = sqlx::query_scalar::<_, String>(
        "INSERT INTO beecount_identity_links \
         (user_id,beecount_user_id,source_email_normalized,source_kind) \
         VALUES ($1,$2,$3,'native') \
         ON CONFLICT (user_id) DO UPDATE SET \
         source_email_normalized=EXCLUDED.source_email_normalized \
         RETURNING beecount_user_id",
    )
    .bind(user_id)
    .bind(response.user.id.as_str())
    .bind(response.user.email.trim().to_lowercase())
    .fetch_one(&state.pool)
    .await
    .map_err(|_error| {
        ApiError::new(
            ErrorCode::InternalError,
            "BeeCount identity link failed",
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;
    Ok(beecount_user_id)
}

async fn register(
    State(state): State<AppState>,
    peer: PeerAddr,
    headers: HeaderMap,
    Json(request): Json<BeeCountCredentials>,
) -> Result<(StatusCode, Json<BeeCountAuthResponse>), ApiError> {
    let device_id = request.device_id.clone();
    let os_version = request.os_version.clone();
    let device_model = request.device_model.clone();
    let response = state
        .auth_service
        .register(
            RegisterRequestV1 {
                email: request.email,
                password: request.password,
                display_name: None,
                invite_token: None,
                app_id: AppId::new(AppId::BEECOUNT),
                device_id: request.device_id,
                device_name: request.device_name,
                platform: request.platform,
                client_version: request.app_version,
                requested_scopes: requested_scopes(),
            },
            &context(&state, &headers, peer),
        )
        .await?;
    persist_device_details(&state, &response, os_version, device_model).await?;
    let beecount_user_id = ensure_identity_link(&state, &response).await?;
    Ok((
        StatusCode::CREATED,
        Json(auth_response(response, device_id, beecount_user_id)?),
    ))
}

async fn login(
    State(state): State<AppState>,
    peer: PeerAddr,
    headers: HeaderMap,
    Json(request): Json<BeeCountCredentials>,
) -> Result<Json<BeeCountAuthResponse>, ApiError> {
    let device_id = request.device_id.clone();
    let os_version = request.os_version.clone();
    let device_model = request.device_model.clone();
    let response = state
        .auth_service
        .login(
            LoginRequestV1 {
                email: request.email,
                password: request.password,
                app_id: AppId::new(AppId::BEECOUNT),
                device_id: request.device_id,
                device_name: request.device_name,
                platform: request.platform,
                client_version: request.app_version,
                requested_scopes: requested_scopes(),
                public_device: false,
            },
            &context(&state, &headers, peer),
        )
        .await?;
    persist_device_details(&state, &response, os_version, device_model).await?;
    let beecount_user_id = ensure_identity_link(&state, &response).await?;
    Ok(Json(auth_response(response, device_id, beecount_user_id)?))
}

async fn persist_device_details(
    state: &AppState,
    response: &TokenResponseV1,
    os_version: Option<String>,
    device_model: Option<String>,
) -> Result<(), ApiError> {
    let device_id = Uuid::parse_str(response.session.device_id.as_str()).map_err(|_| {
        ApiError::new(
            ErrorCode::InternalError,
            "invalid LifeTrace device id",
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;
    sqlx::query(
        "UPDATE cloud_devices SET os_version=COALESCE($2,os_version), \
         device_model=COALESCE($3,device_model) WHERE id=$1",
    )
        .bind(device_id)
        .bind(
            os_version
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
        .bind(
            device_model
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty()),
        )
        .execute(&state.pool)
        .await
        .map_err(|_| {
            ApiError::new(
                ErrorCode::TemporarilyUnavailable,
                "BeeCount device metadata temporarily unavailable",
                StatusCode::SERVICE_UNAVAILABLE,
            )
        })?;
    Ok(())
}

async fn refresh(
    State(state): State<AppState>,
    peer: PeerAddr,
    headers: HeaderMap,
    Json(request): Json<BeeCountRefreshRequest>,
) -> Result<Json<BeeCountAuthResponse>, ApiError> {
    let (response, device_id, beecount_user_id) = state
        .auth_service
        .refresh_compat(
            request.refresh_token,
            AppId::BEECOUNT,
            &context(&state, &headers, peer),
        )
        .await?;
    Ok(Json(auth_response(response, device_id, beecount_user_id)?))
}

async fn logout(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    peer: PeerAddr,
    headers: HeaderMap,
    Json(_request): Json<BeeCountLogoutRequest>,
) -> Result<Json<BeeCountAcceptedResponse>, ApiError> {
    ensure_beecount_principal(&principal)?;
    state
        .auth_service
        .logout(&principal, &context(&state, &headers, peer))
        .await?;
    Ok(Json(BeeCountAcceptedResponse { accepted: true }))
}

async fn two_factor_status(
    principal: AuthenticatedPrincipal,
) -> Result<Json<BeeCountTwoFactorStatus>, ApiError> {
    ensure_beecount_principal(&principal)?;
    Ok(Json(BeeCountTwoFactorStatus {
        enabled: false,
        enabled_at: None,
    }))
}

async fn sync_push(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(request): Json<BeeCountSyncPushRequest>,
) -> Result<Json<BeeCountSyncPushResponse>, ApiError> {
    ensure_beecount_principal(&principal)?;
    principal.require_scope("sync:write")?;
    let touched_ledgers = request
        .changes
        .iter()
        .filter_map(|change| {
            let kind = BeeCountEntityKind::parse(&change.entity_type)?;
            match kind.scope() {
                BeeCountScope::User => Some(USER_GLOBAL_LEDGER_SENTINEL.to_owned()),
                BeeCountScope::Ledger => change.ledger_id.clone().or_else(|| {
                    (kind == BeeCountEntityKind::Ledger).then(|| change.entity_sync_id.clone())
                }),
            }
        })
        .collect::<BTreeSet<_>>();
    let shared_resource_events = request
        .changes
        .iter()
        .filter(|change| matches!(change.entity_type.as_str(), "category" | "account" | "tag"))
        .map(|change| {
            (
                change.entity_type.clone(),
                change.action.clone(),
                if change.action == "delete" {
                    serde_json::json!({"sync_id":change.entity_sync_id})
                } else {
                    change.payload.clone()
                },
            )
        })
        .collect::<Vec<_>>();
    let response = BeeCountSyncService::new(state.pool.clone())
        .push(&principal.user_id, principal.device_id.as_str(), request)
        .await?;
    if response.accepted > 0 {
        for ledger_id in touched_ledgers {
            let users = crate::beecount_collaboration::member_user_ids(&state.pool, &ledger_id)
                .await
                .unwrap_or_else(|_| Vec::new());
            if users.is_empty() {
                state.beecount_realtime.publish_sync_change(
                    &principal.user_id,
                    &ledger_id,
                    response.server_cursor,
                    response.server_timestamp,
                );
            } else {
                for user_id in users {
                    state.beecount_realtime.publish(
                        &user_id.to_string(),
                        serde_json::json!({
                            "type":"sync_change","ledgerId":ledger_id,
                            "serverCursor":response.server_cursor,
                            "serverTimestamp":response.server_timestamp,
                        }),
                    );
                }
            }
        }
        if !shared_resource_events.is_empty() {
            if let Ok(owner_uuid) = Uuid::parse_str(principal.user_id.as_str()) {
                if let Ok(editors) =
                    crate::beecount_collaboration::editor_members_for_owner(&state.pool, owner_uuid)
                        .await
                {
                    for (ledger_id, editor_user_id) in editors {
                        for (resource_type, action, payload) in &shared_resource_events {
                            state.beecount_realtime.publish(
                                &editor_user_id.to_string(),
                                serde_json::json!({
                                    "type":"shared_resource_change","ledgerId":ledger_id,
                                    "resourceType":resource_type,"action":action,"payload":payload,
                                }),
                            );
                        }
                    }
                }
            }
        }
    }
    Ok(Json(response))
}

async fn sync_pull(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Query(query): Query<BeeCountPullQuery>,
) -> Result<Json<BeeCountSyncPullResponse>, ApiError> {
    ensure_beecount_principal(&principal)?;
    principal.require_scope("sync:read")?;
    BeeCountSyncService::new(state.pool.clone())
        .pull(
            &principal.user_id,
            query.since,
            query.device_id.as_deref(),
            query.limit,
        )
        .await
        .map(Json)
}

async fn sync_ledgers(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<Json<Vec<BeeCountSyncLedgerOut>>, ApiError> {
    ensure_beecount_principal(&principal)?;
    principal.require_scope("sync:read")?;
    BeeCountSyncService::new(state.pool.clone())
        .ledgers(&principal.user_id)
        .await
        .map(Json)
}

async fn sync_full(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Query(query): Query<BeeCountFullQuery>,
) -> Result<Json<BeeCountSyncFullResponse>, ApiError> {
    ensure_beecount_principal(&principal)?;
    principal.require_scope("sync:read")?;
    if query.ledger_id.is_empty() || query.ledger_id.len() > 256 {
        return Err(ApiError::new(
            ErrorCode::InvalidRequest,
            "invalid BeeCount ledger id",
            StatusCode::BAD_REQUEST,
        ));
    }
    BeeCountSyncService::new(state.pool.clone())
        .full(&principal.user_id, &query.ledger_id)
        .await
        .map(Json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_response_uses_beecount_snake_case() {
        let value = serde_json::to_value(BeeCountTwoFactorStatus {
            enabled: false,
            enabled_at: None,
        })
        .unwrap();
        assert_eq!(value["enabled"], false);
        assert!(value.get("enabled_at").is_some());
        assert!(value.get("enabledAt").is_none());
    }

    #[test]
    fn beecount_scope_is_finance_and_files_only() {
        let scopes = crate::auth::scope::default_scopes(AppId::BEECOUNT);
        assert!(scopes.contains(&"finance:write".to_owned()));
        assert!(scopes.contains(&"files:write".to_owned()));
        assert!(!scopes.contains(&"mail:read".to_owned()));
    }
}
