#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cloud_auth;
mod contracts;
mod database;
mod desktop;
mod server;
mod sync;
mod vault;

use std::sync::Arc;

use anyhow::Context;
use cloud_auth::CloudCredentialManager;
use desktop::DesktopState;
use sync::runtime::SyncRuntime;
use vault::VaultState;

pub fn run() {
    let app_data_dir = desktop::app_data_dir().expect("failed to resolve local app data directory");
    database::bootstrap(&app_data_dir).expect("failed to bootstrap database");
    let vault_state = VaultState::new(app_data_dir.join("vault"))
        .expect("failed to initialize local private album state");
    let cloud_credentials = Arc::new(CloudCredentialManager::new(&app_data_dir));
    let sync_runtime = Arc::new(
        SyncRuntime::new(
            app_data_dir.join("lifetrace.db"),
            cloud_credentials.clone(),
            desktop::device_id(&app_data_dir).expect("failed to resolve device id"),
        )
        .expect("failed to initialize sync runtime"),
    );
    let state = DesktopState::new(app_data_dir, cloud_credentials, sync_runtime)
        .expect("failed to initialize desktop state");
    let server_state = state.server_state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(state)
        .manage(vault_state)
        .invoke_handler(tauri::generate_handler![
            desktop::cloud_credential_set,
            desktop::cloud_credential_get,
            desktop::cloud_credential_clear,
            desktop::sync_set_session,
            desktop::sync_clear_session,
            desktop::sync_bind_current_profile,
            desktop::sync_create_cloud_profile,
            desktop::sync_profiles,
            desktop::sync_set_active_profile,
            desktop::sync_status,
            desktop::sync_now,
            desktop::sync_conflicts,
            desktop::sync_resolve_conflict,
            desktop::mobile_upload_status,
            desktop::mobile_upload_start,
            desktop::mobile_upload_stop,
            desktop::photo_status,
            desktop::photo_set_compatibility,
            desktop::photo_create_pairing,
            desktop::photo_cancel_pairing,
            desktop::photo_recover,
            desktop::photo_export_certificate,
            desktop::note_copy_attachment,
            desktop::note_open_attachment,
            desktop::note_show_attachment,
            desktop::note_delete_attachment,
            desktop::write_text_file,
            desktop::read_text_file,
            vault::vault_status,
            vault::vault_initialize,
            vault::vault_unlock,
            vault::vault_lock,
            vault::vault_list_assets,
            vault::vault_import_files,
            vault::vault_read_asset,
            vault::vault_read_thumbnail,
            vault::vault_delete_asset,
            vault::vault_change_password,
            vault::vault_set_auto_lock,
            vault::vault_delete_all
        ])
        .setup(move |_app| {
            let state = server_state.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = server::run(state).await {
                    eprintln!("LifeTrace local API stopped: {error:#}");
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running LifeTrace");
}
