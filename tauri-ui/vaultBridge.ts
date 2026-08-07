import { invoke } from "@tauri-apps/api/core";

export function installVaultBridge() {
  if (!("__TAURI_INTERNALS__" in window)) return;

  window.vaultApi = {
    status: () => invoke<VaultStatus>("vault_status_background"),
    initialize: (password) => invoke<VaultStatus>("vault_initialize", { password }),
    unlock: (password) => invoke<VaultStatus>("vault_unlock_background", { password }),
    lock: () => invoke<VaultStatus>("vault_lock"),
    listAssets: (options = {}) => invoke<VaultAsset[]>("vault_list_assets_background", {
      trashed: options.trashed ?? false,
      albumId: options.albumId ?? null,
    }),
    listAlbums: () => invoke<VaultAlbum[]>("vault_list_albums"),
    hidePhotosFromSyncAlbum: (photoIds, albumId = null) =>
      invoke<{ started: boolean; count: number }>("vault_hide_photos_from_sync_album_background", {
        photoIds,
        albumId,
      }),
    restoreToSyncAlbum: (assetId) =>
      invoke<VaultAsset>("vault_restore_to_sync_album_background", { assetId }),
    readAsset: (assetId) => invoke<VaultAssetPayload>("vault_read_asset_background", { assetId }),
    readThumbnail: (assetId) => invoke<VaultThumbnailPayload>("vault_read_thumbnail_background", { assetId }),
    moveToTrash: (assetId) => invoke<void>("vault_move_to_trash", { assetId }),
    restoreAsset: (assetId) => invoke<void>("vault_restore_asset", { assetId }),
    deleteAssetPermanently: (assetId) => invoke<void>("vault_delete_asset_permanently_background", { assetId }),
    createAlbum: (name) => invoke<VaultAlbum>("vault_create_album", { name }),
    renameAlbum: (albumId, name) => invoke<void>("vault_rename_album", { albumId, name }),
    deleteAlbum: (albumId) => invoke<void>("vault_delete_album", { albumId }),
    setAssetAlbum: (assetId, albumId, assigned) => invoke<void>("vault_set_asset_album", { assetId, albumId, assigned }),
    verifyIntegrity: () => invoke<VaultIntegrityReport>("vault_verify_integrity_background"),
    changePassword: (oldPassword, newPassword) => invoke<VaultStatus>("vault_change_password", { oldPassword, newPassword }),
    setAutoLock: (seconds) => invoke<VaultStatus>("vault_set_auto_lock", { seconds }),
    setLockOnBlur: (enabled) => invoke<VaultStatus>("vault_set_lock_on_blur", { enabled }),
    deleteAll: (confirmation) => invoke<void>("vault_delete_all_background", { confirmation }),
  };
}
