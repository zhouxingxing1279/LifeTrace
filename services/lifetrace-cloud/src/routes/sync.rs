//! Sync protocol v1 endpoints with application-scope authorization.

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use lifetrace_contracts::registry::REGISTRY;
use lifetrace_contracts::sync::v1::{
    CapabilitiesResponseV1, PullRequestV1, PullResponseV1, PushRequestV1, PushResponseV1,
    SnapshotRequestV1, SnapshotResponseV1,
};
use lifetrace_contracts::{EntityType, ErrorCode};

use crate::auth::scope;
use crate::auth::AuthenticatedPrincipal;
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
    principal: AuthenticatedPrincipal,
    Json(request): Json<PushRequestV1>,
) -> Result<Json<PushResponseV1>, ApiError> {
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
    state
        .store
        .push(&principal.user_id, &request)
        .await
        .map(Json)
}

async fn pull(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(mut request): Json<PullRequestV1>,
) -> Result<Json<PullResponseV1>, ApiError> {
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
    principal: AuthenticatedPrincipal,
    Json(mut request): Json<SnapshotRequestV1>,
) -> Result<Json<SnapshotResponseV1>, ApiError> {
    principal.require_scope("sync:read")?;
    authorize_client(&principal, request.client.app_id.as_str())?;
    request.entity_types = authorized_entity_filter(&principal, request.entity_types)?;
    state
        .store
        .snapshot(&principal.user_id, &request)
        .await
        .map(Json)
}
