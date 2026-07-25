import { VoaPythonSource, toLegacyEnglishArticle } from "@/src/server/englishSources/voaPython";
import type { EnglishContentSourceState, EnglishSyncTask } from "@/src/types/english";
import { DEFAULT_VOA_SOURCES, ENGLISH_SYNC_CONFIG } from "./config";
import { decideArticle, sourceStatusAfterSync } from "./decision";
import { countWords, englishRatio, normalizeContent } from "./normalize";
import type { EnglishContentSource, NormalizedEnglishArticle, SourceFetchResult } from "./source";
import {
  addLog,
  createTask,
  existingArticleIdentities,
  failedArticleRows,
  getLibraryStats,
  getSourceHealthMetrics,
  getTask,
  listSourceStates,
  patchLibraryState,
  patchTask,
  recordFetchFailure,
  resetFailedProcessingJobs,
  updateSourceAfterSync,
  upsertSyncedArticle,
} from "./storage";

const sourceFactory = (state: EnglishContentSourceState): EnglishContentSource => {
  const config = DEFAULT_VOA_SOURCES.find((source) => source.sourceKey === state.sourceKey);
  if (!config || state.sourceType !== "voa") throw new Error(`暂不支持数据源类型：${state.sourceType}`);
  return new VoaPythonSource(state.sourceKey, config.feedKey);
};

const quality = (article: NormalizedEnglishArticle) => {
  const content = normalizeContent(article.content);
  const words = countWords(content);
  const ratio = englishRatio(content);
  let score = 100;
  const reasons: string[] = [];
  if (!article.title.trim() || article.title.trim().length < 5) { score -= 30; reasons.push("标题无效"); }
  if (words < ENGLISH_SYNC_CONFIG.minimumWords) { score -= 45; reasons.push(`正文少于 ${ENGLISH_SYNC_CONFIG.minimumWords} 词`); }
  if (words > ENGLISH_SYNC_CONFIG.maximumWords) { score -= 20; reasons.push(`正文超过 ${ENGLISH_SYNC_CONFIG.maximumWords} 词`); }
  if (ratio < ENGLISH_SYNC_CONFIG.minimumEnglishRatio) { score -= 40; reasons.push("英文内容占比不足"); }
  if (!article.attribution || !article.license_type) { score -= 20; reasons.push("缺少来源或授权说明"); }
  const boilerplateHits = (content.match(/\b(subscribe|cookie policy|privacy policy|follow us|all rights reserved)\b/gi) ?? []).length;
  if (boilerplateHits >= 4) { score -= 25; reasons.push("疑似包含大量导航或页脚"); }
  score = Math.max(0, score);
  const sentences = content.split(/[.!?]+/).filter((item) => item.trim()).length || 1;
  const averageSentenceLength = words / sentences;
  const longWordRatio = (content.match(/\b[A-Za-z]{9,}\b/g)?.length ?? 0) / Math.max(words, 1);
  const complexity = averageSentenceLength + longWordRatio * 80;
  const cefrLevel = complexity < 12 ? "A2" as const
    : complexity < 18 ? "B1" as const
      : complexity < 25 ? "B2" as const
        : "C1" as const;
  return {
    content, words, score, reasons, cefrLevel,
    status: score >= ENGLISH_SYNC_CONFIG.minimumQualityScore ? "READY" as const : "REJECTED" as const,
  };
};

type Counts = Pick<EnglishSyncTask,
  "totalCount" | "successCount" | "insertedCount" | "updatedCount" | "skippedCount" | "failedCount"
>;
const emptyCounts = (): Counts => ({
  totalCount: 0, successCount: 0, insertedCount: 0,
  updatedCount: 0, skippedCount: 0, failedCount: 0,
});

async function processFetchResult(
  task: EnglishSyncTask,
  source: EnglishContentSourceState,
  result: SourceFetchResult,
  counts: Counts,
  addToTotal = true,
) {
  const identities = await existingArticleIdentities();
  if (addToTotal) counts.totalCount += result.articles.length + result.failed.length;
  let index = 0;
  for (const incoming of result.articles) {
    index += 1;
    await patchTask(task.taskId, {
      totalCount: counts.totalCount, currentArticle: incoming.title,
      progress: counts.totalCount ? (counts.successCount + counts.failedCount) / counts.totalCount : 0,
    });
    try {
      const assessment = quality(incoming);
      const decision = await decideArticle(incoming, identities);
      if (decision.action === "skip" || decision.action === "duplicate_content") {
        counts.skippedCount += 1;
        counts.successCount += 1;
        continue;
      }
      const id = decision.existingId ?? `${incoming.source_key}-${incoming.external_id || crypto.randomUUID()}`;
      const legacy = toLegacyEnglishArticle(
        { ...incoming, content: assessment.content },
        id,
        decision.contentHash,
        assessment.score,
        assessment.status,
        assessment.cefrLevel,
      );
      if (assessment.reasons.length) legacy.lastError = assessment.reasons.join("；");
      await upsertSyncedArticle(legacy, decision.normalizedSourceUrl, decision.existingId);
      identities.push({
        id, sourceKey: incoming.source_key, externalId: incoming.external_id,
        normalizedSourceUrl: decision.normalizedSourceUrl, contentHash: decision.contentHash,
        title: incoming.title, publishedAt: incoming.published_at,
      });
      if (decision.action === "insert") counts.insertedCount += 1;
      else counts.updatedCount += 1;
      counts.successCount += 1;
    } catch (error) {
      counts.failedCount += 1;
      await addLog({
        taskId: task.taskId, sourceKey: source.sourceKey, level: "error",
        event: "article_failed", requestUrl: incoming.source_url,
        message: error instanceof Error ? error.message : "文章写入失败", retryCount: 0,
      });
    } finally {
      await patchTask(task.taskId, {
        ...counts, progress: counts.totalCount
          ? Math.min(1, (counts.successCount + counts.failedCount) / counts.totalCount)
          : 1,
      });
    }
  }
  for (const failure of result.failed) {
    counts.failedCount += 1;
    if (failure.sourceUrl) {
      await recordFetchFailure({
        sourceKey: source.sourceKey, sourceName: source.sourceName, category: source.category,
        sourceUrl: failure.sourceUrl, title: failure.title, error: failure.error,
      });
    }
    await addLog({
      taskId: task.taskId, sourceKey: source.sourceKey, level: "error",
      event: "fetch_failed", requestUrl: failure.sourceUrl, message: failure.error,
      retryCount: failure.retryCount ?? 0,
    });
  }
  await patchTask(task.taskId, {
    ...counts, progress: counts.totalCount
      ? Math.min(1, (counts.successCount + counts.failedCount) / counts.totalCount)
      : 1,
  });
  void index;
}

async function runSource(
  task: EnglishSyncTask,
  source: EnglishContentSourceState,
  counts: Counts,
  mode: "latest" | "history" | "repair",
  overrideLimit?: number,
  fixedTotal = false,
) {
  const started = Date.now();
  const adapter = sourceFactory(source);
  await addLog({
    taskId: task.taskId, sourceKey: source.sourceKey, level: "info",
    event: "source_started", requestUrl: source.sourceUrl,
    message: `开始${mode === "history" ? "历史回填" : mode === "repair" ? "补漏扫描" : "增量同步"}`,
    retryCount: 0,
  });
  try {
    let result: SourceFetchResult;
    const options = {
      limit: overrideLimit ?? (mode === "history" ? source.initialFetchLimit : source.recentScanLimit),
      overlapDays: source.overlapDays,
      cursor: mode === "history" ? source.syncCursor : undefined,
      requestIntervalMs: source.requestIntervalMs,
    };
    if (mode === "history") result = await adapter.fetchHistory(options);
    else result = await adapter.fetchLatest({ ...options, limit: mode === "repair" ? ENGLISH_SYNC_CONFIG.weeklyScanLimit : options.limit });
    const before = counts.insertedCount;
    await processFetchResult(task, source, result, counts, !fixedTotal);
    const inserted = counts.insertedCount - before;
    const latestPublishedAt = result.articles
      .map((article) => article.published_at)
      .filter((value): value is string => Boolean(value))
      .sort().at(-1);
    const effectiveLastNew = inserted ? new Date().toISOString() : source.lastNewArticleAt;
    const status = sourceStatusAfterSync({
      now: new Date(), lastNewArticleAt: effectiveLastNew,
      consecutiveFailures: 0, requestSucceeded: true,
      staleAfterDays: ENGLISH_SYNC_CONFIG.staleAfterDays,
      errorFailureThreshold: ENGLISH_SYNC_CONFIG.errorFailureThreshold,
    });
    await updateSourceAfterSync(source, {
      ok: true, newCount: inserted, status,
      latestPublishedAt, cursor: result.nextCursor,
    });
    await addLog({
      taskId: task.taskId, sourceKey: source.sourceKey, level: result.failed.length ? "warning" : "info",
      event: "source_finished", requestUrl: source.sourceUrl,
      message: `发现 ${result.discoveredCount} 篇，新增 ${inserted} 篇，单篇失败 ${result.failed.length} 篇`,
      retryCount: 0, durationMs: Date.now() - started,
      details: { discovered: result.discoveredCount, inserted, failed: result.failed.length },
    });
    return true;
  } catch (error) {
    counts.failedCount += 1;
    const rateLimited = Boolean((error as Error & { rateLimited?: boolean }).rateLimited);
    const failures = source.consecutiveFailures + 1;
    const status = sourceStatusAfterSync({
      now: new Date(), lastNewArticleAt: source.lastNewArticleAt,
      consecutiveFailures: failures, requestSucceeded: false, rateLimited,
      staleAfterDays: ENGLISH_SYNC_CONFIG.staleAfterDays,
      errorFailureThreshold: ENGLISH_SYNC_CONFIG.errorFailureThreshold,
    });
    const message = error instanceof Error ? error.message : "数据源同步失败";
    await updateSourceAfterSync(source, { ok: false, newCount: 0, status, error: message });
    await addLog({
      taskId: task.taskId, sourceKey: source.sourceKey, level: "error",
      event: "source_failed", requestUrl: source.sourceUrl, message,
      retryCount: failures, durationMs: Date.now() - started,
    });
    return false;
  }
}

async function retryFailed(task: EnglishSyncTask, sources: EnglishContentSourceState[], counts: Counts) {
  const resetJobs = await resetFailedProcessingJobs(task.sourceKey);
  if (resetJobs) {
    await addLog({
      taskId: task.taskId, sourceKey: task.sourceKey, level: "info",
      event: "processing_requeued", message: `${resetJobs} 个 AI 分析任务已重新排队`,
      retryCount: 0,
    });
  }
  const rows = await failedArticleRows(task.sourceKey);
  counts.totalCount = rows.length;
  for (const [index, row] of rows.entries()) {
    const state = sources.find((source) => source.sourceKey === row.source_key);
    if (!state) { counts.failedCount += 1; continue; }
    try {
      const article = await sourceFactory(state).fetchArticleDetail(row.source_url);
      await processFetchResult(task, state, {
        articles: [article], failed: [], discoveredCount: 1,
      }, counts, false);
    } catch (error) {
      counts.failedCount += 1;
      await recordFetchFailure({
        sourceKey: state.sourceKey, sourceName: state.sourceName, category: state.category,
        sourceUrl: row.source_url, error: error instanceof Error ? error.message : "重试失败",
      });
    }
    await patchTask(task.taskId, { ...counts, progress: rows.length ? (index + 1) / rows.length : 1 });
  }
}

export async function runEnglishSyncTask(taskId: string) {
  const task = await getTask(taskId);
  if (!task || !["PENDING", "RUNNING"].includes(task.status)) return task;
  const startedAt = task.startedAt ?? new Date().toISOString();
  await patchTask(taskId, { status: "RUNNING", startedAt });
  await addLog({
    taskId, sourceKey: task.sourceKey, level: "info", event: "task_started",
    message: `同步任务开始：${task.taskType}`, retryCount: 0,
  });
  const counts = emptyCounts();
  let successfulSources = 0;
  try {
    const allSources = await listSourceStates(false);
    const sources = task.sourceKey ? allSources.filter((source) => source.sourceKey === task.sourceKey) : allSources;
    if (!sources.length) throw new Error(task.sourceKey ? "指定数据源不存在或已禁用" : "没有启用的数据源");

    if (task.taskType === "backfill") {
      await patchLibraryState({ status: "running", currentSourceKey: sources[0]?.sourceKey });
      counts.totalCount = sources.reduce(
        (total, source) => total + (task.requestedLimit ?? source.initialFetchLimit),
        0,
      );
      await patchTask(taskId, { totalCount: counts.totalCount });
    }
    if (task.taskType === "retry_failed") {
      await retryFailed(task, sources, counts);
    } else if (task.taskType === "monthly_health") {
      counts.totalCount = sources.length;
      for (const source of sources) {
        try {
          const health = await sourceFactory(source).healthCheck();
          if (!health.ok) throw new Error(health.detail || "健康检查失败");
          const metrics = await getSourceHealthMetrics(source.sourceKey);
          const status = sourceStatusAfterSync({
            now: new Date(), lastNewArticleAt: source.lastNewArticleAt,
            consecutiveFailures: 0, requestSucceeded: true,
            staleAfterDays: ENGLISH_SYNC_CONFIG.staleAfterDays,
            errorFailureThreshold: ENGLISH_SYNC_CONFIG.errorFailureThreshold,
          });
          await updateSourceAfterSync(source, { ok: true, newCount: 0, status });
          counts.successCount += 1;
          successfulSources += 1;
          await addLog({
            taskId, sourceKey: source.sourceKey, level: "info", event: "health_ok",
            requestUrl: source.sourceUrl,
            message: `数据源可访问；解析成功率 ${Math.round(metrics.parserSuccessRate * 100)}%；重复组 ${metrics.duplicateGroups}；异常短文 ${metrics.shortArticles}；带音频 ${metrics.withAudio}`,
            retryCount: 0, details: metrics,
          });
        } catch (error) {
          counts.failedCount += 1;
          const rateLimited = Boolean((error as Error & { rateLimited?: boolean }).rateLimited);
          const status = sourceStatusAfterSync({
            now: new Date(), lastNewArticleAt: source.lastNewArticleAt,
            consecutiveFailures: source.consecutiveFailures + 1, requestSucceeded: false, rateLimited,
            staleAfterDays: ENGLISH_SYNC_CONFIG.staleAfterDays,
            errorFailureThreshold: ENGLISH_SYNC_CONFIG.errorFailureThreshold,
          });
          await updateSourceAfterSync(source, {
            ok: false, newCount: 0, status,
            error: error instanceof Error ? error.message : "健康检查失败",
          });
          await addLog({
            taskId, sourceKey: source.sourceKey, level: "error", event: "health_failed",
            requestUrl: source.sourceUrl, message: error instanceof Error ? error.message : "健康检查失败",
            retryCount: source.consecutiveFailures,
          });
        }
        await patchTask(taskId, { ...counts, progress: (counts.successCount + counts.failedCount) / counts.totalCount });
      }
    } else {
      const mode = task.taskType === "backfill" ? "history" : task.taskType === "weekly_repair" ? "repair" : "latest";
      for (const source of sources) {
        if (task.taskType === "backfill") {
          const library = await getLibraryStats();
          await patchLibraryState({ status: "running", count: library.total, currentSourceKey: source.sourceKey });
        }
        if (await runSource(
          task,
          source,
          counts,
          mode,
          task.taskType === "backfill" ? task.requestedLimit : undefined,
          task.taskType === "backfill",
        )) {
          successfulSources += 1;
        }
      }
    }

    const finalStatus = counts.failedCount === 0
      ? "COMPLETED"
      : successfulSources > 0 || counts.successCount > 0 ? "PARTIAL_SUCCESS" : "FAILED";
    await patchTask(taskId, {
      ...counts, status: finalStatus, progress: 1,
      finishedAt: new Date().toISOString(), currentArticle: null,
      lastError: finalStatus === "FAILED" ? "所有数据源或文章均处理失败" : null,
    });
    if (task.taskType === "backfill") {
      const library = await getLibraryStats();
      await patchLibraryState({
        status: finalStatus === "FAILED" ? "failed" : "completed",
        count: library.total, currentSourceKey: null,
        error: finalStatus === "FAILED" ? "初始化未能导入文章" : null,
      });
    }
    await addLog({
      taskId, sourceKey: task.sourceKey, level: finalStatus === "FAILED" ? "error" : "info",
      event: "task_finished",
      message: `任务结束：新增 ${counts.insertedCount}、更新 ${counts.updatedCount}、跳过 ${counts.skippedCount}、失败 ${counts.failedCount}`,
      retryCount: 0, details: counts,
    });
  } catch (error) {
    const message = error instanceof Error ? error.message : "同步任务失败";
    await patchTask(taskId, {
      ...counts, status: "FAILED", finishedAt: new Date().toISOString(),
      progress: 1, lastError: message,
    });
    if (task.taskType === "backfill") {
      await patchLibraryState({ status: "failed", count: counts.insertedCount, error: message });
    }
    await addLog({
      taskId, sourceKey: task.sourceKey, level: "error", event: "task_failed",
      message, retryCount: 0,
    });
  }
  return getTask(taskId);
}

export async function scheduleEnglishSync(
  taskType: EnglishSyncTask["taskType"],
  sourceKey?: string,
  force = false,
  requestedLimit?: number,
) {
  if (taskType === "backfill" && !force && !sourceKey) {
    const stats = await getLibraryStats();
    if (stats.initialization.status === "completed") {
      return { task: undefined, created: false, reason: "文章库已经完成初始化" };
    }
  }
  const safeLimit = requestedLimit == null ? undefined : Math.max(1, Math.min(Math.round(requestedLimit), 500));
  return createTask(taskType, sourceKey, safeLimit);
}

export async function shouldRunStartupSync() {
  const sources = await listSourceStates(false);
  const now = Date.now();
  return sources.some((source) =>
    !source.lastSuccessAt || now - new Date(source.lastSuccessAt).getTime() >= source.syncInterval * 1000,
  );
}
