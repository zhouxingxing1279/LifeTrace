//! Sync protocol v1 endpoints with application-scope authorization.

use std::collections::BTreeSet;

use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use lifetrace_contracts::registry::REGISTRY;
use lifetrace_contracts::sync::v1::{
    CapabilitiesResponseV1, PullRequestV1, PullResponseV1, PushRequestV1, PushResponseV1,
    SnapshotRequestV1, SnapshotResponseV1,
};
use lifetrace_contracts::{EntityType, ErrorCode};

use crate::auth::scope;
use crate::auth::security::cookie_value;
use crate::auth::{AuthCredential, AuthenticatedPrincipal};
use crate::beecount_compat::{beecount_wire_id, USER_GLOBAL_LEDGER_SENTINEL};
use crate::error::ApiError;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/api/v1/sync/capabilities", get(capabilities))
        .route("/api/v1/sync/push", post(push))
        .route("/api/v1/sync/pull", post(pull))
        .route("/api/v1/sync/snapshot", post(snapshot))
}

async fn capabilities(
    State(state): State<AppState>,
) -> Result<Json<CapabilitiesResponseV1>, ApiError> {
    state.store.capabilities().await.map(Json)
}

/// Native clients authenticate with a Bearer token. Browser clients authenticate
/// with the HttpOnly web-session cookie and must prove same-origin intent with
/// the existing CSRF token on every POST-based sync endpoint.
async fn sync_principal(
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

fn authorize_client(principal: &AuthenticatedPrincipal, app_id: &str) -> Result<(), ApiError> {
    if principal.app_id.as_str() == app_id {
        Ok(())
    } else {
        Err(ApiError::new(
            ErrorCode::AuthInvalid,
            "authenticated application does not match sync client",
            StatusCode::UNAUTHORIZED,
        ))
    }
}

fn authorize_entity(
    principal: &AuthenticatedPrincipal,
    entity_type: &str,
    write: bool,
) -> Result<(), ApiError> {
    let required = scope::required_entity_scope(entity_type, write).ok_or_else(|| {
        ApiError::new(
            ErrorCode::UnknownEntityType,
            format!("entity type is not authorized for sync: {entity_type}"),
            StatusCode::BAD_REQUEST,
        )
    })?;
    principal.require_scope(required)
}

fn authorized_entity_filter(
    principal: &AuthenticatedPrincipal,
    requested: Option<Vec<EntityType>>,
) -> Result<Option<Vec<EntityType>>, ApiError> {
    if let Some(values) = requested {
        for value in &values {
            authorize_entity(principal, value.as_str(), false)?;
        }
        return Ok(Some(values));
    }

    // `None` is the protocol's canonical all-entity scope. Preserve it when
    // the principal can read every registered entity type so cursors emitted
    // by push/snapshot remain valid for an unfiltered pull. Restricted apps
    // receive an explicit allow-list and therefore a distinct scope hash.
    let has_full_read_scope = REGISTRY.iter().all(|descriptor| {
        scope::required_entity_scope(descriptor.entity_type, false)
            .is_some_and(|required| principal.scopes.contains(required))
    });
    if has_full_read_scope {
        return Ok(None);
    }

    let values = REGISTRY
        .iter()
        .filter_map(|descriptor| {
            let required = scope::required_entity_scope(descriptor.entity_type, false)?;
            principal
                .scopes
                .contains(required)
                .then(|| EntityType::new(descriptor.entity_type))
        })
        .collect();
    Ok(Some(values))
}

async fn push(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<PushRequestV1>,
) -> Result<Json<PushResponseV1>, ApiError> {
    let principal = sync_principal(&state, &headers).await?;
    principal.require_scope("sync:write")?;
    authorize_client(&principal, request.client.app_id.as_str())?;
    for change in &request.changes {
        if let Some(required) = scope::required_entity_scope(change.entity_type.as_str(), true) {
            principal.require_scope(required)?;
        } else if REGISTRY
            .iter()
            .any(|descriptor| descriptor.entity_type == change.entity_type.as_str())
        {
            // A registered entity without an authorization mapping is a
            // server configuration error and must fail closed.
            return Err(ApiError::new(
                ErrorCode::AuthScopeDenied,
                format!(
                    "no write scope is configured for entity type: {}",
                    change.entity_type
                ),
                StatusCode::FORBIDDEN,
            ));
        }
        // Unknown protocol entity types are deliberately passed to the batch
        // processor, which returns the stable per-item UNKNOWN_ENTITY_TYPE
        // rejection required by the sync contract.
    }
    let notify_beecount = request
        .changes
        .iter()
        .any(|change| change.entity_type.as_str().starts_with("finance."));
    let notify_user_global = request.changes.iter().any(|change| {
        matches!(
            change.entity_type.as_str(),
            "finance.account" | "finance.category" | "finance.tag"
        )
    });
    let response = state.store.push(&principal.user_id, &request).await?;
    if state.database_enabled && notify_beecount {
        publish_native_sync_change(&state, &principal, notify_user_global).await;
    }
    Ok(Json(response))
}

async fn publish_native_sync_change(
    state: &AppState,
    principal: &AuthenticatedPrincipal,
    notify_user_global: bool,
) {
    let Ok(user_uuid) = uuid::Uuid::parse_str(principal.user_id.as_str()) else {
        return;
    };
    let Ok(server_cursor) = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(MAX(cursor),0)::BIGINT FROM sync_change_log WHERE user_id=$1",
    )
    .bind(user_uuid)
    .fetch_one(&state.pool)
    .await
    else {
        return;
    };
    let mut ledger_ids = BTreeSet::new();
    if notify_user_global {
        ledger_ids.insert(USER_GLOBAL_LEDGER_SENTINEL.to_owned());
    }
    if let Ok(rows) = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT entity_id,payload->>'beecountLedgerId' FROM sync_entities \
         WHERE user_id=$1 AND entity_type='finance.ledger' AND is_deleted=FALSE",
    )
    .bind(user_uuid)
    .fetch_all(&state.pool)
    .await
    {
        for (entity_id, legacy_id) in rows {
            ledger_ids.insert(legacy_id.unwrap_or_else(|| beecount_wire_id(&entity_id)));
        }
    }
    let now = chrono::Utc::now();
    for ledger_id in ledger_ids {
        if ledger_id == USER_GLOBAL_LEDGER_SENTINEL {
            state.beecount_realtime.publish_sync_change(
                &principal.user_id,
                &ledger_id,
                server_cursor,
                now,
            );
            continue;
        }
        let users = crate::beecount_collaboration::member_user_ids(&state.pool, &ledger_id)
            .await
            .unwrap_or_else(|_| Vec::new());
        if users.is_empty() {
            state.beecount_realtime.publish_sync_change(
                &principal.user_id,
                &ledger_id,
                server_cursor,
                now,
            );
        } else {
            for user_id in users {
                state.beecount_realtime.publish(
                    &user_id.to_string(),
                    serde_json::json!({
                        "type":"sync_change","ledgerId":ledger_id,
                        "serverCursor":server_cursor,"serverTimestamp":now,
                    }),
                );
            }
        }
    }
}

async fn pull(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut request): Json<PullRequestV1>,
) -> Result<Json<PullResponseV1>, ApiError> {
    let principal = sync_principal(&state, &headers).await?;
    principal.require_scope("sync:read")?;
    authorize_client(&principal, request.client.app_id.as_str())?;
    request.entity_types = authorized_entity_filter(&principal, request.entity_types)?;
    state
        .store
        .pull(&principal.user_id, &request)
        .await
        .map(Json)
}

async fn snapshot(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut request): Json<SnapshotRequestV1>,
) -> Result<Json<SnapshotResponseV1>, ApiError> {
    let principal = sync_principal(&state, &headers).await?;
    principal.require_scope("sync:read")?;
    authorize_client(&principal, request.client.app_id.as_str())?;
    request.entity_types = authorized_entity_filter(&principal, request.entity_types)?;
    state
        .store
        .snapshot(&principal.user_id, &request)
        .await
        .map(Json)
}
