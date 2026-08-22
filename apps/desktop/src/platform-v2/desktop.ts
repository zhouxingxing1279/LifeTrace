import { invoke } from "@tauri-apps/api/core";
import type { LifeTraceState } from "../../../web/src/v2/model";
import type { NativeStatus, PlatformAdapter } from "../../../web/src/v2/platform";

const STORAGE_KEY = "lifetrace.frontend-v2.desktop-state";

async function safeInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T | { error: string }> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    return { error: error instanceof Error ? error.message : String(error) };
  }
}

export const desktopPlatform: PlatformAdapter = {
  kind: "desktop",
  label: "Desktop · Native",
  async loadState() {
    try {
      const raw = window.localStorage.getItem(STORAGE_KEY);
      return raw ? (JSON.parse(raw) as LifeTraceState) : null;
    } catch {
      return null;
    }
  },
  async saveState(state) {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  },
  async getNativeStatus(): Promise<NativeStatus> {
    const [storage, sync, photo, vault] = await Promise.all([
      safeInvoke("storage_status"),
      safeInvoke("sync_status"),
      safeInvoke("photo_status"),
      safeInvoke("vault_status")
    ]);
    return { storage, sync, photo, vault };
  },
  async syncNow() {
    return safeInvoke("sync_now");
  },
  async openExternal(url) {
    await safeInvoke("desktop_open_url", { url });
  }
};

export const nativeCommands = {
  storage: ["storage_status", "storage_migrate"],
  sync: ["sync_status", "sync_now", "sync_conflicts", "sync_resolve_conflict"],
  photo: ["photo_status", "mobile_upload_status", "mobile_upload_start", "mobile_upload_stop", "photo_create_pairing", "photo_cancel_pairing", "photo_recover", "photo_set_compatibility", "photo_export_certificate"],
  notes: ["note_copy_attachment", "note_delete_attachment", "note_open_attachment", "note_show_attachment"],
  files: ["write_text_file", "read_text_file", "desktop_open_url"],
  vault: ["vault_status", "vault_initialize", "vault_unlock", "vault_lock", "vault_list_assets", "vault_list_albums", "vault_verify_integrity", "vault_change_password", "vault_set_auto_lock", "vault_set_lock_on_blur"]
} as const;
