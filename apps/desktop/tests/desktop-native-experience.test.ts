import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

test("desktop workbench provides native command and persistent layout behaviors", () => {
  const shell = readFileSync("src/components/DesktopWorkbenchShell.tsx", "utf8");

  assert.match(shell, /CommandPalette/);
  assert.match(shell, /event\.ctrlKey \|\| event\.metaKey/);
  assert.match(shell, /key === "k"/);
  assert.match(shell, /SIDEBAR_COMPACT_KEY/);
  assert.match(shell, /window\.localStorage\.setItem\(SIDEBAR_COMPACT_KEY/);
  assert.doesNotMatch(shell, /INSPECTOR_OPEN_KEY|lt-desk-inspector|桌面辅助面板/);
  assert.match(shell, /path: "\/app\/photos", label: "相册", icon: Images/);
  assert.match(shell, /打开本机工具/);
  assert.match(shell, /立即同步/);
  assert.match(shell, /开启隐私模式/);
});

test("desktop photos return to the primary navigation without duplicating the local tools page", () => {
  const workspace = readFileSync("src/components/DesktopCloudWorkspace.tsx", "utf8");
  const localTools = readFileSync("src/components/DesktopLocalToolsCenter.tsx", "utf8");

  assert.match(workspace, /path === "\/app\/photos" \? <PhotoSyncModule \/>/);
  assert.doesNotMatch(localTools, /PhotoSyncModule|id: "photos"/);
});

test("manual bookkeeping stays absent while finance remains available and fitness import lives in fitness", () => {
  const shell = readFileSync("src/components/DesktopWorkbenchShell.tsx", "utf8");
  const router = readFileSync("../web/src/app/DesktopFeatureRouter.tsx", "utf8");
  const workspace = readFileSync("src/components/DesktopCloudWorkspace.tsx", "utf8");
  const localTools = readFileSync("src/components/DesktopLocalToolsCenter.tsx", "utf8");
  const fitnessImport = readFileSync("src/components/DesktopFitnessImport.tsx", "utf8");

  assert.doesNotMatch(shell, /手动记账/);
  assert.match(shell, /path: "\/app\/finance", label: "财务"/);
  assert.match(router, /path="\/app\/finance\/\*"/);
  assert.doesNotMatch(localTools, /训练和账单|健身数据/);
  assert.match(localTools, /label: "账单导入"/);
  assert.match(workspace, /path === "\/app\/fitness"/);
  assert.doesNotMatch(fitnessImport, /MobileUploadControl|手机上传/);
  assert.match(fitnessImport, /XunjiImportPanel/);
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
