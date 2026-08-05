#![allow(linker_messages)]

mod cloud_auth;
pub mod contracts;
mod database;
mod desktop;
mod server;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            cloud_auth::cloud_credential_set,
            cloud_auth::cloud_credential_get,
            cloud_auth::cloud_credential_clear,
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
        ])
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let resource_dir = app.path().resource_dir()?;
            let photo_runtime = server::photo::Runtime::new(data_dir.clone());
            app.manage(desktop::DesktopState {
                data_dir: data_dir.clone(),
                photo_runtime: photo_runtime.clone(),
            });
            tauri::async_runtime::spawn(async move {
                if let Err(error) = server::serve(data_dir, resource_dir, photo_runtime).await {
                    eprintln!("LifeTrace local service stopped: {error}");
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run LifeTrace");
}
