import assert from "node:assert/strict";
import test from "node:test";
import {
  AuthApi,
  WebSyncStore,
  amountToCents,
  createFinanceAccount,
  createNote,
  createTransaction,
  createVocabulary,
} from "../web-client/src/core";

class MemoryStorage {
  private readonly values = new Map<string, string>();
  getItem(key: string) { return this.values.get(key) ?? null; }
  setItem(key: string, value: string) { this.values.set(key, value); }
  removeItem(key: string) { this.values.delete(key); }
}

function jsonResponse(payload: unknown, status = 200) {
  return new Response(JSON.stringify(payload), { status, headers: { "content-type": "application/json" } });
}

test("amountToCents parses exact decimal cents", () => {
  assert.equal(amountToCents("12"), 1200);
  assert.equal(amountToCents("12.3"), 1230);
  assert.equal(amountToCents("-0.05"), -5);
  assert.throws(() => amountToCents("1.234"), /金额格式无效/);
});

test("domain factories satisfy EPIC-02 required fields", () => {
  const account = createFinanceAccount("user-1", "device-1", "微信零钱");
  const transaction = createTransaction("user-1", "device-1", { accountId: account.meta.id, amount: "23.50", type: "expense", note: "午餐" });
  const note = createNote("user-1", "device-1", "想法", "今天继续推进 LifeTrace");
  const vocabulary = createVocabulary("user-1", "device-1", "Resilient", "有韧性的");

  assert.equal(account.accountType, "cash");
  assert.equal(transaction.amountCents, 2350);
  assert.equal(transaction.status, "confirmed");
  assert.equal(note.noteType, "quick");
  assert.equal(note.contentMarkdown, "今天继续推进 LifeTrace");
  assert.equal(vocabulary.normalizedWord, "resilient");
  assert.equal(vocabulary.status, "LEARNING");
});

test("browser login uses the HttpOnly-cookie endpoint and required scopes", async () => {
  let captured: { url: string; init?: RequestInit } | null = null;
  const api = new AuthApi(async (input, init) => {
    captured = { url: String(input), init };
    return jsonResponse({ user: { id: "u", email: "a@b.com" }, session: { id: "s" }, csrfToken: "csrf" });
  });
  await api.login("a@b.com", "secret", false);
  assert.equal(captured?.url, "/api/v1/web/session/login");
  assert.equal(captured?.init?.credentials, "include");
  const body = JSON.parse(String(captured?.init?.body)) as { requestedScopes: string[] };
  assert.ok(body.requestedScopes.includes("sync:write"));
  assert.ok(body.requestedScopes.includes("finance:write"));
  assert.ok(body.requestedScopes.includes("notes:write"));
  assert.ok(body.requestedScopes.includes("english:write"));
});

test("offline changes persist and are restored", () => {
  const storage = new MemoryStorage();
  const first = new WebSyncStore("user-1", "device-1", storage);
  const note = createNote("user-1", "device-1", "离线", "断网也能记录");
  first.queueUpsert("note.note", note);

  const restored = new WebSyncStore("user-1", "device-1", storage).snapshot();
  assert.equal(restored.outbox.length, 1);
  assert.equal(restored.entities["note.note"]?.[note.meta.id]?.title, "离线");
  assert.equal(restored.outbox[0]?.baseServerVersion, "0");
});

test("sync accepts local changes then applies server pull in cursor order", async () => {
  const storage = new MemoryStorage();
  const requests: Array<{ url: string; body: Record<string, unknown> }> = [];
  const note = createNote("user-1", "device-1", "本地", "待同步");
  const serverVocabulary = createVocabulary("user-1", "server-device", "steady", "稳定的");

  const fetcher = async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input);
    const body = JSON.parse(String(init?.body)) as Record<string, unknown>;
    requests.push({ url, body });
    if (url.endsWith("/push")) {
      const changes = body.changes as Array<Record<string, unknown>>;
      return jsonResponse({
        requestId: body.requestId,
        serverTime: new Date().toISOString(),
        latestCursor: "10",
        results: changes.map((change) => ({
          status: "accepted",
          changeId: change.changeId,
          entityType: change.entityType,
          entityId: change.entityId,
          serverVersion: "3",
          cursor: "10",
          serverModifiedAt: new Date().toISOString(),
        })),
      });
    }
    return jsonResponse({
      requestId: body.requestId,
      serverTime: new Date().toISOString(),
      nextCursor: "11",
      hasMore: false,
      changes: [{
        cursor: "11",
        entityType: "english.vocabulary",
        entityId: serverVocabulary.meta.id,
        operation: "upsert",
        serverVersion: "1",
        serverModifiedAt: new Date().toISOString(),
        payload: serverVocabulary,
        tombstone: null,
        originDeviceId: "server-device",
      }],
    });
  };

  const store = new WebSyncStore("user-1", "device-1", storage, fetcher);
  store.queueUpsert("note.note", note);
  const result = await store.sync();

  assert.equal(requests[0]?.url, "/api/v1/sync/push");
  assert.equal(requests[1]?.url, "/api/v1/sync/pull");
  assert.equal(result.outbox.length, 0);
  assert.equal(result.cursor, "11");
  assert.equal(result.entities["note.note"]?.[note.meta.id]?.meta.serverVersion, "3");
  assert.equal(result.entities["english.vocabulary"]?.[serverVocabulary.meta.id]?.meta.serverVersion, "1");
});

test("sync resolves optimistic conflict with current server entity", async () => {
  const storage = new MemoryStorage();
  const local = createNote("user-1", "device-1", "本地标题", "本地内容");
  const server = { ...local, title: "服务器标题", meta: { ...local.meta, serverVersion: "8" } };
  let call = 0;
  const fetcher = async (input: RequestInfo | URL, init?: RequestInit) => {
    call += 1;
    const body = JSON.parse(String(init?.body)) as Record<string, unknown>;
    if (String(input).endsWith("/push")) {
      const change = (body.changes as Array<Record<string, unknown>>)[0]!;
      return jsonResponse({ requestId: body.requestId, serverTime: new Date().toISOString(), latestCursor: "8", results: [{ status: "conflict", conflictId: "c1", changeId: change.changeId, entityType: "note.note", entityId: local.meta.id, clientBaseServerVersion: "0", currentServerVersion: "8", serverEntity: server, serverDeleted: false, reason: "version_mismatch" }] });
    }
    return jsonResponse({ requestId: body.requestId, serverTime: new Date().toISOString(), changes: [], nextCursor: "8", hasMore: false });
  };
  const store = new WebSyncStore("user-1", "device-1", storage, fetcher);
  store.queueUpsert("note.note", local);
  const result = await store.sync();
  assert.equal(call, 2);
  assert.equal(result.outbox.length, 0);
  assert.equal(result.entities["note.note"]?.[local.meta.id]?.title, "服务器标题");
  assert.equal(result.conflicts.length, 1);
});
