import assert from "node:assert/strict";
import test from "node:test";
import { WebManagementApi } from "../web-client/src/core";

function response(payload: unknown, status = 200) {
  return new Response(JSON.stringify(payload), { status, headers: { "content-type": "application/json" } });
}

test("Web management uses Cookie routes and CSRF on mutations", async () => {
  const calls: Array<{ url: string; init?: RequestInit }> = [];
  const api = new WebManagementApi(async (input, init) => {
    const url = String(input);
    calls.push({ url, init });
    if (url === "/api/v1/web/devices") return response({ devices: [] });
    if (url === "/api/v1/web/sessions") return response({ sessions: [] });
    if (url.includes("/devices/") && init?.method === "PATCH") return response({ id: "d1", deviceName: "Laptop" });
    return response({ accepted: true });
  });

  await api.devices();
  await api.sessions();
  await api.renameDevice("d1", "Laptop", "csrf-token");
  await api.revokeDevice("d2", "csrf-token");
  await api.revokeSession("s2", "csrf-token");

  assert.deepEqual(calls.map((call) => call.url), [
    "/api/v1/web/devices",
    "/api/v1/web/sessions",
    "/api/v1/web/devices/d1",
    "/api/v1/web/devices/d2/revoke",
    "/api/v1/web/sessions/s2",
  ]);
  for (const call of calls) assert.equal(call.init?.credentials, "include");
  for (const call of calls.slice(2)) {
    assert.equal(new Headers(call.init?.headers).get("x-csrf-token"), "csrf-token");
  }
});
