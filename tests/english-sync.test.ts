import assert from "node:assert/strict";
import test from "node:test";
import { DatabaseSync } from "node:sqlite";
import { decideArticle, sourceStatusAfterSync } from "../src/server/englishSync/decision";
import { calculateContentHash, normalizeContent, normalizeUrl } from "../src/server/englishSync/normalize";
import type { NormalizedEnglishArticle } from "../src/server/englishSync/source";

const article = (overrides: Partial<NormalizedEnglishArticle> = {}): NormalizedEnglishArticle => ({
  source_key: "voa_science",
  external_id: "123",
  source_url: "https://LearningEnglish.VOANews.com/a/story/123.html?utm_source=test#player",
  title: "A useful science story",
  content: "This is an English article with stable content. ".repeat(60),
  category: "Science",
  published_at: "2026-01-01T00:00:00.000Z",
  license_type: "VOA terms apply",
  attribution: "VOA Learning English",
  ...overrides,
});

test("normalizes URLs without dropping meaningful query parameters", () => {
  assert.equal(
    normalizeUrl("HTTPS://Example.COM/path/?b=2&utm_source=x&a=1#part"),
    "https://example.com/path?a=1&b=2",
  );
});

test("normalizes layout-only content changes to the same SHA-256", async () => {
  const first = "Hello\u200b   world.\r\n\r\nThis is a test.";
  const second = "Hello world.\nThis is a test.";
  assert.equal(normalizeContent(first), normalizeContent(second));
  assert.equal(await calculateContentHash(first), await calculateContentHash(second));
});

test("repeated sync skips identical articles and updates changed content", async () => {
  const incoming = article();
  const hash = await calculateContentHash(incoming.content);
  const existing = [{
    id: "voa-123", sourceKey: incoming.source_key, externalId: incoming.external_id,
    normalizedSourceUrl: normalizeUrl(incoming.source_url), contentHash: hash,
  }];
  assert.equal((await decideArticle(incoming, existing)).action, "skip");
  assert.equal((await decideArticle(article({ content: `${incoming.content} Updated.` }), existing)).action, "update");
});

test("different URLs with identical content are detected as content duplicates", async () => {
  const incoming = article({ external_id: "456", source_url: "https://learningenglish.voanews.com/a/other/456.html" });
  const existing = [{ id: "one", contentHash: await calculateContentHash(incoming.content) }];
  assert.equal((await decideArticle(incoming, existing)).action, "duplicate_content");
});

test("a previously failed article is updated in place when detail retry succeeds", async () => {
  const incoming = article();
  const decision = await decideArticle(incoming, [{
    id: "failed-row",
    sourceKey: incoming.source_key,
    externalId: incoming.external_id,
    normalizedSourceUrl: normalizeUrl(incoming.source_url),
  }]);
  assert.equal(decision.action, "update");
  assert.equal(decision.existingId, "failed-row");
});

test("a per-article failure does not stop later decisions", async () => {
  const items = [article(), null, article({ external_id: "789", source_url: "https://learningenglish.voanews.com/a/other/789.html" })];
  let success = 0;
  let failed = 0;
  for (const item of items) {
    try {
      if (!item) throw new Error("mock detail failure");
      await decideArticle(item, []);
      success += 1;
    } catch {
      failed += 1;
    }
  }
  assert.deepEqual({ success, failed }, { success: 2, failed: 1 });
});

test("a successful no-new-article sync remains successful and can become stale", () => {
  assert.equal(sourceStatusAfterSync({
    now: new Date("2026-03-10"), lastNewArticleAt: "2026-01-01",
    consecutiveFailures: 0, requestSucceeded: true, staleAfterDays: 30, errorFailureThreshold: 3,
  }), "stale");
  assert.equal(sourceStatusAfterSync({
    now: new Date("2026-03-10"), lastNewArticleAt: "2026-03-09",
    consecutiveFailures: 0, requestSucceeded: true, staleAfterDays: 30, errorFailureThreshold: 3,
  }), "active");
});

test("consecutive failures become error and 429 becomes rate_limited", () => {
  const common = { now: new Date(), staleAfterDays: 30, errorFailureThreshold: 3 };
  assert.equal(sourceStatusAfterSync({ ...common, consecutiveFailures: 3, requestSucceeded: false }), "error");
  assert.equal(sourceStatusAfterSync({ ...common, consecutiveFailures: 1, requestSucceeded: false, rateLimited: true }), "rate_limited");
});

test("interrupted backfill resumes by skipping persisted identities", async () => {
  const first = article();
  const persisted = [{
    id: "voa-123", sourceKey: first.source_key, externalId: first.external_id,
    normalizedSourceUrl: normalizeUrl(first.source_url), contentHash: await calculateContentHash(first.content),
  }];
  assert.equal((await decideArticle(first, persisted)).action, "skip");
  assert.equal((await decideArticle(article({
    external_id: "999", source_url: "https://learningenglish.voanews.com/a/new/999.html",
    content: "A different English article. ".repeat(80),
  }), persisted)).action, "insert");
});

test("SQLite uniqueness prevents concurrent duplicate articles and running tasks", () => {
  const db = new DatabaseSync(":memory:");
  db.exec(`
    CREATE TABLE articles (id TEXT PRIMARY KEY, source_key TEXT, external_id TEXT, url TEXT);
    CREATE UNIQUE INDEX source_external ON articles(source_key, external_id);
    CREATE UNIQUE INDEX source_url ON articles(url);
    CREATE TABLE tasks (id TEXT PRIMARY KEY, status TEXT);
    CREATE UNIQUE INDEX single_running ON tasks((1)) WHERE status IN ('PENDING','RUNNING');
  `);
  db.prepare("INSERT INTO articles VALUES (?, ?, ?, ?)").run("1", "voa", "123", "https://example.com/a");
  assert.throws(() => db.prepare("INSERT INTO articles VALUES (?, ?, ?, ?)").run("2", "voa", "123", "https://example.com/b"));
  assert.throws(() => db.prepare("INSERT INTO articles VALUES (?, ?, ?, ?)").run("3", "other", "999", "https://example.com/a"));
  db.prepare("INSERT INTO tasks VALUES (?, 'RUNNING')").run("one");
  assert.throws(() => db.prepare("INSERT INTO tasks VALUES (?, 'PENDING')").run("two"));
  db.close();
});
