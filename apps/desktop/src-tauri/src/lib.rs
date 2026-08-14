#![allow(linker_messages)]

mod cloud_auth;
pub mod contracts;
mod database;
mod desktop;
mod execution;
mod execution_calendar;
mod execution_memo;
mod execution_relation;
mod execution_reminder;
mod execution_structure;
mod execution_waiting;
mod observability;
mod server;
mod storage;
mod sync;
mod vault;

use std::sync::Arc;
use std::time::Duration;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            cloud_auth::cloud_credential_set,
            cloud_auth::cloud_credential_get,
            cloud_auth::cloud_credential_clear,
            observability::client_log_write,
            observability::client_log_path,
            observability::client_log_read_recent,
            storage::storage_status,
            storage::storage_migrate,
            desktop::photo_status,
            desktop::mobile_upload_status,
            desktop::mobile_upload_start,
            desktop::mobile_upload_stop,
            desktop::photo_create_pairing,
            desktop::photo_cancel_pairing,
            desktop::photo_recover,
            desktop::photo_set_compatibility,
            desktop::photo_export_certificate,
            desktop::note_copy_attachment,
            desktop::note_delete_attachment,
            desktop::note_open_attachment,
            desktop::note_show_attachment,
            desktop::write_text_file,
            desktop::read_text_file,
            desktop::desktop_open_url,
            sync::commands::sync_set_session,
            sync::commands::sync_clear_session,
            sync::commands::sync_bind_current_profile,
            sync::commands::sync_create_cloud_profile,
            sync::commands::sync_profiles,
            sync::commands::sync_set_active_profile,
            sync::commands::sync_status,
            sync::commands::sync_now,
            sync::commands::sync_conflicts,
            sync::commands::sync_resolve_conflict,
            vault::vault_status,
            vault::vault_initialize,
            vault::vault_unlock,
            vault::vault_lock,
            vault::vault_list_assets,
            vault::vault_list_albums,
            vault::vault_hide_photos_from_sync_album,
            vault::vault_restore_to_sync_album,
            vault::vault_read_asset,
            vault::vault_read_thumbnail,
            vault::vault_move_to_trash,
            vault::vault_restore_asset,
            vault::vault_delete_asset_permanently,
            vault::vault_create_album,
            vault::vault_rename_album,
            vault::vault_delete_album,
            vault::vault_set_asset_album,
            vault::vault_verify_integrity,
            vault::vault_change_password,
            vault::vault_set_auto_lock,
            vault::vault_set_lock_on_blur,
            vault::vault_delete_all,
            vault::vault_status_background,
            vault::vault_unlock_background,
            vault::vault_list_assets_background,
            vault::vault_hide_photos_from_sync_album_background,
            vault::vault_read_asset_background,
            vault::vault_read_thumbnail_background,
            vault::vault_restore_to_sync_album_background,
            vault::vault_delete_asset_permanently_background,
            vault::vault_verify_integrity_background,
            vault::vault_delete_all_background,
        ])
        .setup(|app| {
            let (data_dir, default_data_dir, storage_config_path) =
                storage::bootstrap(app.handle()).map_err(std::io::Error::other)?;
            let resource_dir = app.path().resource_dir()?;

            app.manage(storage::StorageState::new(
                data_dir.clone(),
                default_data_dir,
                storage_config_path.clone(),
            ));
            storage::schedule_pending_cleanup(storage_config_path);

            let vault_state = Arc::new(vault::VaultState::new(data_dir.join("vault"))?);
            app.manage(vault_state);
            let photo_runtime = server::photo::Runtime::new(data_dir.clone());
            app.manage(desktop::DesktopState {
                data_dir: data_dir.clone(),
                photo_runtime: photo_runtime.clone(),
            });
            let sync_state = sync::SyncDesktopState::new(data_dir.clone());
            app.manage(sync_state.clone());
            let scheduler_state = sync_state.clone();
            tauri::async_runtime::spawn(async move {
                scheduler_state.scheduler().await;
            });
            let photo_relay_state = sync_state.clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(30));
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                loop {
                    interval.tick().await;
                    let authenticated = {
                        let auth = photo_relay_state.auth.read().await;
                        auth.access_token.is_some() && auth.cloud_user_id.is_some()
                    };
                    if !authenticated {
                        continue;
                    }
                    if let Err(error) = sync::photo_staging::drain(&photo_relay_state).await {
                        eprintln!("LifeTrace cloud photo staging drain skipped: {error}");
                    }
                }
            });
            tauri::async_runtime::spawn(async move {
                if let Err(error) =
                    server::serve(data_dir, resource_dir, photo_runtime, sync_state).await
                {
                    eprintln!("LifeTrace local service stopped: {error}");
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run LifeTrace");
}
