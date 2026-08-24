import assert from "node:assert/strict";
import test from "node:test";
import { CloudStateRepository } from "../src/v2/api/cloud";

test("web sync uses the authenticated lifetrace-web application id", async () => {
  const originalFetch = globalThis.fetch;
  const calls: Array<{ input: string; init?: RequestInit }> = [];
  let requestNumber = 0;

  globalThis.fetch = (async (input: string | URL | Request, init?: RequestInit) => {
    calls.push({ input: String(input), init });
    requestNumber += 1;
    if (requestNumber === 1) {
      return new Response(JSON.stringify({
        user: { id: "user-1", email: "user@example.com" },
        session: { deviceId: "device-1" },
        csrfToken: "csrf-1"
      }), { status: 200, headers: { "content-type": "application/json" } });
    }
    return new Response(JSON.stringify({
      snapshotId: "snapshot-1",
      nextPageToken: null,
      items: []
    }), { status: 200, headers: { "content-type": "application/json" } });
  }) as typeof fetch;

  try {
    const repository = new CloudStateRepository();
    const session = await repository.getSession();
    assert.equal(session.authenticated, true);
    await repository.loadState();

    assert.equal(calls[1]?.input, "/api/v1/sync/snapshot");
    const body = JSON.parse(String(calls[1]?.init?.body));
    assert.equal(body.client.appId, "lifetrace-web");
    assert.equal(body.client.platform, "web");
    assert.equal(body.client.deviceId, "device-1");
  } finally {
    globalThis.fetch = originalFetch;
  }
});
