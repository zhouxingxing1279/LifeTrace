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

test("desktop authenticated workspace reuses the browser route renderer instead of duplicating features", () => {
  const desktopApp = readFileSync("src/components/DesktopApp.tsx", "utf8");
  const workspace = readFileSync("src/components/DesktopCloudWorkspace.tsx", "utf8");
  const routes = readFileSync("web-client/src/navigation.ts", "utf8");
  const routeView = readFileSync("web-client/src/components/RouteView.tsx", "utf8");
  const tauriMain = readFileSync("tauri-ui/main.tsx", "utf8");
  const tauriLib = readFileSync("src-tauri/src/lib.rs", "utf8");
  const cloudApi = readFileSync("src-tauri/src/cloud_api.rs", "utf8");

  assert.match(desktopApp, /DesktopCloudWorkspace/);
  assert.match(desktopApp, /HengXuShell/);
  assert.match(desktopApp, /云端功能/);
  assert.match(desktopApp, /本地功能/);
  assert.match(workspace, /<RouteView/);
  assert.match(workspace, /setCloudFetchOverride\(desktopCloudFetch\)/);
  assert.match(workspace, /invoke<NativeCloudApiResponse>\("cloud_api_http_request"/);
  assert.match(workspace, /cloudAuthClient\.refresh\(\)/);
  assert.match(workspace, /syncLocalReplica/);
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
