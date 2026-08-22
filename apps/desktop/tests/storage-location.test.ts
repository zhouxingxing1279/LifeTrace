import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";

test("storage migration runs blocking work on Tauri's managed runtime", async () => {
  const source = await readFile("src-tauri/src/storage.rs", "utf8");
  assert.doesNotMatch(source, /tokio::task::spawn_blocking/);
  assert.match(source, /tauri::async_runtime::spawn_blocking/);
  assert.match(source, /bulk_copy\(&source, &target_for_task/);
  assert.match(source, /restart_required: false/);
  assert.match(source, /value\.restart_required = true/);
});

test("old storage is committed only after verification and deleted by a background worker", async () => {
  const source = await readFile("src-tauri/src/storage.rs", "utf8");
  const finalizeStart = source.indexOf("fn finalize_pending");
  const commitStart = source.indexOf("fn commit_migration");
  const cancelStart = source.indexOf("fn cancel_failed_migration");
  const cleanupStart = source.indexOf("fn retry_old_directory_cleanup");
  const scheduleStart = source.indexOf("pub fn schedule_pending_cleanup");
  const bootstrapStart = source.indexOf("pub fn bootstrap");
  const finalizeBody = source.slice(finalizeStart, commitStart);
  const commitBody = source.slice(commitStart, cancelStart);
  const cleanupBody = source.slice(cleanupStart, scheduleStart);
  const scheduleBody = source.slice(scheduleStart, bootstrapStart);

  assert.ok(finalizeStart >= 0 && commitStart > finalizeStart);
  assert.match(finalizeBody, /copy_incremental\(&pending\.source, &pending\.target\)\?/);
  assert.match(finalizeBody, /remove_stale_entries\(&pending\.source, &pending\.target\)\?/);
  assert.match(finalizeBody, /verify_tree\(&pending\.source, &pending\.target\)\?/);
  assert.doesNotMatch(finalizeBody, /remove_dir_all/);
  assert.match(commitBody, /config\.active_data_dir = Some\(pending\.target\.clone\(\)\)/);
  assert.match(commitBody, /config\.cleanup_pending = Some\(pending\.source\.clone\(\)\)/);
  assert.match(commitBody, /save_config\(locator, config\)/);
  assert.doesNotMatch(commitBody, /remove_dir_all/);
  assert.match(cleanupBody, /fs::remove_dir_all\(&old_path\)/);
  assert.match(scheduleBody, /tauri::async_runtime::spawn_blocking/);
  assert.match(scheduleBody, /retry_old_directory_cleanup\(&config_path\)/);
  assert.match(source, /PRAGMA integrity_check/);
});

test("failed final verification keeps the old directory active and never schedules deletion", async () => {
  const source = await readFile("src-tauri/src/storage.rs", "utf8");
  const cancelStart = source.indexOf("fn cancel_failed_migration");
  const retryStart = source.indexOf("fn retry_old_directory_cleanup");
  const cancelBody = source.slice(cancelStart, retryStart);
  assert.match(cancelBody, /config\.active_data_dir = Some\(pending\.source\.clone\(\)\)/);
  assert.match(cancelBody, /config\.pending_migration = None/);
  assert.match(cancelBody, /config\.cleanup_pending = None/);
  assert.doesNotMatch(cancelBody, /remove_dir_all/);
});

test("all desktop data roots are created from the resolved storage directory", async () => {
  const source = await readFile("src-tauri/src/lib.rs", "utf8");
  assert.match(source, /storage::bootstrap\(app\.handle\(\)\)/);
  assert.match(source, /storage::schedule_pending_cleanup\(storage_config_path\)/);
  assert.match(source, /VaultState::new\(data_dir\.join\("vault"\)\)/);
  assert.match(source, /Runtime::new\(data_dir\.clone\(\)\)/);
  assert.match(source, /SyncDesktopState::new\(data_dir\.clone\(\)\)/);
  assert.match(source, /server::serve\(data_dir, resource_dir, photo_runtime, sync_state\)/);
});

test("migrated database rewrites note attachment absolute paths to the new root", async () => {
  const source = await readFile("src-tauri/src/storage.rs", "utf8");
  assert.match(source, /fn rewrite_local_file_paths/);
  assert.match(source, /target_root[\s\S]*\.join\("attachments"\)/);
  assert.match(source, /UPDATE note_attachments SET storage_path=\?1 WHERE id=\?2/);
});

test("frontend v2 desktop adapter keeps storage status and migration commands available", async () => {
  const adapter = await readFile("src/platform-v2/desktop.ts", "utf8");
  assert.match(adapter, /storage_status/);
  assert.match(adapter, /storage_migrate/);
});
