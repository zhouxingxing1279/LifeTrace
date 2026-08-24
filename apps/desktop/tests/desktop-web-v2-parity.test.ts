import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import test from "node:test";

const root = process.cwd();
const read = (path: string) => readFileSync(join(root, path), "utf8");

test("desktop cloud workspace mounts the maintained apps/web feature layer", () => {
  const workspace = read("src/components/DesktopCloudWorkspace.tsx");
  assert.match(workspace, /\.\.\/\.\.\/\.\.\/web\/src\/app\/AppContext/);
  assert.match(workspace, /DesktopFeatureRouter/);
  assert.match(workspace, /setCloudFetchOverride\(desktopCloudFetch\)/);
  assert.doesNotMatch(workspace, /web-client/);
});

test("desktop native shell owns navigation while web owns feature pages", () => {
  const shell = read("src/components/DesktopWorkbenchShell.tsx");
  const router = read("../web/src/app/DesktopFeatureRouter.tsx");
  assert.doesNotMatch(shell, /web-client/);
  for (const route of [
    "/app/today",
    "/app/execution",
    "/app/calendar",
    "/app/habits",
    "/app/fitness",
    "/app/health",
    "/app/notes",
    "/app/english",
    "/app/review",
    "/app/finance",
    "/app/assistant",
    "/app/search",
    "/app/settings",
  ]) {
    assert.match(router, new RegExp(route.replaceAll("/", "\\/")));
  }
  assert.doesNotMatch(workspaceSource(), /AppShell/);
});

function workspaceSource(): string {
  return read("src/components/DesktopCloudWorkspace.tsx");
}

test("tauri entry loads the current web visual contract instead of legacy css", () => {
  const entry = read("tauri-ui/main.tsx");
  assert.match(entry, /web\/src\/styles\/globals\.css/);
  assert.doesNotMatch(entry, /web-client\/src/);
});
