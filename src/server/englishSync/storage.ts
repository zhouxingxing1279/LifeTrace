import { env } from "cloudflare:workers";
import { DEFAULT_VOA_SOURCES, ENGLISH_SYNC_CONFIG } from "./config";
import { calculateContentHash, normalizeContent, normalizeUrl } from "./normalize";
import type { EnglishArticle, EnglishContentSourceState, EnglishLibraryStats, EnglishSyncLog, EnglishSyncTask } from "@/src/types/english";

const articleColumns: Array<[string, string]> = [
  ["source_key", "text"], ["source_name", "text"], ["source_category", "text"],
  ["external_id", "text"], ["source_url", "text"], ["normalized_source_url", "text"],
  ["title", "text"], ["summary", "text"], ["content", "text"], ["author", "text"],
  ["published_at", "text"], ["source_updated_at", "text"], ["fetched_at", "text"],
  ["created_at", "text"], ["content_hash", "text"], ["word_count", "integer"],
  ["language", "text"], ["cefr_level", "text"], ["estimated_reading_minutes", "integer"],
  ["quality_score", "real"], ["audio_url", "text"], ["image_url", "text"],
  ["has_audio", "integer"], ["license_type", "text"], ["attribution", "text"],
  ["processing_status", "text"], ["fetch_status", "text"], ["retry_count", "integer DEFAULT 0"],
  ["last_error", "text"],
];

let schemaPromise: Promise<void> | undefined;

export function ensureEnglishSyncSchema() {
  schemaPromise ??= (async () => {
    await env.DB.batch([
      env.DB.prepare(`CREATE TABLE IF NOT EXISTS english_articles (
        id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL
      )`),
      env.DB.prepare(`CREATE TABLE IF NOT EXISTS english_content_sources (
        id TEXT PRIMARY KEY, source_key TEXT NOT NULL UNIQUE, source_name TEXT NOT NULL,
        source_type TEXT NOT NULL, source_url TEXT NOT NULL, category TEXT NOT NULL,
        enabled INTEGER NOT NULL DEFAULT 1, sync_interval INTEGER NOT NULL DEFAULT 86400,
        initial_fetch_limit INTEGER NOT NULL DEFAULT 100, recent_scan_limit INTEGER NOT NULL DEFAULT 30,
        overlap_days INTEGER NOT NULL DEFAULT 14, request_interval_ms INTEGER NOT NULL DEFAULT 1000,
        last_sync_at TEXT, last_success_at TEXT, last_new_article_at TEXT,
        latest_external_published_at TEXT, sync_cursor TEXT,
        consecutive_failures INTEGER NOT NULL DEFAULT 0, status TEXT NOT NULL DEFAULT 'active',
        last_error TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
      )`),
      env.DB.prepare(`CREATE TABLE IF NOT EXISTS english_library_state (
        id TEXT PRIMARY KEY, initialization_status TEXT NOT NULL DEFAULT 'not_started',
        initialized_at TEXT, initial_article_count INTEGER NOT NULL DEFAULT 0,
        target_article_count INTEGER NOT NULL DEFAULT 500, current_source_key TEXT,
        last_error TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
      )`),
      env.DB.prepare(`CREATE TABLE IF NOT EXISTS english_sync_tasks (
        task_id TEXT PRIMARY KEY, task_type TEXT NOT NULL, source_key TEXT, requested_limit INTEGER, status TEXT NOT NULL,
        started_at TEXT, finished_at TEXT, total_count INTEGER NOT NULL DEFAULT 0,
        success_count INTEGER NOT NULL DEFAULT 0, inserted_count INTEGER NOT NULL DEFAULT 0,
        updated_count INTEGER NOT NULL DEFAULT 0, skipped_count INTEGER NOT NULL DEFAULT 0,
        failed_count INTEGER NOT NULL DEFAULT 0, current_article TEXT,
        progress REAL NOT NULL DEFAULT 0, last_error TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
      )`),
      env.DB.prepare(`CREATE TABLE IF NOT EXISTS english_sync_logs (
        id TEXT PRIMARY KEY, task_id TEXT NOT NULL, source_key TEXT, level TEXT NOT NULL,
        event TEXT NOT NULL, request_url TEXT, message TEXT NOT NULL,
        retry_count INTEGER NOT NULL DEFAULT 0, duration_ms INTEGER,
        details_json TEXT, created_at TEXT NOT NULL
      )`),
      env.DB.prepare(`CREATE TABLE IF NOT EXISTS english_processing_queue (
        id TEXT PRIMARY KEY, article_id TEXT NOT NULL, job_type TEXT NOT NULL,
        status TEXT NOT NULL DEFAULT 'PENDING', retry_count INTEGER NOT NULL DEFAULT 0,
        last_error TEXT, available_at TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
        UNIQUE(article_id, job_type)
      )`),
    ]);
    const info = await env.DB.prepare("PRAGMA table_info(english_articles)").all<{ name: string }>();
    const existing = new Set(info.results.map((column) => column.name));
    const missing = articleColumns.filter(([name]) => !existing.has(name));
    if (missing.length) {
      await env.DB.batch(missing.map(([name, type]) =>
        env.DB.prepare(`ALTER TABLE english_articles ADD COLUMN ${name} ${type}`),
      ));
    }
    const taskInfo = await env.DB.prepare("PRAGMA table_info(english_sync_tasks)").all<{ name: string }>();
    if (!taskInfo.results.some((column) => column.name === "requested_limit")) {
      await env.DB.prepare("ALTER TABLE english_sync_tasks ADD COLUMN requested_limit INTEGER").run();
    }
    await env.DB.batch([
      env.DB.prepare(`CREATE UNIQUE INDEX IF NOT EXISTS english_articles_source_external_unique
        ON english_articles(source_key, external_id)
        WHERE source_key IS NOT NULL AND external_id IS NOT NULL`),
      env.DB.prepare(`CREATE UNIQUE INDEX IF NOT EXISTS english_articles_source_url_unique
        ON english_articles(normalized_source_url)
        WHERE normalized_source_url IS NOT NULL`),
      env.DB.prepare("CREATE INDEX IF NOT EXISTS english_articles_content_hash_idx ON english_articles(content_hash)"),
      env.DB.prepare("CREATE INDEX IF NOT EXISTS english_sync_tasks_status_idx ON english_sync_tasks(status, created_at)"),
      env.DB.prepare(`CREATE UNIQUE INDEX IF NOT EXISTS english_sync_tasks_single_running
        ON english_sync_tasks((1)) WHERE status IN ('PENDING', 'RUNNING')`),
      env.DB.prepare("CREATE INDEX IF NOT EXISTS english_sync_logs_task_idx ON english_sync_logs(task_id, created_at)"),
    ]);

    const legacyRows = await env.DB.prepare(`
      SELECT id, data_json FROM english_articles
      WHERE source_key IS NULL OR content_hash IS NULL
    `).all<{ id: string; data_json: string }>();
    for (const row of legacyRows.results) {
      try {
        const article = JSON.parse(row.data_json) as EnglishArticle;
        const content = normalizeContent(article.content || "");
        const sourceUrl = article.sourceUrl ? normalizeUrl(article.sourceUrl) : null;
        const hash = content ? await calculateContentHash(content) : null;
        await env.DB.prepare(`
          UPDATE english_articles SET source_key = ?, source_name = ?, source_category = ?,
            external_id = ?, source_url = ?, normalized_source_url = ?, title = ?,
            summary = ?, content = ?, author = ?, published_at = ?, fetched_at = ?,
            created_at = ?, content_hash = ?, word_count = ?, language = ?,
            cefr_level = ?, estimated_reading_minutes = ?, audio_url = ?, image_url = ?,
            has_audio = ?, license_type = ?, attribution = ?, processing_status = ?,
            fetch_status = ?, retry_count = ?, last_error = ?
          WHERE id = ?
        `).bind(
          article.sourceKey ?? article.source ?? null, article.sourceName ?? null,
          article.sourceCategory ?? article.category, article.externalId ?? null,
          article.sourceUrl ?? null, sourceUrl, article.title, article.summary ?? null,
          content, article.author ?? null, article.publishedAt ?? null, article.fetchedAt ?? null,
          article.createdTime, article.contentHash ?? hash, article.wordCount ?? null,
          article.language ?? "en", article.level, article.estimatedMinutes,
          article.audioUrl ?? null, article.imageUrl ?? null, article.audioUrl ? 1 : 0,
          article.licenseType ?? null, article.attribution ?? article.rightsNote ?? null,
          article.processingStatus ?? "READY", article.fetchStatus ?? "SUCCESS",
          article.retryCount ?? 0, article.lastError ?? null, row.id,
        ).run();
      } catch {
        // A malformed legacy row remains readable through data_json and is not allowed
        // to prevent the rest of the English library schema from initializing.
      }
    }

    const now = new Date().toISOString();
    await env.DB.batch(DEFAULT_VOA_SOURCES.map((source) => env.DB.prepare(`
      INSERT INTO english_content_sources (
        id, source_key, source_name, source_type, source_url, category, enabled,
        sync_interval, initial_fetch_limit, recent_scan_limit, overlap_days,
        request_interval_ms, status, created_at, updated_at
      ) VALUES (?, ?, ?, 'voa', ?, ?, 1, ?, ?, ?, ?, 1000, 'active', ?, ?)
      ON CONFLICT(source_key) DO NOTHING
    `).bind(
      source.sourceKey, source.sourceKey, source.sourceName, source.sourceUrl, source.category,
      ENGLISH_SYNC_CONFIG.syncIntervalSeconds, ENGLISH_SYNC_CONFIG.defaultInitialFetchLimit,
      ENGLISH_SYNC_CONFIG.recentScanLimit, ENGLISH_SYNC_CONFIG.overlapDays, now, now,
    )));
    await env.DB.prepare(`
      INSERT INTO english_library_state (
        id, initialization_status, initial_article_count, target_article_count, created_at, updated_at
      ) VALUES ('english_library_initialized', 'not_started', 0, ?, ?, ?)
      ON CONFLICT(id) DO NOTHING
    `).bind(DEFAULT_VOA_SOURCES.length * ENGLISH_SYNC_CONFIG.defaultInitialFetchLimit, now, now).run();
  })().catch((error) => {
    schemaPromise = undefined;
    throw error;
  });
  return schemaPromise;
}

type SourceRow = {
  id: string; source_key: string; source_name: string; source_type: string; source_url: string;
  category: string; enabled: number; sync_interval: number; initial_fetch_limit: number;
  recent_scan_limit: number; overlap_days: number; request_interval_ms: number;
  last_sync_at: string | null; last_success_at: string | null; last_new_article_at: string | null;
  latest_external_published_at: string | null; sync_cursor: string | null;
  consecutive_failures: number; status: EnglishContentSourceState["status"]; last_error: string | null;
  created_at: string; updated_at: string; article_count: number;
};

const sourceFromRow = (row: SourceRow): EnglishContentSourceState => ({
  id: row.id, sourceKey: row.source_key, sourceName: row.source_name,
  sourceType: row.source_type, sourceUrl: row.source_url, category: row.category,
  enabled: Boolean(row.enabled), syncInterval: row.sync_interval,
  initialFetchLimit: row.initial_fetch_limit, recentScanLimit: row.recent_scan_limit,
  overlapDays: row.overlap_days, requestIntervalMs: row.request_interval_ms,
  lastSyncAt: row.last_sync_at ?? undefined, lastSuccessAt: row.last_success_at ?? undefined,
  lastNewArticleAt: row.last_new_article_at ?? undefined,
  latestExternalPublishedAt: row.latest_external_published_at ?? undefined,
  syncCursor: row.sync_cursor ?? undefined, consecutiveFailures: row.consecutive_failures,
  status: row.status, lastError: row.last_error ?? undefined, articleCount: row.article_count,
  createdAt: row.created_at, updatedAt: row.updated_at,
});

export async function listSourceStates(includeDisabled = true) {
  await ensureEnglishSyncSchema();
  const rows = await env.DB.prepare(`
    SELECT s.*, COUNT(a.id) AS article_count
    FROM english_content_sources s
    LEFT JOIN english_articles a ON a.source_key = s.source_key
    ${includeDisabled ? "" : "WHERE s.enabled = 1"}
    GROUP BY s.id ORDER BY s.source_name
  `).all<SourceRow>();
  return rows.results.map(sourceFromRow);
}

export async function setSourceEnabled(sourceKey: string, enabled: boolean) {
  await ensureEnglishSyncSchema();
  const now = new Date().toISOString();
  await env.DB.prepare(`
    UPDATE english_content_sources SET enabled = ?, status = ?, updated_at = ? WHERE source_key = ?
  `).bind(enabled ? 1 : 0, enabled ? "active" : "disabled", now, sourceKey).run();
  return (await listSourceStates()).find((source) => source.sourceKey === sourceKey);
}

export async function createTask(taskType: EnglishSyncTask["taskType"], sourceKey?: string, requestedLimit?: number) {
  await ensureEnglishSyncSchema();
  const taskId = crypto.randomUUID();
  const now = new Date().toISOString();
  const abandonedBefore = new Date(Date.now() - 30 * 60_000).toISOString();
  await env.DB.prepare(`
    UPDATE english_sync_tasks SET status = 'PARTIAL_SUCCESS', finished_at = ?,
      last_error = COALESCE(last_error, '应用中断，任务已释放；再次执行将从已保存内容继续'),
      updated_at = ?
    WHERE status IN ('PENDING','RUNNING') AND updated_at < ?
  `).bind(now, now, abandonedBefore).run();
  try {
    await env.DB.prepare(`
      INSERT INTO english_sync_tasks (
        task_id, task_type, source_key, requested_limit, status, total_count, success_count,
        inserted_count, updated_count, skipped_count, failed_count, progress, created_at, updated_at
      ) VALUES (?, ?, ?, ?, 'PENDING', 0, 0, 0, 0, 0, 0, 0, ?, ?)
    `).bind(taskId, taskType, sourceKey ?? null, requestedLimit ?? null, now, now).run();
  } catch (error) {
    const running = await getActiveTask();
    if (running) return { task: running, created: false };
    throw error;
  }
  return { task: await getTask(taskId) as EnglishSyncTask, created: true };
}

type TaskRow = {
  task_id: string; task_type: EnglishSyncTask["taskType"]; source_key: string | null;
  requested_limit: number | null;
  status: EnglishSyncTask["status"]; started_at: string | null; finished_at: string | null;
  total_count: number; success_count: number; inserted_count: number; updated_count: number;
  skipped_count: number; failed_count: number; current_article: string | null; progress: number;
  last_error: string | null; created_at: string; updated_at: string;
};

const taskFromRow = (row: TaskRow): EnglishSyncTask => ({
  taskId: row.task_id, taskType: row.task_type, sourceKey: row.source_key ?? undefined,
  requestedLimit: row.requested_limit ?? undefined,
  status: row.status, startedAt: row.started_at ?? undefined, finishedAt: row.finished_at ?? undefined,
  totalCount: row.total_count, successCount: row.success_count, insertedCount: row.inserted_count,
  updatedCount: row.updated_count, skippedCount: row.skipped_count, failedCount: row.failed_count,
  currentArticle: row.current_article ?? undefined, progress: row.progress,
  lastError: row.last_error ?? undefined, createdAt: row.created_at, updatedAt: row.updated_at,
});

export async function getTask(taskId: string) {
  await ensureEnglishSyncSchema();
  const row = await env.DB.prepare("SELECT * FROM english_sync_tasks WHERE task_id = ?")
    .bind(taskId).first<TaskRow>();
  return row ? taskFromRow(row) : undefined;
}

export async function getActiveTask() {
  await ensureEnglishSyncSchema();
  const row = await env.DB.prepare(`
    SELECT * FROM english_sync_tasks WHERE status IN ('PENDING','RUNNING') ORDER BY created_at DESC LIMIT 1
  `).first<TaskRow>();
  return row ? taskFromRow(row) : undefined;
}

export async function listTasks(limit = 20) {
  await ensureEnglishSyncSchema();
  const rows = await env.DB.prepare("SELECT * FROM english_sync_tasks ORDER BY created_at DESC LIMIT ?")
    .bind(Math.max(1, Math.min(limit, 100))).all<TaskRow>();
  return rows.results.map(taskFromRow);
}

export async function patchTask(taskId: string, patch: Partial<{
  status: EnglishSyncTask["status"]; startedAt: string; finishedAt: string; totalCount: number;
  successCount: number; insertedCount: number; updatedCount: number; skippedCount: number;
  failedCount: number; currentArticle: string | null; progress: number; lastError: string | null;
}>) {
  const map: Record<string, string> = {
    status: "status", startedAt: "started_at", finishedAt: "finished_at", totalCount: "total_count",
    successCount: "success_count", insertedCount: "inserted_count", updatedCount: "updated_count",
    skippedCount: "skipped_count", failedCount: "failed_count", currentArticle: "current_article",
    progress: "progress", lastError: "last_error",
  };
  const entries = Object.entries(patch).filter(([key]) => map[key]);
  if (!entries.length) return getTask(taskId);
  const values = entries.map(([, value]) => value);
  const sql = entries.map(([key]) => `${map[key]} = ?`).join(", ");
  await env.DB.prepare(`UPDATE english_sync_tasks SET ${sql}, updated_at = ? WHERE task_id = ?`)
    .bind(...values, new Date().toISOString(), taskId).run();
  return getTask(taskId);
}

export async function addLog(input: Omit<EnglishSyncLog, "id" | "createdAt">) {
  const id = crypto.randomUUID();
  const createdAt = new Date().toISOString();
  await env.DB.prepare(`
    INSERT INTO english_sync_logs (
      id, task_id, source_key, level, event, request_url, message,
      retry_count, duration_ms, details_json, created_at
    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
  `).bind(
    id, input.taskId, input.sourceKey ?? null, input.level, input.event,
    input.requestUrl ?? null, input.message, input.retryCount, input.durationMs ?? null,
    input.details ? JSON.stringify(input.details) : null, createdAt,
  ).run();
}

export async function listLogs(taskId?: string, limit = 100): Promise<EnglishSyncLog[]> {
  await ensureEnglishSyncSchema();
  type Row = {
    id: string; task_id: string; source_key: string | null; level: EnglishSyncLog["level"];
    event: string; request_url: string | null; message: string; retry_count: number;
    duration_ms: number | null; details_json: string | null; created_at: string;
  };
  const statement = taskId
    ? env.DB.prepare("SELECT * FROM english_sync_logs WHERE task_id = ? ORDER BY created_at DESC LIMIT ?").bind(taskId, limit)
    : env.DB.prepare("SELECT * FROM english_sync_logs ORDER BY created_at DESC LIMIT ?").bind(limit);
  const rows = await statement.all<Row>();
  return rows.results.map((row) => ({
    id: row.id, taskId: row.task_id, sourceKey: row.source_key ?? undefined,
    level: row.level, event: row.event, requestUrl: row.request_url ?? undefined,
    message: row.message, retryCount: row.retry_count, durationMs: row.duration_ms ?? undefined,
    details: row.details_json ? JSON.parse(row.details_json) : undefined, createdAt: row.created_at,
  }));
}

export async function existingArticleIdentities() {
  await ensureEnglishSyncSchema();
  return (await env.DB.prepare(`
    SELECT id, source_key AS sourceKey, external_id AS externalId,
      normalized_source_url AS normalizedSourceUrl, content_hash AS contentHash,
      title, published_at AS publishedAt FROM english_articles
  `).all<{
    id: string; sourceKey?: string; externalId?: string; normalizedSourceUrl?: string;
    contentHash?: string; title?: string; publishedAt?: string;
  }>()).results;
}

export async function upsertSyncedArticle(article: EnglishArticle, normalizedSourceUrl: string, existingId?: string) {
  const id = existingId ?? article.id;
  const stored = { ...article, id, normalizedSourceUrl };
  const statement = env.DB.prepare(`
    INSERT INTO english_articles (
      id, data_json, updated_at, source_key, source_name, source_category, external_id,
      source_url, normalized_source_url, title, summary, content, author, published_at,
      source_updated_at, fetched_at, created_at, content_hash, word_count, language,
      cefr_level, estimated_reading_minutes, quality_score, audio_url, image_url,
      has_audio, license_type, attribution, processing_status, fetch_status, retry_count, last_error
    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    ON CONFLICT(id) DO UPDATE SET
      data_json=excluded.data_json, updated_at=excluded.updated_at, source_key=excluded.source_key,
      source_name=excluded.source_name, source_category=excluded.source_category,
      external_id=excluded.external_id, source_url=excluded.source_url,
      normalized_source_url=excluded.normalized_source_url, title=excluded.title,
      summary=excluded.summary, content=excluded.content, author=excluded.author,
      published_at=excluded.published_at, source_updated_at=excluded.source_updated_at,
      fetched_at=excluded.fetched_at, content_hash=excluded.content_hash,
      word_count=excluded.word_count, language=excluded.language, cefr_level=excluded.cefr_level,
      estimated_reading_minutes=excluded.estimated_reading_minutes,
      quality_score=excluded.quality_score, audio_url=excluded.audio_url,
      image_url=excluded.image_url, has_audio=excluded.has_audio,
      license_type=excluded.license_type, attribution=excluded.attribution,
      processing_status=excluded.processing_status, fetch_status=excluded.fetch_status,
      retry_count=excluded.retry_count, last_error=excluded.last_error
  `).bind(
    id, JSON.stringify(stored), article.updatedAt, article.sourceKey ?? null, article.sourceName ?? null,
    article.sourceCategory ?? null, article.externalId ?? null, article.sourceUrl ?? null,
    normalizedSourceUrl, article.title, article.summary ?? null, article.content, article.author ?? null,
    article.publishedAt ?? null, article.sourceUpdatedAt ?? null, article.fetchedAt ?? null,
    article.createdTime, article.contentHash ?? null, article.wordCount ?? null, article.language ?? "en",
    article.level, article.estimatedMinutes, article.qualityScore ?? null, article.audioUrl ?? null,
    article.imageUrl ?? null, article.hasAudio ? 1 : 0, article.licenseType ?? null,
    article.attribution ?? null, article.processingStatus ?? "FETCHED", article.fetchStatus ?? "SUCCESS",
    article.retryCount ?? 0, article.lastError ?? null,
  );
  await statement.run();
  const now = new Date().toISOString();
  await env.DB.prepare(`
    INSERT INTO english_processing_queue (
      id, article_id, job_type, status, retry_count, available_at, created_at, updated_at
    ) VALUES (?, ?, 'AI_ENRICHMENT', 'PENDING', 0, ?, ?, ?)
    ON CONFLICT(article_id, job_type) DO NOTHING
  `).bind(crypto.randomUUID(), id, now, now, now).run();
  return id;
}

export async function failedArticleRows(sourceKey?: string) {
  await ensureEnglishSyncSchema();
  const statement = sourceKey
    ? env.DB.prepare("SELECT id, source_key, source_url FROM english_articles WHERE fetch_status = 'FAILED' AND source_key = ?").bind(sourceKey)
    : env.DB.prepare("SELECT id, source_key, source_url FROM english_articles WHERE fetch_status = 'FAILED'");
  return (await statement.all<{ id: string; source_key: string; source_url: string }>()).results;
}

export async function resetFailedProcessingJobs(sourceKey?: string) {
  await ensureEnglishSyncSchema();
  const now = new Date().toISOString();
  const result = sourceKey
    ? await env.DB.prepare(`
        UPDATE english_processing_queue SET status = 'PENDING', last_error = NULL,
          available_at = ?, updated_at = ?
        WHERE status = 'FAILED' AND article_id IN (
          SELECT id FROM english_articles WHERE source_key = ?
        )
      `).bind(now, now, sourceKey).run()
    : await env.DB.prepare(`
        UPDATE english_processing_queue SET status = 'PENDING', last_error = NULL,
          available_at = ?, updated_at = ? WHERE status = 'FAILED'
      `).bind(now, now).run();
  return result.meta.changes ?? 0;
}

export async function recordFetchFailure(input: {
  sourceKey: string; sourceName: string; category: string; sourceUrl: string;
  title?: string; error: string;
}) {
  const now = new Date().toISOString();
  const id = `failed-${input.sourceKey}-${await shortHash(input.sourceUrl)}`;
  const data: EnglishArticle = {
    id, title: input.title || "抓取失败，等待重试", level: "B1", category: "Life",
    content: "", vocabulary: [], questions: [], difficulty: 3, estimatedMinutes: 5,
    source: "voa", sourceKey: input.sourceKey, sourceName: input.sourceName,
    sourceCategory: input.category, sourceUrl: input.sourceUrl,
    processingStatus: "FAILED", fetchStatus: "FAILED", retryCount: 1,
    lastError: input.error, createdTime: now, updatedAt: now,
  };
  await env.DB.prepare(`
    INSERT INTO english_articles (
      id, data_json, updated_at, source_key, source_name, source_category,
      source_url, normalized_source_url, title, content, created_at,
      processing_status, fetch_status, retry_count, last_error
    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, '', ?, 'FAILED', 'FAILED', 1, ?)
    ON CONFLICT(normalized_source_url) DO UPDATE SET
      fetch_status='FAILED', processing_status='FAILED',
      retry_count=COALESCE(english_articles.retry_count, 0) + 1,
      last_error=excluded.last_error, updated_at=excluded.updated_at
  `).bind(
    id, JSON.stringify(data), now, input.sourceKey, input.sourceName, input.category,
    input.sourceUrl, input.sourceUrl, data.title, now, input.error,
  ).run();
}

async function shortHash(value: string) {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(value));
  return [...new Uint8Array(digest)].slice(0, 8).map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

export async function updateSourceAfterSync(source: EnglishContentSourceState, input: {
  ok: boolean; newCount: number; status: EnglishContentSourceState["status"];
  error?: string; latestPublishedAt?: string; cursor?: string;
}) {
  const now = new Date().toISOString();
  const failures = input.ok ? 0 : source.consecutiveFailures + 1;
  await env.DB.prepare(`
    UPDATE english_content_sources SET last_sync_at = ?, last_success_at = ?,
      last_new_article_at = CASE WHEN ? > 0 THEN ? ELSE last_new_article_at END,
      latest_external_published_at = COALESCE(?, latest_external_published_at),
      sync_cursor = COALESCE(?, sync_cursor), consecutive_failures = ?, status = ?,
      last_error = ?, updated_at = ? WHERE source_key = ?
  `).bind(
    now, input.ok ? now : source.lastSuccessAt ?? null, input.newCount, now,
    input.latestPublishedAt ?? null, input.cursor ?? null, failures, input.status,
    input.error ?? null, now, source.sourceKey,
  ).run();
}

export async function patchLibraryState(input: {
  status: "not_started" | "running" | "completed" | "failed";
  count?: number; currentSourceKey?: string | null; error?: string | null;
}) {
  const now = new Date().toISOString();
  await env.DB.prepare(`
    UPDATE english_library_state SET initialization_status = ?,
      initialized_at = CASE WHEN ? = 'completed' THEN ? ELSE initialized_at END,
      initial_article_count = COALESCE(?, initial_article_count),
      current_source_key = ?, last_error = ?, updated_at = ?
    WHERE id = 'english_library_initialized'
  `).bind(input.status, input.status, now, input.count ?? null, input.currentSourceKey ?? null, input.error ?? null, now).run();
}

export async function getLibraryStats(): Promise<EnglishLibraryStats> {
  await ensureEnglishSyncSchema();
  const aggregate = await env.DB.prepare(`
    SELECT COUNT(*) total,
      SUM(CASE WHEN processing_status = 'READY' OR processing_status IS NULL THEN 1 ELSE 0 END) ready,
      SUM(CASE WHEN processing_status IN ('FETCHED','CLEANED','ANALYZED') THEN 1 ELSE 0 END) pending,
      SUM(CASE WHEN processing_status = 'FAILED' OR fetch_status = 'FAILED' THEN 1 ELSE 0 END) failed,
      SUM(CASE WHEN processing_status = 'REJECTED' THEN 1 ELSE 0 END) rejected,
      SUM(CASE WHEN has_audio = 1 OR json_extract(data_json, '$.audioUrl') IS NOT NULL THEN 1 ELSE 0 END) with_audio
    FROM english_articles
  `).first<{ total: number; ready: number; pending: number; failed: number; rejected: number; with_audio: number }>();
  const byCefrRows = await env.DB.prepare(`
    SELECT COALESCE(cefr_level, json_extract(data_json, '$.level'), 'unknown') label, COUNT(*) count
    FROM english_articles GROUP BY label
  `).all<{ label: string; count: number }>();
  const byCategoryRows = await env.DB.prepare(`
    SELECT COALESCE(source_category, json_extract(data_json, '$.category'), 'unknown') label, COUNT(*) count
    FROM english_articles GROUP BY label
  `).all<{ label: string; count: number }>();
  const sourceTimes = await env.DB.prepare(`
    SELECT MAX(last_sync_at) last_sync_at, MAX(last_new_article_at) last_new_article_at
    FROM english_content_sources
  `).first<{ last_sync_at: string | null; last_new_article_at: string | null }>();
  const state = await env.DB.prepare("SELECT * FROM english_library_state WHERE id = 'english_library_initialized'")
    .first<{
      initialization_status: EnglishLibraryStats["initialization"]["status"];
      initialized_at: string | null; initial_article_count: number; target_article_count: number;
      current_source_key: string | null; last_error: string | null;
    }>();
  return {
    total: aggregate?.total ?? 0, ready: aggregate?.ready ?? 0, pending: aggregate?.pending ?? 0,
    failed: aggregate?.failed ?? 0, rejected: aggregate?.rejected ?? 0, withAudio: aggregate?.with_audio ?? 0,
    byCefr: Object.fromEntries(byCefrRows.results.map((row) => [row.label, row.count])),
    byCategory: Object.fromEntries(byCategoryRows.results.map((row) => [row.label, row.count])),
    lastSyncAt: sourceTimes?.last_sync_at ?? undefined,
    lastNewArticleAt: sourceTimes?.last_new_article_at ?? undefined,
    initialization: {
      status: state?.initialization_status ?? "not_started",
      initializedAt: state?.initialized_at ?? undefined,
      initialArticleCount: state?.initial_article_count ?? 0,
      targetArticleCount: state?.target_article_count ?? 500,
      currentSourceKey: state?.current_source_key ?? undefined,
      lastError: state?.last_error ?? undefined,
    },
  };
}

export async function getSourceHealthMetrics(sourceKey: string) {
  await ensureEnglishSyncSchema();
  const metrics = await env.DB.prepare(`
    SELECT COUNT(*) total,
      SUM(CASE WHEN content IS NOT NULL AND LENGTH(content) > 0 THEN 1 ELSE 0 END) parsed,
      SUM(CASE WHEN word_count IS NOT NULL AND word_count < ? THEN 1 ELSE 0 END) short_articles,
      SUM(CASE WHEN has_audio = 1 THEN 1 ELSE 0 END) with_audio,
      SUM(CASE WHEN fetch_status = 'FAILED' THEN 1 ELSE 0 END) failed
    FROM english_articles WHERE source_key = ?
  `).bind(ENGLISH_SYNC_CONFIG.minimumWords, sourceKey).first<{
    total: number; parsed: number; short_articles: number; with_audio: number; failed: number;
  }>();
  const duplicates = await env.DB.prepare(`
    SELECT COUNT(*) count FROM (
      SELECT content_hash FROM english_articles
      WHERE source_key = ? AND content_hash IS NOT NULL
      GROUP BY content_hash HAVING COUNT(*) > 1
    )
  `).bind(sourceKey).first<{ count: number }>();
  const total = metrics?.total ?? 0;
  return {
    total,
    parserSuccessRate: total ? (metrics?.parsed ?? 0) / total : 1,
    shortArticles: metrics?.short_articles ?? 0,
    withAudio: metrics?.with_audio ?? 0,
    failed: metrics?.failed ?? 0,
    duplicateGroups: duplicates?.count ?? 0,
  };
}
