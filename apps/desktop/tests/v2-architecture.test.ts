import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

const root = path.resolve(import.meta.dirname, "..");
const repo = path.resolve(root, "../..");

const read = (file: string) => fs.readFileSync(file, "utf8");

test("desktop renderer consumes the shared frontend v2 layer", () => {
  const renderer = read(path.join(root, "tauri-ui/main.tsx"));
  assert.match(renderer, /web\/src\/v2\/App/);
  assert.match(renderer, /desktopPlatform/);
});

test("native runtime survives clean-room rewrite", () => {
  const rust = read(path.join(root, "src-tauri/src/lib.rs"));
  for (const command of ["storage_status", "sync_now", "photo_status", "vault_status", "desktop_open_url"]) assert.ok(rust.includes(command), `${command} must remain registered`);
});

test("legacy visual roots are absent", () => {
  for (const relative of ["app", "src/components", "src/ui"]) assert.equal(fs.existsSync(path.join(root, relative)), false, `${relative} must stay removed`);
  assert.ok(fs.existsSync(path.join(repo, "apps/web/src/v2/design-system/ui.tsx")));
});
