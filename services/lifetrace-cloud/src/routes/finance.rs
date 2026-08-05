//! Finance business CRUD example backed by the same persistent sync repository.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use lifetrace_contracts::ids::{ChangeId, DeviceId, EntityId, RequestId};
use lifetrace_contracts::json_value::JsonValue;
use lifetrace_contracts::sync::v1::{
    AppId, ChangeOperation, ClientPlatform, SyncChangeV1, SyncClientInfo,
};
use lifetrace_contracts::{ErrorCode, PROTOCOL_VERSION, SCHEMA_VERSION};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::AuthenticatedPrincipal;
use crate::error::ApiError;
use crate::state::AppState;

const ENTITY_TYPE: &str = "finance.transaction";

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/api/v1/finance/transactions", get(list).post(create))
        .route(
            "/api/v1/finance/transactions/{id}",
            get(get_one).delete(delete_one),
        )
}

async fn list(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
) -> Result<Json<Value>, ApiError> {
    let items: Vec<Value> = state
        .store
        .list_entities(&principal.user_id, ENTITY_TYPE)
        .await?
        .into_iter()
        .map(|snapshot| {
            json!({
                "id": snapshot.entity_id,
                "serverVersion": snapshot.server_version,
                "data": snapshot.payload,
            })
        })
        .collect();
    Ok(Json(json!({ "items": items })))
}

async fn get_one(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let entity_id = EntityId::new(id);
    match state
        .store
        .entity(&principal.user_id, ENTITY_TYPE, entity_id.as_str())
        .await?
    {
        Some(entity) if !entity.deleted => Ok(Json(json!({
            "id": entity.entity_id,
            "serverVersion": lifetrace_contracts::ServerVersion::from_u64(entity.server_version),
            "data": entity.payload,
        }))),
        _ => Err(ApiError::new(
            ErrorCode::InvalidRequest,
            "transaction not found",
            StatusCode::NOT_FOUND,
        )),
    }
}

async fn create(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let entity_id = payload
        .get("meta")
        .and_then(|meta| meta.get("id"))
        .and_then(Value::as_str)
        .map(EntityId::new)
        .ok_or_else(|| {
            ApiError::new(
                ErrorCode::InvalidEntityPayload,
                "payload meta.id is required",
                StatusCode::BAD_REQUEST,
            )
        })?;

    let change = SyncChangeV1 {
        change_id: ChangeId::new(Uuid::new_v4().to_string()),
        entity_type: lifetrace_contracts::EntityType::new(ENTITY_TYPE),
        entity_id: entity_id.clone(),
        operation: ChangeOperation::new(ChangeOperation::UPSERT),
        base_server_version: lifetrace_contracts::ServerVersion::zero(),
        entity_schema_version: 1,
        client_modified_at: Utc::now(),
        payload: Some(JsonValue(payload)),
        atomic_group_id: None,
        dependencies: vec![],
    };
    let request = lifetrace_contracts::sync::v1::PushRequestV1 {
        request_id: RequestId::new(Uuid::new_v4().to_string()),
        client: client_info(&principal.device_id),
        changes: vec![change],
    };

    let response = state.store.push(&principal.user_id, &request).await?;
    match &response.results[0] {
        lifetrace_contracts::sync::v1::PushChangeResultV1::Accepted {
            server_version, ..
        } => Ok(Json(json!({
            "id": entity_id,
            "serverVersion": server_version,
        }))),
        lifetrace_contracts::sync::v1::PushChangeResultV1::Conflict { reason, .. } => {
            Err(ApiError::new(
                ErrorCode::BaseVersionMismatch,
                format!("conflict: {reason}"),
                StatusCode::CONFLICT,
            ))
        }
        lifetrace_contracts::sync::v1::PushChangeResultV1::Rejected { code, message, .. } => {
            Err(ApiError::new(code.clone(), message, StatusCode::BAD_REQUEST))
        }
        lifetrace_contracts::sync::v1::PushChangeResultV1::Duplicate { .. } => Err(ApiError::new(
            ErrorCode::InvalidRequest,
            "duplicate change id",
            StatusCode::CONFLICT,
        )),
    }
}

async fn delete_one(
    State(state): State<AppState>,
    principal: AuthenticatedPrincipal,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let entity_id = EntityId::new(id);
    let base = state
        .store
        .current_version(&principal.user_id, ENTITY_TYPE, &entity_id)
        .await?
        .ok_or_else(|| {
            ApiError::new(
                ErrorCode::InvalidRequest,
                "transaction not found",
                StatusCode::NOT_FOUND,
            )
        })?;

    let change = SyncChangeV1 {
        change_id: ChangeId::new(Uuid::new_v4().to_string()),
        entity_type: lifetrace_contracts::EntityType::new(ENTITY_TYPE),
        entity_id: entity_id.clone(),
        operation: ChangeOperation::new(ChangeOperation::DELETE),
        base_server_version: lifetrace_contracts::ServerVersion::from_u64(base),
        entity_schema_version: 1,
        client_modified_at: Utc::now(),
        payload: None,
        atomic_group_id: None,
        dependencies: vec![],
    };
    let request = lifetrace_contracts::sync::v1::PushRequestV1 {
        request_id: RequestId::new(Uuid::new_v4().to_string()),
        client: client_info(&principal.device_id),
        changes: vec![change],
    };

    let response = state.store.push(&principal.user_id, &request).await?;
    match &response.results[0] {
        lifetrace_contracts::sync::v1::PushChangeResultV1::Accepted { .. } => {
            Ok(Json(json!({ "deleted": true })))
        }
        _ => Err(ApiError::new(
            ErrorCode::BaseVersionMismatch,
            "delete conflicted with a concurrent change",
            StatusCode::CONFLICT,
        )),
    }
}

fn client_info(device_id: &DeviceId) -> SyncClientInfo {
    SyncClientInfo {
        app_id: AppId::new(AppId::DESKTOP),
        client_version: "0.2.1".to_owned(),
        platform: ClientPlatform::new(ClientPlatform::WINDOWS),
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        device_id: device_id.clone(),
    }
}
