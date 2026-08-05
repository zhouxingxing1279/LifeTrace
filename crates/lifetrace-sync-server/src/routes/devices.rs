//! Device registration placeholder.
//!
//! Real registration, naming, revocation and tokens belong to EPIC-04. This
//! endpoint exists so the sync flow has a stable device identity to point at.

use std::collections::HashMap;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::post;
use axum::{Json, Router};
use lifetrace_contracts::DeviceId;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::routes::resolve_user;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceRegisterRequest {
    pub app_id: String,
    pub platform: String,
    #[serde(default)]
    pub device_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceRegisterResponse {
    pub device_id: DeviceId,
    pub status: &'static str,
    pub message: &'static str,
}

#[derive(Debug, Clone)]
pub struct RegisteredDevice {
    pub device_id: DeviceId,
    pub user_id: lifetrace_contracts::UserId,
    pub app_id: String,
    pub platform: String,
    pub device_name: Option<String>,
}

/// Registry of registered devices (in-memory; EPIC-04 owns persistence).
#[derive(Default)]
pub struct DeviceRegistry {
    devices: HashMap<DeviceId, RegisteredDevice>,
}

impl DeviceRegistry {
    pub fn register(&mut self, user_id: lifetrace_contracts::UserId, request: DeviceRegisterRequest) -> DeviceId {
        let device_id = DeviceId::new(Uuid::new_v4().to_string());
        self.devices.insert(
            device_id.clone(),
            RegisteredDevice {
                device_id: device_id.clone(),
                user_id,
                app_id: request.app_id,
                platform: request.platform,
                device_name: request.device_name,
            },
        );
        device_id
    }

    pub fn is_registered(&self, device_id: &DeviceId) -> bool {
        self.devices.contains_key(device_id)
    }
}

pub fn router() -> Router<AppState> {
    Router::<AppState>::new().route("/api/v1/devices/register", post(register))
}

async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<DeviceRegisterRequest>,
) -> Json<serde_json::Value> {
    let user = resolve_user(&headers);
    let mut registry = state.devices.write().unwrap();
    let device_id = registry.register(user.clone(), request);
    Json(json!({
        "deviceId": device_id,
        "status": "registered",
        "message": "Device registered (EPIC-04 will own real registration)"
    }))
}
