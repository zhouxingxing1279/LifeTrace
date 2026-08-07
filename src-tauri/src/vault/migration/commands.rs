#[tauri::command]
pub fn vault_status_background(
    state: State<'_, Arc<VaultState>>,
) -> std::result::Result<VaultStatus, String> {
    command_result(state.migration_status())
}

#[tauri::command]
pub async fn vault_unlock_background(
    password: String,
    state: State<'_, Arc<VaultState>>,
) -> std::result::Result<VaultStatus, String> {
    let delay = state.remaining_attempt_delay();
    if !delay.is_zero() {
        tokio::time::sleep(delay).await;
    }
    let vault = state.inner().clone();
    let unlock_vault = vault.clone();
    tauri::async_runtime::spawn_blocking(move || command_result(unlock_vault.unlock(&password)))
        .await
        .map_err(|error| error.to_string())??;
    let resume_vault = vault.clone();
    let works = tauri::async_runtime::spawn_blocking(move || {
        command_result(resume_vault.resume_migration_works())
    })
    .await
    .map_err(|error| error.to_string())??;
    if !works.is_empty() {
        tauri::async_runtime::spawn_blocking(move || vault.process_migration_batch(works));
    }
    command_result(state.migration_status())
}

#[tauri::command]
pub fn vault_list_assets_background(
    trashed: bool,
    album_id: Option<String>,
    state: State<'_, Arc<VaultState>>,
) -> std::result::Result<Vec<VaultAssetView>, String> {
    command_result(state.list_assets_with_migrations(trashed, album_id.as_deref()))
}

#[tauri::command]
pub async fn vault_hide_photos_from_sync_album_background(
    photo_ids: Vec<String>,
    album_id: Option<String>,
    state: State<'_, Arc<VaultState>>,
) -> std::result::Result<serde_json::Value, String> {
    if photo_ids.is_empty() {
        return Err("没有选择要隐藏的照片".to_owned());
    }
    let vault = state.inner().clone();
    let prepare_vault = vault.clone();
    let works = tauri::async_runtime::spawn_blocking(move || {
        command_result(prepare_vault.prepare_photo_migrations(photo_ids, album_id))
    })
    .await
    .map_err(|error| error.to_string())??;
    let count = works.len();
    if count > 0 {
        tauri::async_runtime::spawn_blocking(move || vault.process_migration_batch(works));
    }
    Ok(serde_json::json!({ "started": count > 0, "count": count }))
}

#[tauri::command]
pub fn vault_read_asset_background(
    asset_id: String,
    state: State<'_, Arc<VaultState>>,
) -> std::result::Result<VaultAssetPayload, String> {
    command_result(state.read_asset_with_migration_key(&asset_id))
}

#[tauri::command]
pub fn vault_read_thumbnail_background(
    asset_id: String,
    state: State<'_, Arc<VaultState>>,
) -> std::result::Result<VaultThumbnailPayload, String> {
    command_result(state.read_thumbnail_with_migration_key(&asset_id))
}

#[tauri::command]
pub async fn vault_restore_to_sync_album_background(
    asset_id: String,
    state: State<'_, Arc<VaultState>>,
) -> std::result::Result<VaultAsset, String> {
    let vault = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        command_result(vault.restore_to_sync_album_with_migration_key(&asset_id))
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn vault_delete_asset_permanently_background(
    asset_id: String,
    state: State<'_, Arc<VaultState>>,
) -> std::result::Result<(), String> {
    command_result(state.delete_asset_permanently_with_key(&asset_id))
}

#[tauri::command]
pub async fn vault_verify_integrity_background(
    state: State<'_, Arc<VaultState>>,
) -> std::result::Result<VaultIntegrityReport, String> {
    let vault = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        command_result(vault.verify_integrity_with_migration_keys())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn vault_delete_all_background(
    confirmation: String,
    state: State<'_, Arc<VaultState>>,
) -> std::result::Result<(), String> {
    let vault = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        command_result(vault.delete_all_with_migrations(&confirmation))
    })
    .await
    .map_err(|error| error.to_string())?
}

