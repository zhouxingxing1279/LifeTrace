"use client";

import { useCallback, useEffect, useState } from "react";
import {
  AlertCircle, CheckCircle2, CirclePause, Database, FileWarning,
  Headphones, History, RefreshCw, RotateCcw, Server, Wrench,
} from "lucide-react";
import type {
  EnglishContentSourceState, EnglishLibraryStats, EnglishSyncLog, EnglishSyncTask,
} from "@/src/types/english";

const request = async <T,>(url: string, init?: RequestInit): Promise<T> => {
  const response = await fetch(url, init);
  const payload = await response.json() as T & { error?: string };
  if (!response.ok) throw new Error(payload.error || "文章库管理操作失败");
  return payload;
};
const post = <T,>(url: string, body: unknown) => request<T>(url, {
  method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body),
});
const dateTime = (value?: string) => value ? new Date(value).toLocaleString("zh-CN", { hour12: false }) : "尚无记录";
const statusLabel: Record<EnglishContentSourceState["status"], string> = {
  active: "正常", stale: "长期无新增", error: "请求异常", disabled: "已停用", rate_limited: "受到限速",
};

export default function EnglishSourceManager({ onArticlesChanged }: { onArticlesChanged: () => void }) {
  const [sources, setSources] = useState<EnglishContentSourceState[]>([]);
  const [stats, setStats] = useState<EnglishLibraryStats>();
  const [activeTask, setActiveTask] = useState<EnglishSyncTask>();
  const [lastTask, setLastTask] = useState<EnglishSyncTask>();
  const [logs, setLogs] = useState<EnglishSyncLog[]>([]);
  const [showLogs, setShowLogs] = useState(false);
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    const [sourceData, statData, taskData] = await Promise.all([
      request<{ sources: EnglishContentSourceState[] }>("/api/english/sources"),
      request<EnglishLibraryStats>("/api/english/articles/stats"),
      request<{ activeTask?: EnglishSyncTask; tasks: EnglishSyncTask[] }>("/api/english/sync/status"),
    ]);
    setSources(sourceData.sources);
    setStats(statData);
    setActiveTask(taskData.activeTask);
    setLastTask(taskData.tasks[0]);
    if (showLogs) setLogs((await request<{ logs: EnglishSyncLog[] }>("/api/english/sync/logs?limit=80")).logs);
    return taskData.activeTask;
  }, [showLogs]);

  useEffect(() => {
    const timer = window.setTimeout(() => {
      void load().catch((error) => setMessage(error instanceof Error ? error.message : "文章库状态加载失败"));
    }, 0);
    return () => window.clearTimeout(timer);
  }, [load]);

  useEffect(() => {
    if (!activeTask) return;
    const timer = window.setInterval(() => {
      void load().then((currentTask) => {
        if (!currentTask) onArticlesChanged();
      }).catch(() => undefined);
    }, 1500);
    return () => window.clearInterval(timer);
  }, [activeTask, load, onArticlesChanged]);

  const start = async (url: string, body: unknown, success: string) => {
    setBusy(true);
    setMessage("");
    try {
      const result = await post<{ taskId?: string; created: boolean; reason?: string }>(url, body);
      setMessage(result.created ? success : result.reason || "已有同步任务正在执行");
      await load();
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "操作失败");
    } finally {
      setBusy(false);
    }
  };

  const progress = activeTask ? Math.round(activeTask.progress * 100) : 0;
  return <section className="en-source-manager" aria-labelledby="source-manager-title">
    <header>
      <div><span className="en-eyebrow">LIBRARY OPERATIONS</span><h3 id="source-manager-title">文章库与数据源</h3><p>基础库存由一次性脚本建立；这里负责增量更新、失败重试和来源健康状态。</p></div>
      <div className="en-source-actions">
        <button type="button" disabled={busy || Boolean(activeTask)} onClick={() => void start("/api/english/sync", { force: true }, "增量同步已开始")}><RefreshCw />立即同步</button>
        <button type="button" disabled={busy || Boolean(activeTask)} onClick={() => void start("/api/english/sync/backfill", { force: true }, "历史回填已开始")}><Database />历史回填</button>
        <button type="button" disabled={busy || Boolean(activeTask)} onClick={() => void start("/api/english/sync/retry-failed", {}, "失败文章重试已开始")}><RotateCcw />重试失败</button>
        <button type="button" disabled={busy || Boolean(activeTask)} onClick={() => void start("/api/english/sync/repair", { deep: false }, "周度补漏已开始")}><Wrench />补漏扫描</button>
        <button type="button" aria-expanded={showLogs} onClick={() => setShowLogs((value) => !value)}><History />{showLogs ? "收起日志" : "查看日志"}</button>
      </div>
    </header>

    {message && <p className="en-source-message" role="status">{message}</p>}
    <div className="en-library-metrics">
      <article><Database /><span>文章总数</span><strong>{stats?.total ?? "—"}</strong></article>
      <article><CheckCircle2 /><span>可推荐 READY</span><strong>{stats?.ready ?? "—"}</strong></article>
      <article><CirclePause /><span>待处理</span><strong>{stats?.pending ?? "—"}</strong></article>
      <article><FileWarning /><span>失败 / 拒绝</span><strong>{stats ? stats.failed + stats.rejected : "—"}</strong></article>
      <article><Headphones /><span>带音频</span><strong>{stats?.withAudio ?? "—"}</strong></article>
    </div>

    {activeTask && <div className="en-sync-progress" aria-live="polite">
      <div><strong>{activeTask.taskType === "backfill" ? "正在建立文章库" : "同步任务正在运行"}</strong><span>{progress}%</span></div>
      <progress max="100" value={progress}>{progress}%</progress>
      <p>已处理 {activeTask.successCount + activeTask.failedCount} / {activeTask.totalCount || stats?.initialization.targetArticleCount || 0}
        {" · "}新增 {activeTask.insertedCount} · 更新 {activeTask.updatedCount} · 跳过 {activeTask.skippedCount} · 失败 {activeTask.failedCount}</p>
      {activeTask.currentArticle && <small title={activeTask.currentArticle}>正在处理：{activeTask.currentArticle}</small>}
    </div>}

    <div className="en-source-table-wrap"><table className="en-source-table">
      <thead><tr><th>来源</th><th>状态</th><th>文章</th><th>上次同步</th><th>上次新增</th><th>失败</th><th>操作</th></tr></thead>
      <tbody>{sources.map((source) => <tr key={source.sourceKey}>
        <td><strong>{source.sourceName}</strong><small>{source.category}</small></td>
        <td><span className={`en-source-status ${source.status}`}><Server />{statusLabel[source.status]}</span></td>
        <td>{source.articleCount}</td><td>{dateTime(source.lastSyncAt)}</td><td>{dateTime(source.lastNewArticleAt)}</td><td>{source.consecutiveFailures}</td>
        <td><div>
          <button type="button" disabled={busy || Boolean(activeTask) || !source.enabled} onClick={() => void start(`/api/english/sources/${encodeURIComponent(source.sourceKey)}/sync`, {}, `${source.sourceName} 同步已开始`)}>同步</button>
          <button type="button" onClick={async () => {
            setBusy(true);
            try {
              await request(`/api/english/sources/${encodeURIComponent(source.sourceKey)}`, {
                method: "PATCH", headers: { "content-type": "application/json" },
                body: JSON.stringify({ enabled: !source.enabled }),
              });
              await load();
            } finally { setBusy(false); }
          }}>{source.enabled ? "停用" : "启用"}</button>
        </div></td>
      </tr>)}</tbody>
    </table></div>

    <div className="en-library-breakdown">
      <p><strong>CEFR：</strong>{Object.entries(stats?.byCefr ?? {}).map(([key, value]) => <span key={key}>{key} {value}</span>)}</p>
      <p><strong>栏目：</strong>{Object.entries(stats?.byCategory ?? {}).map(([key, value]) => <span key={key}>{key} {value}</span>)}</p>
      <p><strong>最近同步：</strong>{dateTime(stats?.lastSyncAt)} <strong>最近新增：</strong>{dateTime(stats?.lastNewArticleAt)}</p>
    </div>

    {showLogs && <div className="en-sync-logs"><h4>最近同步日志</h4>
      {logs.length ? logs.map((log) => <article key={log.id} className={log.level}>
        <time>{dateTime(log.createdAt)}</time><strong>{log.sourceKey || "全部来源"}</strong><p>{log.message}</p>{log.durationMs != null && <small>{log.durationMs} ms</small>}
      </article>) : <p>暂无同步日志。</p>}
    </div>}
    {!activeTask && lastTask && <p className="en-last-task">最近任务：{lastTask.status} · 新增 {lastTask.insertedCount} · 更新 {lastTask.updatedCount} · 跳过 {lastTask.skippedCount} · 失败 {lastTask.failedCount}
      {lastTask.lastError && <span><AlertCircle />{lastTask.lastError}</span>}</p>}
  </section>;
}
