import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

const mediaFilters = [
  {
    name: "照片和视频",
    extensions: [
      "jpg", "jpeg", "png", "webp", "gif", "bmp", "tif", "tiff",
      "mp4", "mov", "m4v", "avi", "mkv", "webm",
    ],
  },
];

export function installVaultBridge() {
  if (!("__TAURI_INTERNALS__" in window)) return;

  window.vaultApi = {
    status: () => invoke<VaultStatus>("vault_status"),
    initialize: (password) => invoke<VaultStatus>("vault_initialize", { password }),
    unlock: (password) => invoke<VaultStatus>("vault_unlock", { password }),
    lock: () => invoke<VaultStatus>("vault_lock"),
    listAssets: (options = {}) => invoke<VaultAsset[]>("vault_list_assets", {
      trashed: options.trashed ?? false,
      albumId: options.albumId ?? null,
    }),
    listAlbums: () => invoke<VaultAlbum[]>("vault_list_albums"),
    async importFiles(options = {}) {
      const selected = await open({ multiple: true, directory: false, filters: mediaFilters });
      if (!selected) return [];
      const sourcePaths = Array.isArray(selected) ? selected : [selected];
      return invoke<VaultAsset[]>("vault_import_files", {
        sourcePaths,
        moveSource: options.moveSource ?? false,
        albumId: options.albumId ?? null,
      });
    },
    readAsset: (assetId) => invoke<VaultAssetPayload>("vault_read_asset", { assetId }),
    readThumbnail: (assetId) => invoke<VaultThumbnailPayload>("vault_read_thumbnail", { assetId }),
    async exportAsset(assetId, removeFromVault) {
      const selected = await open({ directory: true, multiple: false });
      if (!selected || Array.isArray(selected)) return null;
      return invoke<string>("vault_export_asset", {
        assetId,
        targetDirectory: selected,
        removeFromVault,
      });
    },
    moveToTrash: (assetId) => invoke<void>("vault_move_to_trash", { assetId }),
    restoreAsset: (assetId) => invoke<void>("vault_restore_asset", { assetId }),
    deleteAssetPermanently: (assetId) => invoke<void>("vault_delete_asset_permanently", { assetId }),
    createAlbum: (name) => invoke<VaultAlbum>("vault_create_album", { name }),
    renameAlbum: (albumId, name) => invoke<void>("vault_rename_album", { albumId, name }),
    deleteAlbum: (albumId) => invoke<void>("vault_delete_album", { albumId }),
    setAssetAlbum: (assetId, albumId, assigned) => invoke<void>("vault_set_asset_album", { assetId, albumId, assigned }),
    verifyIntegrity: () => invoke<VaultIntegrityReport>("vault_verify_integrity"),
    changePassword: (oldPassword, newPassword) => invoke<VaultStatus>("vault_change_password", { oldPassword, newPassword }),
    setAutoLock: (seconds) => invoke<VaultStatus>("vault_set_auto_lock", { seconds }),
    setLockOnBlur: (enabled) => invoke<VaultStatus>("vault_set_lock_on_blur", { enabled }),
    deleteAll: (confirmation) => invoke<void>("vault_delete_all", { confirmation }),
  };
}
