import assert from "node:assert/strict";
import test from "node:test";
import { buildWebSyncClient, WEB_SYNC_APP_ID } from "../src/v2/api/cloud";

test("web sync client uses the canonical authenticated application id", () => {
  const client = buildWebSyncClient("device-123");

  assert.equal(WEB_SYNC_APP_ID, "lifetrace-web");
  assert.equal(client.appId, "lifetrace-web");
  assert.notEqual(client.appId, "web");
  assert.equal(client.platform, "web");
  assert.equal(client.deviceId, "device-123");
});
