import assert from "node:assert/strict";
import test from "node:test";
import { apiRequest, LifeTraceApiError } from "../src/v2/api/client";

test("apiRequest uses cookie credentials and csrf for cloud mutations", async () => {
  const original = globalThis.fetch;
  let captured: RequestInit | undefined;
  globalThis.fetch = (async (_input: string | URL | Request, init?: RequestInit) => {
    captured = init;
    return new Response(JSON.stringify({ ok: true }), { status: 200, headers: { "content-type": "application/json" } });
  }) as typeof fetch;
  try {
    const result = await apiRequest<{ ok: boolean }>("/api/v1/sync/push", { method: "POST", csrfToken: "csrf-test", body: { requestId: "r1" } });
    assert.equal(result.ok, true);
    assert.equal(captured?.credentials, "include");
    assert.equal(new Headers(captured?.headers).get("x-csrf-token"), "csrf-test");
    assert.equal(new Headers(captured?.headers).get("content-type"), "application/json");
  } finally {
    globalThis.fetch = original;
  }
});

test("apiRequest normalizes structured cloud errors", async () => {
  const original = globalThis.fetch;
  globalThis.fetch = (async () => new Response(JSON.stringify({ code: "SYNC_CONFLICT", message: "conflict", retryable: true }), { status: 409, headers: { "content-type": "application/json" } })) as typeof fetch;
  try {
    await assert.rejects(() => apiRequest("/api/v1/sync/push"), (error: unknown) => {
      assert.ok(error instanceof LifeTraceApiError);
      assert.equal(error.status, 409);
      assert.equal(error.code, "SYNC_CONFLICT");
      assert.equal(error.retryable, true);
      return true;
    });
  } finally {
    globalThis.fetch = original;
  }
});
