import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";

function functionBody(source: string, marker: string, nextMarker: string): string {
  const start = source.indexOf(marker);
  const end = source.indexOf(nextMarker, start + marker.length);
  assert.ok(start >= 0, `missing marker: ${marker}`);
  assert.ok(end > start, `missing next marker: ${nextMarker}`);
  return source.slice(start, end);
}

test("offline desktop session automatically retries the stored refresh credential", async () => {
  const store = await readFile("src/stores/useCloudAuthStore.ts", "utf8");
  const desktop = await readFile("src/components/DesktopApp.tsx", "utf8");
  const client = await readFile("src/services/cloudAuth.ts", "utf8");

  const reconnect = functionBody(store, "  async reconnect()", "  async loadCapabilities()");
  assert.match(reconnect, /get\(\)\.phase !== "offline"/);
  assert.match(reconnect, /cloudAuthClient\.hasStoredCredential\(\)/);
  assert.match(reconnect, /cloudAuthClient\.restore\(\)/);
  assert.match(reconnect, /set\(authenticatedPatch\(snapshot\)\)/);
  assert.doesNotMatch(reconnect, /cloudAuthClient\.login\(/);
  assert.doesNotMatch(reconnect, /password/);

  assert.match(desktop, /OFFLINE_RECONNECT_INTERVAL_MS = 10_000/);
  assert.match(desktop, /window\.setInterval\(retry, OFFLINE_RECONNECT_INTERVAL_MS\)/);
  assert.match(desktop, /window\.addEventListener\("online", retry\)/);
  assert.match(desktop, /if \(phase !== "offline" \|\| !user\) return/);

  assert.match(client, /const refreshToken = await credentialApi\(\)\.get\(\)/);
  assert.match(client, /\/api\/v1\/auth\/refresh/);
});

test("invalid saved refresh credential exits offline mode instead of looping forever", async () => {
  const store = await readFile("src/stores/useCloudAuthStore.ts", "utf8");
  const client = await readFile("src/services/cloudAuth.ts", "utf8");

  const reconnect = functionBody(store, "  async reconnect()", "  async loadCapabilities()");
  assert.match(reconnect, /if \(!hasCredential\) \{\s*set\(anonymousPatch\(\)\)/s);
  assert.match(client, /if \(response\.status === 401 \|\| response\.status === 403\) await this\.clearLocal\(\)/);
});

test("password minimum fallback and cloud defaults are 9 characters", async () => {
  const accountEntry = await readFile("src/components/account/AccountEntry.tsx", "utf8");
  const security = await readFile("src/components/account/AccountSecurityPanel.tsx", "utf8");
  const cloudConfig = await readFile("../../../services/cloud/src/config.rs", "utf8");
  const localCompose = await readFile("../../../deploy/cloud/docker-compose.local.yml", "utf8");
  const productionCompose = await readFile("../../../deploy/cloud/docker-compose.production.example.yml", "utf8");

  assert.match(accountEntry, /passwordMinLength = auth\.capabilities\?\.passwordMinLength \?\? 9/);
  assert.match(security, /minLength = auth\.capabilities\?\.passwordMinLength \?\? 9/);
  assert.match(cloudConfig, /auth_password_min_length: 9/);
  assert.match(localCompose, /AUTH_PASSWORD_MIN_LENGTH: "\$\{AUTH_PASSWORD_MIN_LENGTH:-9\}"/);
  assert.match(productionCompose, /AUTH_PASSWORD_MIN_LENGTH: "\$\{AUTH_PASSWORD_MIN_LENGTH:-9\}"/);
});
