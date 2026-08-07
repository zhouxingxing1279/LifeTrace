import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";

test("storage migration runs bulk file copying off the UI/main thread", async () => {
  const source = await readFile("src-tauri/src/storage.rs", "utf8");
  assert.match(source, /tokio::task::spawn_blocking/);
  assert.match(source, /bulk_copy\(&source, &target_for_task/);
  assert.match(source, /restart_required: false/);
  assert.match(source, /value\.restart_required = true/);
});

test("old storage is removed only after final synchronization and verification", async () => {
  const source = await readFile("src-tauri/src/storage.rs", "utf8");
  const finalize = source.indexOf("finalize_pending(&pending, &locator)?");
  const activate = source.indexOf("config.active_data_dir = Some(pending.target.clone())");
  const cleanup = source.indexOf("fs::remove_dir_all(&pending.source)");
  assert.ok(finalize >= 0, "pending migration must be finalized");
  assert.ok(activate > finalize, "new location must be activated after verification");
  assert.ok(cleanup > activate, "old location must only be deleted after the new location is committed");
  assert.match(source, /verify_tree\(&pending\.source, &pending\.target\)\?/);
  assert.match(source, /PRAGMA integrity_check/);
});

test("all desktop data roots are created from the resolved storage directory", async () => {
  const source = await readFile("src-tauri/src/lib.rs", "utf8");
  assert.match(source, /storage::bootstrap\(app\.handle\(\)\)/);
  assert.match(source, /VaultState::new\(data_dir\.join\("vault"\)\)/);
  assert.match(source, /Runtime::new\(data_dir\.clone\(\)\)/);
  assert.match(source, /SyncDesktopState::new\(data_dir\.clone\(\)\)/);
  assert.match(source, /server::serve\(data_dir, resource_dir, photo_runtime, sync_state\)/);
});

test("settings exposes storage location picker, progress and restart completion", async () => {
  const panel = await readFile("src/components/StorageLocationPanel.tsx", "utf8");
  const bridge = await readFile("tauri-ui/apiBridge.ts", "utf8");
  const settings = await readFile("src/components/CloudAccountPanel.tsx", "utf8");
  assert.match(settings, /settings-storage/);
  assert.match(settings, /<StorageLocationPanel/);
  assert.match(panel, /更改位置/);
  assert.match(panel, /重启并完成迁移/);
  assert.match(panel, /后台线程执行/);
  assert.match(bridge, /directory: true/);
  assert.match(bridge, /storage_migrate/);
  assert.match(bridge, /relaunch\(\)/);
});
