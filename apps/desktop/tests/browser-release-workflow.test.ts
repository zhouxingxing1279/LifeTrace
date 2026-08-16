import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";

test("Windows release does not let tauri-action merge an existing latest.json", async () => {
  const workflow = await readFile("../../.github/workflows/release-windows.yml", "utf8");
  assert.match(workflow, /uploadUpdaterJson:\s*false/);
  assert.match(workflow, /- name: Generate updater manifests/);
  assert.match(workflow, /gh release upload[\s\S]*--clobber/);
});

test("Windows updater manifest is rebuilt from the uploaded NSIS installer and signature", async () => {
  const workflow = await readFile("../../.github/workflows/release-windows.yml", "utf8");
  assert.match(workflow, /\*_\$\{version\}_x64-setup\.exe/);
  assert.match(workflow, /Updater signature asset not found/);
  assert.match(workflow, /Accept = "application\/octet-stream"/);
  assert.match(workflow, /"windows-x86_64"/);
  assert.match(workflow, /"windows-x86_64-nsis"/);
  assert.match(workflow, /browser_download_url/);
});

test("Windows release keeps lint unit tests and frontend build as independent gates", async () => {
  const workflow = await readFile("../../.github/workflows/release-windows.yml", "utf8");
  assert.match(workflow, /- name: Lint\s+[\s\S]*?run: npm run lint/);
  assert.match(workflow, /- name: Unit tests\s+[\s\S]*?run: npm run test:unit/);
  assert.match(workflow, /- name: Frontend build\s+[\s\S]*?run: npm run web:build/);
  assert.doesNotMatch(workflow, /- name: Lint, unit tests and frontend build/);
});
