import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

test("desktop HTML provides a visible boot shell before JavaScript modules execute", () => {
  const html = readFileSync("tauri-ui/index.html", "utf8");

  assert.match(html, /data-lifetrace-boot-pending="true"/);
  assert.match(html, /LifeTrace 正在启动/);
  assert.match(html, /正在加载桌面组件/);
});

test("desktop startup catches early initialization failures instead of leaving a blank WebView", () => {
  const main = readFileSync("tauri-ui/main.tsx", "utf8");

  assert.match(main, /function renderStartupFailure/);
  assert.match(main, /async function start\(\)/);
  assert.match(main, /renderStartupStatus\("正在初始化桌面环境…"\)/);
  assert.match(main, /try \{[\s\S]*installAppPreferences\(\)[\s\S]*await restoreWindowPlacement\(\)[\s\S]*installTauriApiBridge\(\)[\s\S]*await waitForTauriBackend\(\)/);
  assert.match(main, /catch \(error\) \{[\s\S]*renderStartupFailure\(error\)/);
  assert.match(main, /desktop\.window_persistence_unavailable/);
});

test("desktop bundle targets a conservative WebView2-compatible JavaScript level", () => {
  const vite = readFileSync("vite.tauri.config.ts", "utf8");
  assert.match(vite, /target:\s*"es2020"/);
});
