import assert from "node:assert/strict";
import test from "node:test";
import "../web-client/src/bootstrap";
import {
  AuthApi,
  CloudConflictError,
  CloudDataStore,
  amountToCents,
  createFinanceAccount,
  createFinanceCategory,
  createNote,
  createTransaction,
  createVocabulary,
  findProbableDuplicate,
  searchEntities,
  type JsonEntity,
} from "../web-client/src/core";
import { mapImportRows, parseCsv } from "../web-client/src/importer";

function response(payload: unknown, status = 200) {
  return new Response(JSON.stringify(payload), { status, headers: { "content-type": "application/json" } });
}

function snapshot(payloads: Array<{ entityType: string; entity: JsonEntity }> = []) {
  return response({
    requestId: "r1",
    snapshotId: "snapshot-1",
    snapshotCursor: "12",
    items: payloads.map(({ entityType, entity }, index) => ({ entityType, entityId: entity.meta.id, serverVersion: String(index + 1), payload: entity })),
    nextPageToken: null,
    completed: true,
    serverTime: new Date().toISOString(),
  });
}

test("amountToCents preserves exact decimal cents", () => {
  assert.equal(amountToCents("12"), 1200);
  assert.equal(amountToCents("12.30"), 1230);
  assert.equal(amountToCents("-0.05"), -5);
  assert.throws(() => amountToCents("1.234"), /金额格式无效/);
});

test("domain factories satisfy registered cloud schemas", () => {
  const account = createFinanceAccount("user-1", "device-1", "微信零钱");
  const category = createFinanceCategory("user-1", "device-1", "餐饮", "expense");
  const transaction = createTransaction("user-1", "device-1", { accountId: account.meta.id, categoryId: category.meta.id, amount: "23.50", type: "expense", note: "午餐" });
  const note = createNote("user-1", "device-1", "想法", "继续推进 LifeTrace");
  const vocabulary = createVocabulary("user-1", "device-1", "Resilient", "有韧性的");
  assert.equal(account.accountType, "cash");
  assert.equal(category.categoryType, "expense");
  assert.equal(transaction.amountCents, 2350);
  assert.equal(transaction.status, "confirmed");
  assert.equal(note.contentMarkdown, "继续推进 LifeTrace");
  assert.equal(vocabulary.normalizedWord, "resilient");
});

test("browser login uses cookie session and required web scopes", async () => {
  let captured: { url: string; init?: RequestInit } | null = null;
  const api = new AuthApi(async (input, init) => {
    captured = { url: String(input), init };
    return response({ user: { id: "u", email: "a@b.com" }, session: { id: "s", deviceId: "d" }, csrfToken: "csrf" });
  });
  await api.login("a@b.com", "secret", false);
  assert.equal(captured?.url, "/api/v1/web/session/login");
  assert.equal(captured?.init?.credentials, "include");
  const body = JSON.parse(String(captured?.init?.body)) as { requestedScopes: string[] };
  for (const scope of ["sync:write", "finance:write", "notes:write", "english:write", "files:write", "account:write", "devices:write", "sessions:write"]) assert.ok(body.requestedScopes.includes(scope), scope);
});

test("initial load comes from cloud snapshot without an outbox", async () => {
  const note = createNote("user-1", "server", "云端笔记", "服务器内容");
  let requestBody: Record<string, unknown> | null = null;
  const store = new CloudDataStore("user-1", "device-1", "csrf", async (input, init) => {
    assert.equal(String(input), "/api/v1/sync/snapshot");
    requestBody = JSON.parse(String(init?.body));
    return snapshot([{ entityType: "note.note", entity: note }]);
  });
  const state = await store.load();
  assert.equal(requestBody?.pageSize, 500);
  assert.equal(state.cursor, "12");
  assert.equal(state.entities["note.note"]?.[note.meta.id]?.title, "云端笔记");
  assert.equal("outbox" in state, false);
});

test("upsert changes memory only after cloud acceptance", async () => {
  const requests: string[] = [];
  const store = new CloudDataStore("user-1", "device-1", "csrf", async (input, init) => {
    requests.push(String(input));
    if (String(input).endsWith("snapshot")) return snapshot();
    const body = JSON.parse(String(init?.body)) as { changes: Array<Record<string, unknown>> };
    const change = body.changes[0]!;
    return response({ results: [{ status: "accepted", changeId: change.changeId, entityType: change.entityType, entityId: change.entityId, serverVersion: "3" }] });
  });
  await store.load();
  const entity = createNote("user-1", "device-1", "直写云端", "没有离线队列");
  assert.equal(store.list("note.note").length, 0);
  const state = await store.upsert("note.note", entity);
  assert.deepEqual(requests, ["/api/v1/sync/snapshot", "/api/v1/sync/push"]);
  assert.equal(state.entities["note.note"]?.[entity.meta.id]?.meta.serverVersion, "3");
});

test("failed cloud write leaves in-memory state unchanged", async () => {
  const store = new CloudDataStore("user-1", "device-1", "csrf", async (input) => String(input).endsWith("snapshot") ? snapshot() : response({ message: "数据库不可用" }, 503));
  await store.load();
  const entity = createNote("user-1", "device-1", "失败", "不得本地落库");
  await assert.rejects(() => store.upsert("note.note", entity), /数据库不可用/);
  assert.equal(store.list("note.note").length, 0);
});

test("optimistic conflict applies the current server entity", async () => {
  const local = createNote("user-1", "device-1", "本地", "本地内容");
  const server = { ...local, title: "服务器标题", meta: { ...local.meta, serverVersion: "8" } };
  const store = new CloudDataStore("user-1", "device-1", "csrf", async (input, init) => {
    if (String(input).endsWith("snapshot")) return snapshot([{ entityType: "note.note", entity: { ...local, meta: { ...local.meta, serverVersion: "2" } } }]);
    const body = JSON.parse(String(init?.body)) as { changes: Array<Record<string, unknown>> };
    const change = body.changes[0]!;
    return response({ results: [{ status: "conflict", changeId: change.changeId, entityType: "note.note", entityId: local.meta.id, currentServerVersion: "8", serverEntity: server, serverDeleted: false, reason: "version_mismatch" }] });
  });
  await store.load();
  await assert.rejects(() => store.upsert("note.note", { ...local, title: "再次修改", meta: { ...local.meta, serverVersion: "2" } }), CloudConflictError);
  assert.equal(store.list("note.note")[0]?.title, "服务器标题");
  assert.equal(store.snapshot().conflicts.length, 1);
});

test("CSV import creates candidate transactions and detects duplicates", () => {
  const rows = parseCsv('交易时间,金额(元),收/支,交易对方,交易单号\n2026-08-01 12:00:00,25.50,支出,餐厅,T001\n');
  assert.equal(rows.length, 2);
  const preview = mapImportRows("user-1", "device-1", [{ 交易时间: "2026-08-01 12:00:00", "金额(元)": "25.50", "收/支": "支出", 交易对方: "餐厅", 交易单号: "T001" }], "wechat_import");
  assert.equal(preview.rows[0]?.status, "candidate");
  assert.equal(preview.rows[0]?.amountCents, 2550);
  assert.equal(findProbableDuplicate(preview.rows[0]!, [preview.rows[0]!])?.meta.id, preview.rows[0]?.meta.id);
});

test("global search spans cloud finance and notes", () => {
  const note = createNote("u", "d", "项目计划", "完成 Web 客户端");
  const transaction = createTransaction("u", "d", { amount: "10", type: "expense", merchant: "咖啡店" });
  const state = { cursor: "1", conflicts: [], lastLoadedAt: new Date().toISOString(), entities: { "note.note": { [note.meta.id]: note }, "finance.transaction": { [transaction.meta.id]: transaction } } };
  assert.equal(searchEntities(state, "Web")[0]?.entityType, "note.note");
  assert.equal(searchEntities(state, "咖啡")[0]?.entityType, "finance.transaction");
});
