import assert from "node:assert/strict";
import test from "node:test";

import { CloudAuthClient } from "../src/services/cloudAuth";

test("cloud auth module does not persist access or refresh tokens in web storage", async () => {
  const source = await import("node:fs/promises").then(fs => fs.readFile("src/services/cloudAuth.ts", "utf8"));
  assert.equal(source.includes('localStorage.setItem("access'), false);
  assert.equal(source.includes('localStorage.setItem("refresh'), false);
  assert.match(source, /cloudCredentialApi/);
});

test("cloud auth persists only the non-sensitive cloud origin needed for restart restore", async () => {
  const source = await import("node:fs/promises").then(fs => fs.readFile("src/services/cloudAuth.ts", "utf8"));
  assert.match(source, /lifetrace-cloud-origin/);
  assert.match(source, /savedCloudOrigin/);
  assert.match(source, /persistCloudOrigin/);
});

test("cloud auth client starts unauthenticated with tokens outside serializable state", () => {
  const client = new CloudAuthClient();
  assert.deepEqual(client.state(), { scopes: [], authenticated: false });
  assert.equal(JSON.stringify(client.state()).includes("Token"), false);
});
