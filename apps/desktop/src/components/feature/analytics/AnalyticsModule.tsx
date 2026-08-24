import { FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import {
  BarChart3,
  ChevronRight,
  Clock3,
  Database,
  RefreshCw,
  Search,
  Sparkles,
} from "lucide-react";

import { analyticsApi } from "@/src/services/analyticsApi";
import type {
  InsightSnapshot,
  ProjectionStatus,
  ReportSnapshot,
  SearchHit,
  TimelineEvent,
} from "@/src/types/analytics";

type Tab = "timeline" | "search" | "report" | "insights";

type Props = {
  openEntity: (entityType: string, entityId: string) => void;
};

const DOMAIN_LABELS: Record<string, string> = {
  finance: "财务",
  habits: "坚持",
  notes: "笔记",
  english: "英语",
  fitness: "健身",
  execution: "执行",
};

function localDate(value = new Date()) {
  const offset = value.getTimezoneOffset() * 60_000;
  return new Date(value.getTime() - offset).toISOString().slice(0, 10);
}

function addDays(value: string, days: number) {
  const date = new Date(`${value}T12:00:00`);
  date.setDate(date.getDate() + days);
  return localDate(date);
}

function formatDateTime(value?: string | null) {
  if (!value) return "";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

function formatMoney(cents: number) {
  return new Intl.NumberFormat("zh-CN", {
    style: "currency",
    currency: "CNY",
    maximumFractionDigits: 2,
  }).format(cents / 100);
}

function formatDuration(seconds: number) {
  if (seconds < 60) return `${seconds} 秒`;
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.round((seconds % 3600) / 60);
  return hours ? `${hours} 小时 ${minutes} 分` : `${minutes} 分钟`;
}

function domainLabel(domain: string) {
  return DOMAIN_LABELS[domain] || domain;
}

export default function AnalyticsModule({ openEntity }: Props) {
  const today = useMemo(() => localDate(), []);
  const [tab, setTab] = useState<Tab>("timeline");
  const [from, setFrom] = useState(() => addDays(today, -29));
  const [to, setTo] = useState(today);
  const [domain, setDomain] = useState("");
  const [status, setStatus] = useState<ProjectionStatus | null>(null);
  const [timeline, setTimeline] = useState<TimelineEvent[]>([]);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [searchText, setSearchText] = useState("");
  const [searchHits, setSearchHits] = useState<SearchHit[]>([]);
  const [report, setReport] = useState<ReportSnapshot | null>(null);
  const [insights, setInsights] = useState<InsightSnapshot[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC";

  const refreshStatus = useCallback(async () => {
    const value = await analyticsApi.status();
    setStatus(value);
  }, []);

  const loadTimeline = useCallback(async (cursor?: string, append = false) => {
    setLoading(true);
    setError("");
    try {
      const page = await analyticsApi.timeline({
        from,
        to,
        domain: domain || undefined,
        cursor,
        limit: 60,
      });
      setTimeline((current) => append ? [...current, ...page.items] : page.items);
      setNextCursor(page.nextCursor || null);
      await refreshStatus();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "时间线加载失败");
    } finally {
      setLoading(false);
    }
  }, [domain, from, refreshStatus, to]);

  const loadReport = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const value = await analyticsApi.report({
        reportType: "custom",
        periodStart: from,
        periodEnd: to,
        timezone,
      });
      setReport(value);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "报告生成失败");
    } finally {
      setLoading(false);
    }
  }, [from, timezone, to]);

  const loadInsights = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      setInsights(await analyticsApi.insights({ periodStart: from, periodEnd: to }));
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "洞察生成失败");
    } finally {
      setLoading(false);
    }
  }, [from, to]);

  useEffect(() => {
    void refreshStatus().catch(() => undefined);
    void loadTimeline();
  }, [loadTimeline, refreshStatus]);

  useEffect(() => {
    if (tab === "report") void loadReport();
    if (tab === "insights") void loadInsights();
  }, [loadInsights, loadReport, tab]);

  const rebuild = async () => {
    setLoading(true);
    setError("");
    try {
      const value = await analyticsApi.rebuild();
      setStatus(value);
      await loadTimeline();
      if (tab === "report") await loadReport();
      if (tab === "insights") await loadInsights();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "分析索引重建失败");
    } finally {
      setLoading(false);
    }
  };

  const runSearch = async (event?: FormEvent) => {
    event?.preventDefault();
    const q = searchText.trim();
    if (!q) {
      setSearchHits([]);
      return;
    }
    setLoading(true);
    setError("");
    try {
      setSearchHits(await analyticsApi.search({ q, domain: domain || undefined, from, to, limit: 80 }));
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "搜索失败");
    } finally {
      setLoading(false);
    }
  };

  const renderEntityButton = (entityType: string, entityId: string) => (
    <button
      type="button"
      className="analytics-open-entity"
      onClick={() => openEntity(entityType, entityId)}
      aria-label="打开原始记录"
    >
      <ChevronRight />
    </button>
  );

  return (
    <section className="analytics-page">
      <header className="analytics-header">
        <div>
          <p className="analytics-eyebrow">EPIC-14 · 本地派生分析</p>
          <h1>分析与洞察</h1>
          <p>把分散在 LifeTrace 各模块里的记录放到同一条时间轴、搜索入口和周期报告中。</p>
        </div>
        <div className="analytics-index-state">
          <span className={status?.dirty ? "dirty" : "ready"}>
            <Database />
            {status ? (status.dirty ? "索引待刷新" : "索引正常") : "读取索引状态"}
          </span>
          <button type="button" className="hx-btn" disabled={loading} onClick={() => void rebuild()}>
            <RefreshCw className={loading ? "spin" : ""} />
            重建索引
          </button>
        </div>
      </header>

      <div className="analytics-controls">
        <label>
          <span>开始日期</span>
          <input type="date" value={from} max={to} onChange={(event) => setFrom(event.target.value)} />
        </label>
        <label>
          <span>结束日期</span>
          <input type="date" value={to} min={from} onChange={(event) => setTo(event.target.value)} />
        </label>
        <label>
          <span>领域</span>
          <select value={domain} onChange={(event) => setDomain(event.target.value)}>
            <option value="">全部领域</option>
            {Object.entries(DOMAIN_LABELS).map(([value, label]) => (
              <option key={value} value={value}>{label}</option>
            ))}
          </select>
        </label>
      </div>

      <nav className="analytics-tabs" aria-label="分析视图">
        <button type="button" className={tab === "timeline" ? "active" : ""} onClick={() => setTab("timeline")}>
          <Clock3 />时间线
        </button>
        <button type="button" className={tab === "search" ? "active" : ""} onClick={() => setTab("search")}>
          <Search />全局搜索
        </button>
        <button type="button" className={tab === "report" ? "active" : ""} onClick={() => setTab("report")}>
          <BarChart3 />周期报告
        </button>
        <button type="button" className={tab === "insights" ? "active" : ""} onClick={() => setTab("insights")}>
          <Sparkles />关联洞察
        </button>
      </nav>

      {error ? <div className="analytics-error" role="alert">{error}</div> : null}

      {tab === "timeline" ? (
        <div className="analytics-panel">
          <div className="analytics-panel-heading">
            <div><h2>统一时间线</h2><p>{timeline.length} 条已加载记录</p></div>
            <button type="button" className="hx-btn" disabled={loading} onClick={() => void loadTimeline()}>
              <RefreshCw />刷新
            </button>
          </div>
          <div className="analytics-timeline">
            {timeline.map((item) => (
              <article className="analytics-event" key={item.id}>
                <div className="analytics-event-time">
                  <strong>{formatDateTime(item.occurredAt)}</strong>
                  <span>{domainLabel(item.domain)}</span>
                </div>
                <div className="analytics-event-body">
                  <h3>{item.title}</h3>
                  {item.summary ? <p>{item.summary}</p> : null}
                  <small>{item.eventType}</small>
                </div>
                {renderEntityButton(item.entityType, item.entityId)}
              </article>
            ))}
            {!timeline.length && !loading ? <div className="analytics-empty">这个时间范围内还没有可展示的记录。</div> : null}
          </div>
          {nextCursor ? (
            <button type="button" className="hx-btn analytics-load-more" disabled={loading} onClick={() => void loadTimeline(nextCursor, true)}>
              加载更早记录
            </button>
          ) : null}
        </div>
      ) : null}

      {tab === "search" ? (
        <div className="analytics-panel">
          <form className="analytics-search" onSubmit={(event) => void runSearch(event)}>
            <Search />
            <input
              value={searchText}
              onChange={(event) => setSearchText(event.target.value)}
              placeholder="搜索笔记、交易、坚持、英语、训练、任务……"
              autoFocus
            />
            <button className="hx-btn primary" type="submit" disabled={loading}>搜索</button>
          </form>
          <div className="analytics-search-results">
            {searchHits.map((hit) => (
              <article key={hit.id} className="analytics-search-hit">
                <div>
                  <span className="analytics-domain-chip">{domainLabel(hit.domain)}</span>
                  <h3>{hit.title}</h3>
                  {hit.snippet ? <p>{hit.snippet}</p> : null}
                  <small>{hit.occurredAt ? formatDateTime(hit.occurredAt) : formatDateTime(hit.updatedAt)}</small>
                </div>
                {renderEntityButton(hit.entityType, hit.entityId)}
              </article>
            ))}
            {!searchHits.length && searchText.trim() && !loading ? <div className="analytics-empty">没有找到匹配记录。</div> : null}
          </div>
        </div>
      ) : null}

      {tab === "report" ? (
        <div className="analytics-panel">
          <div className="analytics-panel-heading">
            <div><h2>周期报告</h2><p>{from} — {to}</p></div>
            <button type="button" className="hx-btn" disabled={loading} onClick={() => void loadReport()}>
              <RefreshCw />重新计算
            </button>
          </div>
          {report ? (
            <>
              <div className="analytics-metrics-grid">
                <div><span>支出</span><strong>{formatMoney(report.facts.finance.expenseCents)}</strong><small>{report.facts.finance.transactionCount} 笔交易</small></div>
                <div><span>坚持完成</span><strong>{Math.round(report.facts.habits.completionRate * 100)}%</strong><small>{report.facts.habits.completedCount}/{report.facts.habits.logCount} 条</small></div>
                <div><span>训练</span><strong>{report.facts.fitness.workoutCount} 次</strong><small>{formatDuration(report.facts.fitness.durationSeconds)}</small></div>
                <div><span>英语</span><strong>{report.facts.english.sessionCount} 次</strong><small>{formatDuration(report.facts.english.readingTimeSeconds)}</small></div>
                <div><span>新笔记</span><strong>{report.facts.notes.createdCount} 篇</strong><small>本周期创建</small></div>
                <div><span>完成任务</span><strong>{report.facts.execution.completedTaskCount} 个</strong><small>{report.facts.execution.taskCount} 个相关任务</small></div>
              </div>
              <div className="analytics-coverage">
                <h3>数据覆盖</h3>
                <div>
                  {Object.entries(report.coverage).map(([key, covered]) => (
                    <span key={key} className={covered ? "covered" : "missing"}>{domainLabel(key)} · {covered ? "有数据" : "无数据"}</span>
                  ))}
                </div>
              </div>
              <p className="analytics-footnote">所有数字由本地 SQLite/Rust 确定性计算，未使用大语言模型进行计数或金额计算。</p>
            </>
          ) : <div className="analytics-empty">正在生成报告……</div>}
        </div>
      ) : null}

      {tab === "insights" ? (
        <div className="analytics-panel">
          <div className="analytics-panel-heading">
            <div><h2>关联洞察</h2><p>只展示满足最低样本量的可解释关系。</p></div>
            <button type="button" className="hx-btn" disabled={loading} onClick={() => void loadInsights()}>
              <RefreshCw />重新计算
            </button>
          </div>
          <div className="analytics-insights">
            {insights.map((item) => (
              <article key={item.id}>
                <div className="analytics-insight-icon"><Sparkles /></div>
                <div>
                  <h3>{item.title}</h3>
                  <p>{item.summary}</p>
                  <small>样本量 {item.sampleSize} · {String(item.confidence.level || "descriptive")} · 非因果结论</small>
                </div>
              </article>
            ))}
            {!insights.length && !loading ? (
              <div className="analytics-empty">当前周期还没有达到样本门槛的关联洞察。继续积累记录后会自动出现。</div>
            ) : null}
          </div>
        </div>
      ) : null}
    </section>
  );
}
