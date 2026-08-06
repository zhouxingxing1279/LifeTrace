import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export function installVaultBridge() {
  if (!("__TAURI_INTERNALS__" in window)) return;

  window.vaultApi = {
    status: () => invoke<VaultStatus>("vault_status"),
    initialize: (password) => invoke<VaultStatus>("vault_initialize", { password }),
    unlock: (password) => invoke<VaultStatus>("vault_unlock", { password }),
    lock: () => invoke<VaultStatus>("vault_lock"),
    listAssets: () => invoke<VaultAsset[]>("vault_list_assets"),
    async importFiles() {
      const selected = await open({
        multiple: true,
        directory: false,
        filters: [
          { name: "照片和视频", extensions: ["jpg", "jpeg", "png", "webp", "gif", "bmp", "tif", "tiff", "mp4", "mov", "m4v", "avi", "mkv", "webm"] },
        ],
      });
      if (!selected) return [];
      const sourcePaths = Array.isArray(selected) ? selected : [selected];
      return invoke<VaultAsset[]>("vault_import_files", { sourcePaths });
    },
    readAsset: (assetId) => invoke<VaultAssetPayload>("vault_read_asset", { assetId }),
    readThumbnail: (assetId) => invoke<VaultThumbnailPayload>("vault_read_thumbnail", { assetId }),
    deleteAsset: (assetId) => invoke<void>("vault_delete_asset", { assetId }),
    changePassword: (oldPassword, newPassword) => invoke<VaultStatus>("vault_change_password", { oldPassword, newPassword }),
    setAutoLock: (seconds) => invoke<VaultStatus>("vault_set_auto_lock", { seconds }),
    deleteAll: (confirmation) => invoke<void>("vault_delete_all", { confirmation }),
  };
}
