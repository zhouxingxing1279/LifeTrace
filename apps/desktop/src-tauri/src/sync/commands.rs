use lifetrace_sync_client::ConflictResolution;
use serde::Serialize;
use tauri::State;

use super::runtime::{self, SessionBindingResult, SyncDesktopState, SyncStatusView};

#[tauri::command]
pub async fn sync_set_session(
    state: State<'_, SyncDesktopState>,
    origin: String,
    access_token: String,
    device_id: String,
) -> Result<SessionBindingResult, String> {
    runtime::set_session(&state, origin, access_token, device_id).await
}

#[tauri::command]
pub async fn sync_clear_session(state: State<'_, SyncDesktopState>) -> Result<(), String> {
    runtime::mark_logged_out(&state).await
}

#[tauri::command]
pub async fn sync_bind_current_profile(
    state: State<'_, SyncDesktopState>,
) -> Result<String, String> {
    runtime::bind_current_profile(&state).await
}

#[tauri::command]
pub async fn sync_create_cloud_profile(
    state: State<'_, SyncDesktopState>,
    display_name: String,
) -> Result<String, String> {
    runtime::create_cloud_profile(&state, &display_name).await
}

#[tauri::command]
pub async fn sync_profiles(
    state: State<'_, SyncDesktopState>,
) -> Result<Vec<crate::database::profile::LocalProfile>, String> {
    runtime::list_profiles(&state)
}

#[tauri::command]
pub async fn sync_set_active_profile(
    state: State<'_, SyncDesktopState>,
    profile_id: String,
) -> Result<(), String> {
    runtime::set_active_profile(&state, &profile_id)
}

#[tauri::command]
pub async fn sync_status(state: State<'_, SyncDesktopState>) -> Result<SyncStatusView, String> {
    runtime::status(&state)
}

#[tauri::command]
pub async fn sync_now(
    state: State<'_, SyncDesktopState>,
    force_snapshot: Option<bool>,
) -> Result<lifetrace_sync_client::SyncRunReport, String> {
    runtime::run_now(&state, force_snapshot.unwrap_or(false)).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictView {
    conflict_id: String,
    entity_type: String,
    entity_id: String,
    kind: String,
    local_payload: Option<serde_json::Value>,
    remote_payload: Option<serde_json::Value>,
    server_deleted: bool,
}

#[tauri::command]
pub async fn sync_conflicts(
    state: State<'_, SyncDesktopState>,
) -> Result<Vec<ConflictView>, String> {
    Ok(runtime::conflicts(&state)
        .await?
        .into_iter()
        .map(|value| ConflictView {
            conflict_id: value.conflict_id.to_string(),
            entity_type: value.entity_type.to_string(),
            entity_id: value.entity_id.to_string(),
            kind: value.kind,
            local_payload: value.local_payload.map(|payload| payload.0),
            remote_payload: value.remote_payload.map(|payload| payload.0),
            server_deleted: value.server_deleted,
        })
        .collect())
}

#[tauri::command]
pub async fn sync_resolve_conflict(
    state: State<'_, SyncDesktopState>,
    conflict_id: String,
    resolution: String,
) -> Result<(), String> {
    let resolution = match resolution.as_str() {
        "accept_remote" => ConflictResolution::AcceptRemote,
        "keep_local" => ConflictResolution::KeepLocal,
        "discard" => ConflictResolution::Discard,
        _ => return Err("不支持的冲突解决方式".to_owned()),
    };
    runtime::resolve_conflict(&state, &conflict_id, resolution).await
}
