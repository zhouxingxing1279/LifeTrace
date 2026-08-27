import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";

test("desktop no longer hosts the web runtime bridge", () => {
  assert.equal(existsSync("src/components/DesktopCloudWorkspace.tsx"), false);
  const app = readFileSync("src/components/DesktopApp.tsx", "utf8");
  assert.doesNotMatch(app, /web\/src|DesktopFeatureRouter|CloudDataStore/);
});

test("tauri entry and vite config are desktop-owned", () => {
  const entry = readFileSync("tauri-ui/main.tsx", "utf8");
  const vite = readFileSync("vite.tauri.config.ts", "utf8");
  const packageJson = readFileSync("package.json", "utf8");

  assert.doesNotMatch(entry, /web\/src\/styles\/globals\.css/);
  assert.doesNotMatch(vite, /appsRoot|web.*postcss|path\.join\([^\n]*"web"/);
  assert.match(vite, /allow: \[projectRoot\]/);
  assert.doesNotMatch(packageJson, /prepare:web-shared|ensure-shared-web-deps/);
});

test("desktop typecheck excludes web application sources", () => {
  const tsconfig = readFileSync("tsconfig.json", "utf8");
  assert.doesNotMatch(tsconfig, /\.\.\/web\/src|apps\/web/);
  assert.match(tsconfig, /src\/app\/\*\*\/\*\.tsx/);
});
