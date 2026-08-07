import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";

test("desktop startup restores cloud session and keeps automatic sync wiring intact", async () => {
  const [entry, store, runtime, server] = await Promise.all([
    readFile("tauri-ui/main.tsx", "utf8"),
    readFile("src/stores/useCloudAuthStore.ts", "utf8"),
    readFile("src-tauri/src/sync/runtime.rs", "utf8"),
    readFile("src-tauri/src/server.rs", "utf8"),
  ]);

  assert.match(entry, /useCloudAuthStore\.getState\(\)\.restore\(\)/);
  assert.match(store, /origin:\s*savedCloudOrigin\(\)/);
  assert.match(store, /cloudAuthClient\.restore\(\)/);
  assert.match(runtime, /pub async fn scheduler/);
  assert.match(runtime, /self\.wake\.notified\(\)/);
  assert.match(runtime, /Duration::from_secs\(2\)/);
  assert.match(runtime, /pub async fn set_session[\s\S]*state\.wake\.notify_one\(\)/);
  assert.match(server, /sync_state\.signal_local_change\(\)/);
});
