import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { createBrowserFetch, setCloudFetchOverride } from "../web-client/src/core";

function response(payload: unknown) {
  return new Response(JSON.stringify(payload), { status: 200, headers: { "content-type": "application/json" } });
}

test("shared browser cloud transport can be redirected by the native desktop shell", async () => {
  let received = "";
  setCloudFetchOverride(async (input) => {
    received = String(input);
    return response({ ok: true });
  });
  try {
    const result = await createBrowserFetch()("/api/v1/sync/snapshot", { method: "POST" });
    assert.equal(result.ok, true);
    assert.equal(received, "/api/v1/sync/snapshot");
  } finally {
    setCloudFetchOverride(undefined);
  }
});

test("desktop reuses browser business routes without reusing the browser application shell", () => {
  const desktopApp = readFileSync("src/components/DesktopApp.tsx", "utf8");
  const workspace = readFileSync("src/components/DesktopCloudWorkspace.tsx", "utf8");
  const desktopShell = readFileSync("src/components/DesktopWorkbenchShell.tsx", "utf8");
  const desktopStyles = readFileSync("app/desktop-cloud-workspace.css", "utf8");
  const routes = readFileSync("web-client/src/navigation.ts", "utf8");
  const routeView = readFileSync("web-client/src/components/RouteView.tsx", "utf8");
  const tauriMain = readFileSync("tauri-ui/main.tsx", "utf8");
  const tauriLib = readFileSync("src-tauri/src/lib.rs", "utf8");
  const cloudApi = readFileSync("src-tauri/src/cloud_api.rs", "utf8");

  assert.match(desktopApp, /DesktopCloudWorkspace/);
  assert.match(desktopApp, /HengXuShell/);
  assert.match(desktopApp, /本机工具/);
  assert.doesNotMatch(desktopApp, /lt-desktop-workspace-switch/);
  assert.doesNotMatch(desktopApp, /云端功能/);
  assert.doesNotMatch(desktopApp, /本地功能/);

  assert.match(workspace, /<RouteView/);
  assert.match(workspace, /<DesktopWorkbenchShell/);
  assert.doesNotMatch(workspace, /CloudAppShell/);
  assert.doesNotMatch(workspace, /web-client\/src\/components\/AppShell/);
  assert.match(workspace, /setCloudFetchOverride\(desktopCloudFetch\)/);
  assert.match(workspace, /invoke<NativeCloudApiResponse>\("cloud_api_http_request"/);
  assert.match(workspace, /cloudAuthClient\.refresh\(\)/);
  assert.match(workspace, /syncLocalReplica/);

  assert.match(desktopShell, /NAV_GROUPS/);
  assert.match(desktopShell, /PAGE_COPY/);
  assert.match(desktopShell, /lt-desktop-commandbar/);
  assert.match(desktopShell, /lt-desk-inspector/);
  assert.match(desktopShell, /onOpenLocalTools/);
  assert.match(desktopShell, /window\.history\.back\(\)/);
  assert.match(desktopStyles, /grid-template-columns/);
  assert.match(desktopStyles, /lt-desk-sidebar/);
  assert.match(desktopStyles, /lt-desk-inspector/);

  assert.match(tauriLib, /cloud_api::cloud_api_http_request/);
  assert.match(cloudApi, /path\.starts_with\("\/api\/v1\/"\)/);
  assert.match(cloudApi, /bearer_auth\(access_token\)/);

  for (const route of [
    "/execution/goals",
    "/execution/control",
    "/photo-challenge",
    "/finance/categories",
    "/finance/budgets",
    "/finance/beecount",
    "/devices",
    "/search",
  ]) {
    assert.ok(routes.includes(`\"${route}\"`), `browser route missing ${route}`);
    assert.ok(routeView.includes(`case \"${route}\"`), `route renderer missing ${route}`);
  }

  for (const stylesheet of [
    "web-tokens.css",
    "web-primitives.css",
    "web-shell.css",
    "web-workspaces.css",
    "web-beecount.css",
    "web-features.css",
    "web-photo-challenge.css",
    "desktop-cloud-workspace.css",
  ]) {
    assert.ok(tauriMain.includes(stylesheet), `Tauri bundle missing ${stylesheet}`);
  }
});

test("desktop cookie-only web routes map to equivalent bearer endpoints", () => {
  const workspace = readFileSync("src/components/DesktopCloudWorkspace.tsx", "utf8");
  const management = readFileSync("web-client/src/cloud/management.ts", "utf8");
  const assistantClient = readFileSync("web-client/src/cloud/assistant.ts", "utf8");
  const nativeAuth = readFileSync("../../services/cloud/src/routes/auth.rs", "utf8");
  const assistantRoute = readFileSync("../../services/cloud/src/routes/assistant.rs", "utf8");

  assert.match(management, /\/api\/v1\/web\/devices/);
  assert.match(management, /\/api\/v1\/web\/sessions/);
  assert.match(assistantClient, /\/api\/v1\/web\/assistant/);
  assert.match(workspace, /\/api\/v1\/auth\/devices/);
  assert.match(workspace, /\/api\/v1\/auth\/sessions/);
  assert.match(workspace, /\/api\/v1\/assistant/);
  assert.match(nativeAuth, /\.route\("\/api\/v1\/auth\/devices"/);
  assert.match(nativeAuth, /\.route\("\/api\/v1\/auth\/sessions"/);
  assert.match(assistantRoute, /\.route\("\/api\/v1\/assistant", post\(native_assistant\)\)/);
  assert.match(assistantRoute, /AuthenticatedPrincipal/);
  assert.match(assistantRoute, /run_assistant\(request\)\.await/);
});

test("photo challenge keeps browser cookie auth and maps desktop to a bearer-only owner endpoint", () => {
  const page = readFileSync("web-client/src/pages/PhotoChallengePage.tsx", "utf8");
  const workspace = readFileSync("src/components/DesktopCloudWorkspace.tsx", "utf8");
  const desktopRoute = readFileSync("../../services/cloud/src/routes/photo_challenge_desktop.rs", "utf8");
  const cloudRoutes = readFileSync("../../services/cloud/src/routes/mod.rs", "utf8");

  assert.match(page, /browserFetch\(`/);
  assert.doesNotMatch(page, /await fetch\(`/);
  assert.match(workspace, /\/api\/v1\/photo-challenge\/desktop-admin/);
  assert.match(desktopRoute, /AuthenticatedPrincipal/);
  assert.match(desktopRoute, /PHOTO_CHALLENGE_OWNER_EMAIL/);
  assert.match(cloudRoutes, /photo_challenge_desktop::router\(\)/);
});

test("photo staging originals are imported only by the authenticated native desktop relay", () => {
  const tauriLib = readFileSync("src-tauri/src/lib.rs", "utf8");
  const nativeRelay = readFileSync("src-tauri/src/sync/photo_staging.rs", "utf8");
  const browserApi = readFileSync("web-client/src/cloud/api.ts", "utf8");
  const browserChallenge = readFileSync("web-client/src/pages/PhotoChallengePage.tsx", "utf8");

  assert.match(tauriLib, /auth\.access_token\.is_some\(\) && auth\.cloud_user_id\.is_some\(\)/);
  assert.match(tauriLib, /sync::photo_staging::drain\(&photo_relay_state\)\.await/);
  assert.match(nativeRelay, /\.bearer_auth\(&token\)/);
  assert.match(nativeRelay, /SHA-256 校验失败，云端副本不会删除/);
  assert.match(nativeRelay, /INSERT OR IGNORE INTO photos/);

  const localCommit = nativeRelay.indexOf("import_into_local_library(data_dir, item, &bytes).await?");
  const cloudAck = nativeRelay.indexOf(".delete(format!(\"{base}/api/v1/photo-staging/{}\", item.id))");
  assert.ok(localCommit >= 0, "native relay must commit the original into the LifeTrace photo library");
  assert.ok(cloudAck > localCommit, "cloud staging ACK must happen only after the local album commit");

  assert.doesNotMatch(browserApi, /\/api\/v1\/photo-staging/);
  assert.doesNotMatch(browserChallenge, /\/api\/v1\/photo-staging/);
});
