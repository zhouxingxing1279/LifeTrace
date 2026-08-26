import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

test("desktop HTML provides a visible boot shell before JavaScript modules execute", () => {
  const html = readFileSync("tauri-ui/index.html", "utf8");

  assert.match(html, /data-lifetrace-boot-pending="true"/);
  assert.match(html, /data-lifetrace-boot-stage="html"/);
  assert.match(html, /LifeTrace 正在启动/);
  assert.match(html, /正在加载桌面组件/);
  assert.match(html, /desktop-startup-watchdog\.js/);
});

test("desktop startup watchdog turns module load stalls into a visible failure", () => {
  const watchdog = readFileSync("public/desktop-startup-watchdog.js", "utf8");

  assert.match(watchdog, /__LIFETRACE_MODULE_STARTED__/);
  assert.match(watchdog, /LifeTrace 桌面组件加载超时/);
  assert.match(watchdog, /15000/);
  assert.match(watchdog, /window\.addEventListener\(\s*["']error["']/);
});

test("desktop startup catches early initialization failures instead of leaving a blank WebView", () => {
  const main = readFileSync("tauri-ui/main.tsx", "utf8");

  assert.match(main, /__LIFETRACE_MODULE_STARTED__\s*=\s*true/);
  assert.match(main, /function renderStartupFailure/);
  assert.match(main, /async function start\(\)/);
  assert.match(main, /renderStartupStatus\("正在初始化桌面环境…"\)/);
  assert.match(main, /try \{[\s\S]*installAppPreferences\(\)[\s\S]*await restoreWindowPlacement\(\)[\s\S]*installTauriApiBridge\(\)[\s\S]*await waitForTauriBackend\(45_000, renderStartupStatus\)/);
  assert.match(main, /catch \(error\) \{[\s\S]*renderStartupFailure\(error\)/);
  assert.match(main, /desktop\.window_persistence_unavailable/);
});

test("backend startup polling avoids AbortSignal.timeout and reports native service failure", () => {
  const startup = readFileSync("tauri-ui/backendStartup.ts", "utf8");

  assert.doesNotMatch(startup, /AbortSignal\.timeout/);
  assert.match(startup, /new AbortController\(\)/);
  assert.match(startup, /local_service_status/);
  assert.match(startup, /phase === "failed"/);
  assert.match(startup, /45_000/);
});

test("desktop bundle targets a conservative WebView2-compatible JavaScript level", () => {
  const vite = readFileSync("vite.tauri.config.ts", "utf8");
  assert.match(vite, /target:\s*"es2020"/);
});

test("theme preference listener supports WebView2 runtimes without MediaQueryList.addEventListener", () => {
  const preferences = readFileSync("src/services/appPreferences.ts", "utf8");
  assert.match(preferences, /typeof media\.addEventListener === "function"/);
  assert.match(preferences, /typeof media\.addListener === "function"/);
});
