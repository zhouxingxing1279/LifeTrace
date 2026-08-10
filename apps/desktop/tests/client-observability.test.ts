import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  ClientRequestError,
  bindFetch,
  clearRecentClientLogs,
  getRecentClientLogs,
  instrumentedFetch,
  sanitizeLogValue,
  serializeClientError,
  type FetchLike,
} from "../src/services/clientObservability";
import { installGlobalFetchInstrumentation } from "../src/services/fetchInstrumentation";
import { AuthApi } from "../web-client/src/cloud/api";

test("serializeClientError preserves the cause chain", () => {
  const root = new TypeError("Can only call window.fetch on instance of Window");
  const wrapped = new Error("登录请求执行失败", { cause: root });

  const serialized = serializeClientError(wrapped);

  assert.equal(serialized.name, "Error");
  assert.equal(serialized.message, "登录请求执行失败");
  assert.equal(serialized.cause?.name, "TypeError");
  assert.match(serialized.cause?.message ?? "", /window\.fetch/);
  assert.ok(serialized.stack);
});

test("sanitizeLogValue redacts credentials and sensitive content bodies", () => {
  const sanitized = sanitizeLogValue({
    authorization: "Bearer abc.def.ghi",
    password: "secret-password",
    rawBody: "完整第三方通知原文",
    body_text: "完整邮件正文",
    notificationContent: "完整通知内容",
    nested: {
      accessToken: "eyJabcdefgh.abcdefgh.abcdefgh",
      note: "safe value",
    },
  }) as Record<string, unknown>;

  assert.equal(sanitized.authorization, "[REDACTED]");
  assert.equal(sanitized.password, "[REDACTED]");
  assert.equal(sanitized.rawBody, "[REDACTED]");
  assert.equal(sanitized.body_text, "[REDACTED]");
  assert.equal(sanitized.notificationContent, "[REDACTED]");
  assert.deepEqual(sanitized.nested, {
    accessToken: "[REDACTED]",
    note: "safe value",
  });
  assert.doesNotMatch(JSON.stringify(sanitized), /完整第三方通知原文|完整邮件正文|完整通知内容/);
});

test("production logger explicitly drops debug events", async () => {
  const source = await readFile("src/services/clientObservability.ts", "utf8");
  assert.match(source, /PRODUCTION_BUILD/);
  assert.match(source, /level === "debug" && PRODUCTION_BUILD/);
});

test("bindFetch preserves the owner required by a native-style fetch", async () => {
  const owner: { calls: number; fetch: FetchLike } = {
    calls: 0,
    fetch: undefined as unknown as FetchLike,
  };

  owner.fetch = function (this: typeof owner): Promise<Response> {
    assert.equal(this, owner);
    this.calls += 1;
    return Promise.resolve(new Response("ok", { status: 200 }));
  };

  const safeFetch = bindFetch(owner.fetch, owner);
  const response = await safeFetch("https://example.test/health");

  assert.equal(response.status, 200);
  assert.equal(owner.calls, 1);
});

test("AuthApi keeps window-style fetch bound to globalThis", async () => {
  const originalFetch = globalThis.fetch;
  let calls = 0;
  const nativeStyleFetch = function (this: typeof globalThis): Promise<Response> {
    assert.equal(this, globalThis);
    calls += 1;
    return Promise.resolve(new Response("{}", {
      status: 200,
      headers: { "content-type": "application/json" },
    }));
  } as typeof globalThis.fetch;
  globalThis.fetch = nativeStyleFetch;

  try {
    const api = new AuthApi(globalThis.fetch);
    await api.session();
    assert.equal(calls, 1);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("instrumentedFetch records synchronous invocation errors before a request exists", async () => {
  clearRecentClientLogs();
  const throwingFetch: FetchLike = () => {
    throw new TypeError("Can only call window.fetch on instance of Window");
  };

  await assert.rejects(
    () => instrumentedFetch(
      throwingFetch,
      "https://example.test/login",
      { method: "POST" },
      { module: "cloud-auth", action: "login" },
    ),
    (error: unknown) => {
      assert.ok(error instanceof ClientRequestError);
      assert.equal(error.category, "programming");
      assert.equal(error.stage, "fetch.invoke");
      assert.equal(error.requestSent, false);
      assert.ok(error.cause instanceof TypeError);
      return true;
    },
  );

  const failed = getRecentClientLogs().findLast((event) => event.event === "api.request.failed");
  assert.ok(failed);
  const data = failed.data as Record<string, unknown>;
  assert.equal(data.stage, "fetch.invoke");
  assert.equal(data.requestSent, false);
  assert.equal(data.category, "programming");
});

test("instrumentedFetch distinguishes asynchronous network rejection", async () => {
  clearRecentClientLogs();
  const rejectingFetch: FetchLike = () => Promise.reject(new TypeError("Failed to fetch"));

  await assert.rejects(
    () => instrumentedFetch(
      rejectingFetch,
      "https://example.test/login",
      { method: "POST" },
      { module: "cloud-auth", action: "login" },
    ),
    (error: unknown) => {
      assert.ok(error instanceof ClientRequestError);
      assert.equal(error.category, "network");
      assert.equal(error.stage, "fetch.await");
      assert.equal(error.requestSent, true);
      return true;
    },
  );

  const failed = getRecentClientLogs().findLast((event) => event.event === "api.request.failed");
  assert.ok(failed);
  const data = failed.data as Record<string, unknown>;
  assert.equal(data.stage, "fetch.await");
  assert.equal(data.requestSent, true);
  assert.equal(data.category, "network");
});

test("instrumentedFetch records a successful response", async () => {
  clearRecentClientLogs();
  const successfulFetch: FetchLike = async () => new Response("{}", {
    status: 200,
    headers: { "content-type": "application/json" },
  });

  const response = await instrumentedFetch(
    successfulFetch,
    "https://example.test/health",
    undefined,
    { module: "cloud", action: "health" },
  );

  assert.equal(response.status, 200);
  const events = getRecentClientLogs();
  assert.ok(events.some((event) => event.event === "api.request.start"));
  assert.ok(events.some((event) => event.event === "api.fetch.invoke"));
  assert.ok(events.some((event) => event.event === "api.response.received"));
  assert.ok(!events.some((event) => event.event === "api.request.failed"));
});

test("instrumentedFetch omits query strings and fragments from logs", async () => {
  clearRecentClientLogs();
  const successfulFetch: FetchLike = async () => new Response(null, { status: 204 });

  await instrumentedFetch(
    successfulFetch,
    "https://example.test/callback?token=secret-value#private-fragment",
    undefined,
    { module: "cloud", action: "callback" },
  );

  const start = getRecentClientLogs().find((event) => event.event === "api.request.start");
  assert.ok(start);
  const data = start.data as Record<string, unknown>;
  assert.equal(data.url, "https://example.test/callback");
  assert.doesNotMatch(JSON.stringify(start), /secret-value|private-fragment/);
});

test("global fetch instrumentation calls the captured native function exactly once", async () => {
  const originalFetch = globalThis.fetch;
  let calls = 0;
  const nativeStyleFetch = function (this: typeof globalThis): Promise<Response> {
    assert.equal(this, globalThis);
    calls += 1;
    return Promise.resolve(new Response(null, { status: 204 }));
  } as typeof globalThis.fetch;
  globalThis.fetch = nativeStyleFetch;

  try {
    installGlobalFetchInstrumentation();
    const response = await globalThis.fetch("https://example.test/ping");
    assert.equal(response.status, 204);
    assert.equal(calls, 1);
  } finally {
    globalThis.fetch = originalFetch;
  }
});