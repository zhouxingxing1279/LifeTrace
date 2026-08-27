import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

test("desktop app keeps the local workspace as the primary runtime", () => {
  const app = readFileSync("src/components/DesktopApp.tsx", "utf8");
  const shell = readFileSync("src/components/HengXuShell.tsx", "utf8");

  assert.match(app, /<DesktopProviders>/);
  assert.match(app, /<HengXuShell \/>/);
  assert.doesNotMatch(app, /DesktopCloudWorkspace|CloudDataStore|DesktopFeatureRouter/);
  assert.match(shell, /正在连接 SQLite 个人系统/);
});

test("desktop local shell keeps native-only modules available", () => {
  const shell = readFileSync("src/components/HengXuShell.tsx", "utf8");

  for (const moduleName of [
    "PhotoSyncModule",
    "NotesModule",
    "AIAssistantModule",
    "ExecutionModule",
    "MailActionCenter",
    "AnalyticsModule",
  ]) {
    assert.match(shell, new RegExp(moduleName));
  }
});

test("desktop runtime treats cloud as background replication", () => {
  const runtime = readFileSync("src/app/DesktopRuntime.tsx", "utf8");
  const app = readFileSync("src/components/DesktopApp.tsx", "utf8");

  assert.match(runtime, /cloudSync\.now/);
  assert.match(runtime, /syncStatus/);
  assert.match(runtime, /window\.addEventListener\("offline"/);
  assert.match(runtime, /never blocks the local Desktop runtime/);
  assert.doesNotMatch(app, /navigator\.onLine.*当前无网络，数据未保存/s);
});

test("desktop restores and tracks native window placement without losing the visibility fallback", () => {
  const main = readFileSync("tauri-ui/main.tsx", "utf8");
  const state = readFileSync("tauri-ui/windowState.ts", "utf8");
  const fit = readFileSync("tauri-ui/windowFit.ts", "utf8");

  assert.match(main, /await restoreWindowPlacement\(\)/);
  assert.match(main, /installWindowPlacementPersistence\(\)/);
  assert.doesNotMatch(main, /void fitWindowToWorkArea\(\)/);

  assert.match(state, /monitorFromPoint/);
  assert.match(state, /primaryMonitor/);
  assert.match(state, /appWindow\.onMoved/);
  assert.match(state, /appWindow\.onResized/);
  assert.match(state, /appWindow\.isMaximized/);
  assert.match(state, /fitWindowToWorkArea/);
  assert.match(state, /WINDOW_STATE_KEY/);

  assert.match(fit, /currentMonitor/);
  assert.match(fit, /workArea/);
});
