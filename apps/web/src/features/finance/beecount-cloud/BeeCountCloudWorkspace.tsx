/*
 * BeeCount Cloud Web finance port.
 *
 * Information architecture, dashboard composition, navigation grouping, visual
 * tokens and interaction conventions are source-derived from:
 *   TNT-Likely/BeeCount-Cloud @ 3e02e499431bdceae2051c1dfb980898d26ef5e1
 *   - frontend/apps/web/src/pages/sections/*
 *   - frontend/apps/web/src/components/sections/OverviewSection.tsx
 *   - frontend/apps/web/src/components/dashboard/*
 *   - frontend/packages/web-features/src/nav.ts
 *   - frontend/packages/web-features/src/features/*
 *
 * LifeTrace intentionally owns only the outer AppShell/session and the API
 * adapter. Finance presentation lives here instead of being rebuilt with
 * LifeTrace's generic Card/Metric components.
 */
import { useEffect, useMemo, useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import {
  ArrowLeft,
  ArrowRight,
  CalendarDays,
  ChevronLeft,
  ChevronRight,
  Cloud,
  Download,
  Filter,
  Landmark,
  LayoutDashboard,
  RefreshCw,
  Search,
  Tags,
  Upload,
  WalletCards,
} from "lucide-react";
import {
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Pie,
  PieChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

import { useApp } from "../../../app/AppContext";
import {
  formatMoney,
  type BeeCountAccount,
  type BeeCountCategory,
  type BeeCountLedger,
  type BeeCountLedgerSnapshot,
  type BeeCountTag,
  type BeeCountTransaction,
} from "../../../services/core";
import { LifeTraceBeeCountAdapter, beeCountMonthSeries } from "../beecount/adapter";
import "./beecount-cloud.css";

const UPSTREAM_SHA = "3e02e499431bdceae2051c1dfb980898d26ef5e1";
const ACTIVE_LEDGER_KEY = "beecount:web:activeLedgerId";
const TX_PAGE_SIZE_DEFAULT = 20;

const PRIMARY_NAV = [
  ["overview", "概览", LayoutDashboard],
  ["transactions", "交易", WalletCards],
  ["accounts", "账户", Landmark],
  ["categories", "分类", Filter],
  ["tags", "标签", Tags],
] as const;

const TOOL_NAV = [
  ["calendar", "日历", CalendarDays],
  ["budgets", "预算", LayoutDashboard],
  ["ledgers", "账本", WalletCards],
  ["import", "导入", Upload],
] as const;

type FinanceView = (typeof PRIMARY_NAV)[number][0] | (typeof TOOL_NAV)[number][0];

type TxFilter = {
  q: string;
  type: string;
  dateFrom: string;
  dateTo: string;
  category: string;
  tag: string;
  amountMin: string;
  amountMax: string;
};

const EMPTY_FILTER: TxFilter = {
  q: "",
  type: "all",
  dateFrom: "",
  dateTo: "",
  category: "",
  tag: "",
  amountMin: "",
  amountMax: "",
};

function financeView(pathname: string): FinanceView {
  const last = pathname.split("/").filter(Boolean).at(-1) ?? "finance";
  const all = [...PRIMARY_NAV, ...TOOL_NAV];
  if (last === "finance") return "overview";
  return all.some(([id]) => id === last) ? last as FinanceView : "overview";
}

function viewPath(view: FinanceView): string {
  return view === "overview" ? "/app/finance" : `/app/finance/${view}`;
}

function readStoredLedger(): string {
  try { return localStorage.getItem(ACTIVE_LEDGER_KEY) ?? ""; }
  catch { return ""; }
}

function storeLedger(id: string) {
  try { localStorage.setItem(ACTIVE_LEDGER_KEY, id); }
  catch { /* private mode */ }
}

export function BeeCountCloudWorkspace() {
  const { privacy, online } = useApp();
  const location = useLocation();
  const navigate = useNavigate();
  const view = financeView(location.pathname);
  const adapter = useMemo(() => new LifeTraceBeeCountAdapter(), []);
  const [ledgers, setLedgers] = useState<BeeCountLedger[]>([]);
  const [activeLedgerId, setActiveLedgerId] = useState(() => readStoredLedger());
  const [snapshot, setSnapshot] = useState<BeeCountLedgerSnapshot | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  async function load(preferred = activeLedgerId) {
    if (!online) {
      setError("当前离线，无法读取 BeeCount 财务数据");
      return;
    }
    setLoading(true);
    setError("");
    try {
      const status = await adapter.status();
      if (!status.enabled || !status.upstreamReachable) throw new Error("BeeCount 财务服务暂时不可用");
      const response = await adapter.ledgers();
      const rows = response.items;
      setLedgers(rows);
      const resolved = rows.some((item) => item.sourceId === preferred)
        ? preferred
        : rows[0]?.sourceId ?? "";
      setActiveLedgerId(resolved);
      if (resolved) storeLedger(resolved);
      setSnapshot(resolved ? await adapter.snapshotAll(resolved) : null);
    } catch (cause) {
      setSnapshot(null);
      setError(cause instanceof Error ? cause.message : "BeeCount 财务加载失败");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void load();
    // Online transition is the only automatic reload trigger. Manual refresh and
    // ledger switch are explicit, matching BeeCount Cloud's page behavior.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [online]);

  async function chooseLedger(id: string) {
    if (!id || id === activeLedgerId) return;
    setLoading(true);
    setError("");
    try {
      setActiveLedgerId(id);
      storeLedger(id);
      setSnapshot(await adapter.snapshotAll(id));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "BeeCount 账本加载失败");
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="beecount-cloud -mx-1 rounded-2xl bg-[hsl(var(--background))] p-3 sm:p-4 lg:p-5">
      <header className="bc-panel mb-4 p-3 sm:p-4">
        <div className="flex flex-col gap-3 xl:flex-row xl:items-center xl:justify-between">
          <div className="flex min-w-0 items-center gap-3">
            <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-primary text-primary-foreground shadow-sm">
              <span className="text-lg font-black">B</span>
            </div>
            <div className="min-w-0">
              <div className="flex items-center gap-2">
                <h1 className="truncate text-lg font-semibold tracking-tight">BeeCount</h1>
                <span className="rounded-full bg-primary/15 px-2 py-0.5 text-[10px] font-semibold text-primary">Cloud Web</span>
              </div>
              <p className="truncate text-xs text-muted-foreground">LifeTrace 仅提供登录、外层导航和 PostgreSQL 兼容接口</p>
            </div>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <label className="flex min-w-0 items-center gap-2 rounded-xl border border-border/70 bg-card px-3 py-2 text-xs text-muted-foreground">
              <Cloud size={14} className="shrink-0 text-primary" />
              <select
                className="max-w-56 bg-transparent text-sm font-medium text-foreground outline-none"
                value={activeLedgerId}
                onChange={(event) => void chooseLedger(event.target.value)}
                aria-label="当前 BeeCount 账本"
              >
                {ledgers.map((ledger) => (
                  <option key={ledger.id} value={ledger.sourceId}>{ledger.name} · {ledger.currency}</option>
                ))}
              </select>
            </label>
            <button
              type="button"
              className="inline-flex h-10 items-center gap-2 rounded-xl border border-border/70 bg-card px-3 text-sm font-medium hover:bg-muted/60 disabled:opacity-50"
              disabled={!online || loading}
              onClick={() => void load(activeLedgerId)}
            >
              <RefreshCw size={15} className={loading ? "animate-spin" : ""} /> 刷新
            </button>
          </div>
        </div>

        <div className="mt-4 flex flex-col gap-2 border-t border-border/60 pt-3 xl:flex-row xl:items-center xl:justify-between">
          <NavGroup items={PRIMARY_NAV} active={view} onSelect={(id) => navigate(viewPath(id))} />
          <NavGroup items={TOOL_NAV} active={view} onSelect={(id) => navigate(viewPath(id))} compact />
        </div>
      </header>

      {error ? (
        <div className="mb-4 rounded-xl border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {error}。财务页面不会回退到旧 LifeTrace 财务数据。
        </div>
      ) : null}

      {!snapshot && !loading ? (
        <Empty title="暂无 BeeCount 财务数据" description="请确认手机端已经同步 BeeCount 账本。" />
      ) : null}

      {snapshot ? (
        <main className={loading ? "pointer-events-none opacity-70 transition-opacity" : "transition-opacity"}>
          {view === "overview" ? <Overview snapshot={snapshot} privacy={privacy} onNavigate={(v) => navigate(viewPath(v))} /> : null}
          {view === "transactions" ? <Transactions snapshot={snapshot} privacy={privacy} /> : null}
          {view === "calendar" ? <Calendar snapshot={snapshot} privacy={privacy} /> : null}
          {view === "budgets" ? <Budgets snapshot={snapshot} privacy={privacy} /> : null}
          {view === "accounts" ? <Accounts snapshot={snapshot} privacy={privacy} /> : null}
          {view === "categories" ? <Categories snapshot={snapshot} /> : null}
          {view === "tags" ? <TagsView snapshot={snapshot} privacy={privacy} /> : null}
          {view === "import" ? <ImportView /> : null}
        </main>
      ) : null}

      {view === "ledgers" ? <Ledgers rows={ledgers} privacy={privacy} activeId={activeLedgerId} onSelect={(id) => void chooseLedger(id)} /> : null}

      <div className="mt-5 flex flex-wrap items-center justify-between gap-2 border-t border-border/50 pt-3 text-[10px] text-muted-foreground">
        <span>BeeCount Cloud Web source port · upstream {UPSTREAM_SHA.slice(0, 10)}</span>
        <span>BeeCount authorship and bundled license retained in beecount/</span>
      </div>
    </div>
  );
}

function NavGroup({
  items,
  active,
  onSelect,
  compact = false,
}: {
  items: readonly (readonly [FinanceView, string, typeof LayoutDashboard])[];
  active: FinanceView;
  onSelect: (id: FinanceView) => void;
  compact?: boolean;
}) {
  return (
    <nav className="flex min-w-0 gap-1 overflow-x-auto" aria-label={compact ? "BeeCount 工具" : "BeeCount 记账导航"}>
      {items.map(([id, label, Icon]) => (
        <button
          type="button"
          key={id}
          onClick={() => onSelect(id)}
          className={`inline-flex shrink-0 items-center gap-1.5 rounded-lg px-3 py-2 text-sm transition-colors ${
            active === id ? "bg-primary/15 font-semibold text-primary" : "text-muted-foreground hover:bg-muted/60 hover:text-foreground"
          }`}
        >
          <Icon size={14} /> {label}
        </button>
      ))}
    </nav>
  );
}

function Overview({
  snapshot,
  privacy,
  onNavigate,
}: {
  snapshot: BeeCountLedgerSnapshot;
  privacy: boolean;
  onNavigate: (view: FinanceView) => void;
}) {
  const [scope, setScope] = useState<"month" | "year" | "all">("month");
  const now = new Date();
  const currentMonth = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}`;
  const currentYear = String(now.getFullYear());
  const scoped = useMemo(() => snapshot.transactions.items.filter((tx) => {
    const date = txDate(tx);
    if (scope === "month") return date.startsWith(currentMonth);
    if (scope === "year") return date.startsWith(currentYear);
    return true;
  }), [snapshot.transactions.items, scope, currentMonth, currentYear]);
  const summary = summarize(scoped);
  const monthSeries = beeCountMonthSeries(snapshot);
  const categoryRows = rankByCategory(scoped);
  const tagRows = rankTags(scoped);
  const distinctDays = new Set(scoped.map(txDate)).size;
  const firstDate = snapshot.transactions.items.length
    ? snapshot.transactions.items.map(txDate).filter(Boolean).sort()[0]
    : "";
  const daysSinceFirst = firstDate ? Math.max(1, Math.ceil((Date.now() - Date.parse(`${firstDate}T00:00:00`)) / 86_400_000)) : 0;

  return (
    <div className="space-y-4">
      <section className="bc-panel bc-hero-border bee-rise-in p-4 sm:p-5">
        <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
          <div>
            <div className="text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground">{snapshot.ledger.name}</div>
            <div className="mt-2 text-3xl font-bold tracking-tight sm:text-4xl">{formatMoney(snapshot.ledger.balanceCents, snapshot.ledger.currency, privacy)}</div>
            <div className="mt-2 text-xs text-muted-foreground">{snapshot.transactions.total} 笔交易 · {snapshot.ledger.currency}</div>
          </div>
          <div className="inline-flex self-start rounded-xl bg-muted/55 p-1">
            {(["month", "year", "all"] as const).map((id) => (
              <button key={id} type="button" onClick={() => setScope(id)} className={`rounded-lg px-3 py-1.5 text-xs font-medium ${scope === id ? "bg-card text-foreground shadow-sm" : "text-muted-foreground"}`}>
                {id === "month" ? "本月" : id === "year" ? "本年" : "全部"}
              </button>
            ))}
          </div>
        </div>
        <div className="mt-5 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <HeroMetric label="收入" value={formatMoney(summary.income, snapshot.ledger.currency, privacy)} tone="income" />
          <HeroMetric label="支出" value={formatMoney(summary.expense, snapshot.ledger.currency, privacy)} tone="expense" />
          <HeroMetric label="结余" value={formatMoney(summary.income - summary.expense, snapshot.ledger.currency, privacy)} />
          <HeroMetric label="记账" value={`${summary.count} 笔`} />
        </div>
      </section>

      <section className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <MiniStat label="记账天数" value={`${distinctDays} 天`} hint="当前视角" />
        <MiniStat label="使用跨度" value={`${daysSinceFirst} 天`} hint={firstDate || "暂无交易"} />
        <MiniStat label="日均笔数" value={distinctDays ? (summary.count / distinctDays).toFixed(1) : "0"} hint="有记录日期" />
        <MiniStat label="活跃账户" value={`${snapshot.accounts.filter((item) => (item.transactionCount ?? 0) > 0).length}`} hint={`${snapshot.accounts.length} 个账户`} />
      </section>

      <SectionDivider>扩展分析</SectionDivider>

      <div className="grid gap-4 lg:grid-cols-2">
        <CategoryDonut rows={categoryRows.slice(0, 8)} currency={snapshot.ledger.currency} privacy={privacy} />
        <YearHeatmap items={snapshot.transactions.items} />
      </div>

      <div className="grid gap-4 lg:grid-cols-[1.05fr_1fr]">
        <AssetDonut accounts={snapshot.accounts} currency={snapshot.ledger.currency} privacy={privacy} />
        <MonthlyTrend rows={monthSeries} currency={snapshot.ledger.currency} privacy={privacy} />
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <RankPanel title="支出分类 Top 5" rows={categoryRows.filter((r) => r.expense > 0).sort((a, b) => b.expense - a.expense).slice(0, 5).map((r) => ({ name: r.name, value: r.expense }))} currency={snapshot.ledger.currency} privacy={privacy} onOpen={() => onNavigate("categories")} />
        <RankPanel title="收入分类 Top 5" rows={categoryRows.filter((r) => r.income > 0).sort((a, b) => b.income - a.income).slice(0, 5).map((r) => ({ name: r.name, value: r.income }))} currency={snapshot.ledger.currency} privacy={privacy} onOpen={() => onNavigate("categories")} />
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <RankPanel title="Top 标签" rows={tagRows.slice(0, 5).map((r) => ({ name: r.name, value: r.value }))} currency={snapshot.ledger.currency} privacy={privacy} onOpen={() => onNavigate("tags")} />
        <RankPanel title="Top 账户" rows={[...snapshot.accounts].sort((a, b) => (b.transactionCount ?? 0) - (a.transactionCount ?? 0)).slice(0, 5).map((a) => ({ name: a.name, value: a.balanceCents ?? 0 }))} currency={snapshot.ledger.currency} privacy={privacy} onOpen={() => onNavigate("accounts")} />
      </div>
    </div>
  );
}

function HeroMetric({ label, value, tone }: { label: string; value: string; tone?: "income" | "expense" }) {
  return <div className="rounded-xl border border-border/60 bg-card/65 p-3"><div className="text-xs text-muted-foreground">{label}</div><div className={`mt-1 text-lg font-semibold ${tone === "income" ? "text-income" : tone === "expense" ? "text-expense" : ""}`}>{value}</div></div>;
}

function MiniStat({ label, value, hint }: { label: string; value: string; hint: string }) {
  return <div className="bc-panel bee-rise-in p-4"><div className="text-xs text-muted-foreground">{label}</div><div className="mt-1 text-xl font-semibold">{value}</div><div className="mt-1 text-[11px] text-muted-foreground">{hint}</div></div>;
}

function SectionDivider({ children }: { children: string }) {
  return <div className="flex items-center gap-2 pt-2"><span className="h-px flex-1 bg-border/60"/><span className="text-[11px] font-semibold uppercase tracking-[0.22em] text-muted-foreground">{children}</span><span className="h-px flex-1 bg-border/60"/></div>;
}

function Transactions({ snapshot, privacy }: { snapshot: BeeCountLedgerSnapshot; privacy: boolean }) {
  const [filter, setFilter] = useState<TxFilter>(EMPTY_FILTER);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(TX_PAGE_SIZE_DEFAULT);
  const rows = useMemo(() => applyTxFilter(snapshot.transactions.items, filter), [snapshot.transactions.items, filter]);
  const pages = Math.max(1, Math.ceil(rows.length / pageSize));
  const safePage = Math.min(page, pages);
  const shown = rows.slice((safePage - 1) * pageSize, safePage * pageSize);

  useEffect(() => { setPage(1); }, [filter, pageSize]);

  return (
    <section className="bc-panel overflow-hidden">
      <div className="border-b border-border/60 p-4">
        <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
          <div><h2 className="text-lg font-semibold">交易</h2><p className="mt-0.5 text-xs text-muted-foreground">{rows.length} / {snapshot.transactions.total} 笔 · 默认每页 {TX_PAGE_SIZE_DEFAULT} 条</p></div>
          <div className="flex gap-2"><button type="button" className="bc-action" onClick={() => setShowAdvanced((v) => !v)}><Filter size={14}/>筛选</button><button type="button" className="bc-action" title="当前 LifeTrace Web 端为只读"><Download size={14}/>导出</button></div>
        </div>
        <div className="bc-toolbar mt-3">
          <div className="grid gap-2 lg:grid-cols-[1fr_180px_auto]">
            <label className="relative"><Search size={15} className="absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground"/><input className="bc-input pl-9" placeholder="搜索备注、账户、分类、标签" value={filter.q} onChange={(e) => setFilter((f) => ({ ...f, q: e.target.value }))}/></label>
            <select className="bc-input" value={filter.type} onChange={(e) => setFilter((f) => ({ ...f, type: e.target.value }))}><option value="all">全部类型</option><option value="expense">支出</option><option value="income">收入</option><option value="transfer">转账</option><option value="refund">退款</option><option value="fee">手续费</option></select>
            <button type="button" className="bc-action" onClick={() => setFilter(EMPTY_FILTER)}>重置</button>
          </div>
          {showAdvanced ? <div className="mt-2 grid gap-2 sm:grid-cols-2 xl:grid-cols-4"><input className="bc-input" type="date" value={filter.dateFrom} onChange={(e) => setFilter((f) => ({ ...f, dateFrom: e.target.value }))}/><input className="bc-input" type="date" value={filter.dateTo} onChange={(e) => setFilter((f) => ({ ...f, dateTo: e.target.value }))}/><select className="bc-input" value={filter.category} onChange={(e) => setFilter((f) => ({ ...f, category: e.target.value }))}><option value="">全部分类</option>{snapshot.categories.map((c) => <option key={c.id} value={c.name}>{c.name}</option>)}</select><select className="bc-input" value={filter.tag} onChange={(e) => setFilter((f) => ({ ...f, tag: e.target.value }))}><option value="">全部标签</option>{snapshot.tags.map((t) => <option key={t.id} value={t.name}>{t.name}</option>)}</select><input className="bc-input" inputMode="decimal" placeholder="最小金额" value={filter.amountMin} onChange={(e) => setFilter((f) => ({ ...f, amountMin: e.target.value }))}/><input className="bc-input" inputMode="decimal" placeholder="最大金额" value={filter.amountMax} onChange={(e) => setFilter((f) => ({ ...f, amountMax: e.target.value }))}/></div> : null}
        </div>
      </div>

      <div className="hidden overflow-x-auto md:block">
        <table className="w-full text-sm"><thead className="bc-table-head"><tr><th className="px-4 text-left font-medium">日期</th><th className="px-4 text-left font-medium">分类 / 备注</th><th className="px-4 text-left font-medium">账户</th><th className="px-4 text-left font-medium">标签</th><th className="px-4 text-right font-medium">金额</th></tr></thead><tbody className="divide-y divide-border/50">{shown.map((tx) => <TransactionTableRow key={tx.id} tx={tx} privacy={privacy}/>)}</tbody></table>
      </div>
      <div className="divide-y divide-border/50 md:hidden">{shown.map((tx) => <TransactionMobileRow key={tx.id} tx={tx} privacy={privacy}/>)}</div>
      {!shown.length ? <Empty title="没有匹配的交易" description="调整筛选条件后再试。" embedded /> : null}
      <Pager page={safePage} pages={pages} pageSize={pageSize} total={rows.length} onPage={setPage} onPageSize={setPageSize}/>
    </section>
  );
}

function TransactionTableRow({ tx, privacy }: { tx: BeeCountTransaction; privacy: boolean }) {
  const income = tx.transactionType === "income";
  return <tr className="hover:bg-muted/25"><td className="whitespace-nowrap px-4 py-3 text-xs text-muted-foreground">{txDate(tx)}</td><td className="max-w-80 px-4 py-3"><div className="truncate font-medium">{tx.categoryName || tx.note || typeLabel(tx.transactionType)}</div>{tx.note && tx.categoryName ? <div className="mt-0.5 truncate text-xs text-muted-foreground">{tx.note}</div> : null}</td><td className="px-4 py-3 text-muted-foreground">{tx.accountName || tx.fromAccountName || "—"}</td><td className="px-4 py-3"><div className="flex flex-wrap gap-1">{tx.tags.slice(0, 3).map((tag) => <span key={tag} className="rounded-full bg-primary/10 px-2 py-0.5 text-[10px] text-primary">{tag}</span>)}</div></td><td className={`whitespace-nowrap px-4 py-3 text-right font-semibold ${income ? "text-income" : "text-expense"}`}>{income ? "+" : "-"}{formatMoney(Math.abs(tx.amountCents), tx.currency, privacy)}</td></tr>;
}

function TransactionMobileRow({ tx, privacy }: { tx: BeeCountTransaction; privacy: boolean }) {
  const income = tx.transactionType === "income";
  return <div className="flex items-center gap-3 p-4"><div className={`flex h-9 w-9 shrink-0 items-center justify-center rounded-xl text-xs font-bold ${income ? "bg-income/10 text-income" : "bg-expense/10 text-expense"}`}>{income ? "收" : "支"}</div><div className="min-w-0 flex-1"><div className="truncate text-sm font-medium">{tx.categoryName || tx.note || typeLabel(tx.transactionType)}</div><div className="mt-0.5 truncate text-xs text-muted-foreground">{txDate(tx)} · {tx.accountName || tx.fromAccountName || "未指定账户"}</div></div><strong className={income ? "text-income" : "text-expense"}>{income ? "+" : "-"}{formatMoney(Math.abs(tx.amountCents), tx.currency, privacy)}</strong></div>;
}

function Pager({ page, pages, pageSize, total, onPage, onPageSize }: { page: number; pages: number; pageSize: number; total: number; onPage: (page: number) => void; onPageSize: (size: number) => void }) {
  return <div className="flex flex-col gap-2 border-t border-border/60 px-4 py-3 sm:flex-row sm:items-center sm:justify-between"><div className="text-xs text-muted-foreground">共 {total} 条 · 第 {page}/{pages} 页</div><div className="flex items-center gap-2"><select className="rounded-lg border border-border/70 bg-card px-2 py-1.5 text-xs" value={pageSize} onChange={(e) => onPageSize(Number(e.target.value))}><option value={20}>20 / 页</option><option value={50}>50 / 页</option><option value={100}>100 / 页</option></select><button className="bc-icon" disabled={page <= 1} onClick={() => onPage(page - 1)}><ChevronLeft size={15}/></button><button className="bc-icon" disabled={page >= pages} onClick={() => onPage(page + 1)}><ChevronRight size={15}/></button></div></div>;
}

function Calendar({ snapshot, privacy }: { snapshot: BeeCountLedgerSnapshot; privacy: boolean }) {
  const newest = snapshot.transactions.items[0];
  const initial = (newest ? txDate(newest) : new Date().toISOString().slice(0, 10)).slice(0, 7);
  const [month, setMonth] = useState(initial);
  const byDate = useMemo(() => {
    const map = new Map<string, { income: number; expense: number; count: number }>();
    for (const tx of snapshot.transactions.items) {
      const date = txDate(tx);
      if (!date.startsWith(month)) continue;
      const row = map.get(date) ?? { income: 0, expense: 0, count: 0 };
      row.count += 1;
      if (tx.transactionType === "income") row.income += tx.amountCents;
      else if (["expense", "fee"].includes(tx.transactionType)) row.expense += tx.amountCents;
      map.set(date, row);
    }
    return map;
  }, [snapshot.transactions.items, month]);
  const cells = calendarCells(month);
  return <section className="bc-panel overflow-hidden"><div className="flex items-center justify-between border-b border-border/60 p-4"><div><h2 className="text-lg font-semibold">日历</h2><p className="text-xs text-muted-foreground">按天查看当前账本收支</p></div><div className="flex items-center gap-2"><button className="bc-icon" onClick={() => setMonth(moveMonth(month, -1))}><ArrowLeft size={15}/></button><strong className="min-w-24 text-center text-sm">{month}</strong><button className="bc-icon" onClick={() => setMonth(moveMonth(month, 1))}><ArrowRight size={15}/></button></div></div><div className="grid grid-cols-7 border-b border-border/50 bg-muted/30 text-center text-[11px] text-muted-foreground">{"一二三四五六日".split("").map((d) => <div key={d} className="py-2">周{d}</div>)}</div><div className="grid grid-cols-7">{cells.map((cell, index) => { const stats = cell ? byDate.get(cell) : undefined; return <div key={`${cell}-${index}`} className="min-h-24 border-b border-r border-border/40 p-2 last:border-r-0 sm:min-h-28"><div className="text-xs font-medium">{cell ? Number(cell.slice(-2)) : ""}</div>{stats ? <div className="mt-2 space-y-1 text-[10px]"><div className="text-income">收 {formatMoney(stats.income, snapshot.ledger.currency, privacy)}</div><div className="text-expense">支 {formatMoney(stats.expense, snapshot.ledger.currency, privacy)}</div><div className="text-muted-foreground">{stats.count} 笔</div></div> : null}</div>; })}</div></section>;
}

function Accounts({ snapshot, privacy }: { snapshot: BeeCountLedgerSnapshot; privacy: boolean }) {
  const visible = snapshot.accounts.filter((a) => !a.hidden);
  const hidden = snapshot.accounts.filter((a) => a.hidden);
  return <div className="space-y-4"><EntityHeader title="账户 / 资产" description={`${visible.length} 个在用账户 · BeeCount user-global 账户`} /><div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">{visible.map((account) => <AccountCard key={account.id} account={account} currency={snapshot.ledger.currency} privacy={privacy}/>)}</div>{hidden.length ? <><SectionDivider>已隐藏</SectionDivider><div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3 opacity-70">{hidden.map((account) => <AccountCard key={account.id} account={account} currency={snapshot.ledger.currency} privacy={privacy}/>)}</div></> : null}</div>;
}

function AccountCard({ account, currency, privacy }: { account: BeeCountAccount; currency: string; privacy: boolean }) {
  return <article className="bc-panel bee-rise-in p-4"><div className="flex items-start justify-between gap-3"><div><div className="font-semibold">{account.name}</div><div className="mt-0.5 text-xs text-muted-foreground">{accountTypeLabel(account.accountType)} · {account.currency || currency}</div></div><Landmark size={18} className="text-primary"/></div><div className="mt-5 text-2xl font-semibold tracking-tight">{formatMoney(account.balanceCents ?? account.openingBalanceCents ?? 0, account.currency || currency, privacy)}</div><div className="mt-3 flex justify-between text-xs text-muted-foreground"><span>{account.transactionCount ?? 0} 笔交易</span><span>{account.note || ""}</span></div></article>;
}

function Categories({ snapshot }: { snapshot: BeeCountLedgerSnapshot }) {
  const expense = snapshot.categories.filter((c) => c.categoryType !== "income");
  const income = snapshot.categories.filter((c) => c.categoryType === "income");
  return <div className="grid gap-4 lg:grid-cols-2"><CategoryColumn title="支出分类" rows={expense}/><CategoryColumn title="收入分类" rows={income}/></div>;
}

function CategoryColumn({ title, rows }: { title: string; rows: BeeCountCategory[] }) {
  return <section className="bc-panel overflow-hidden"><div className="border-b border-border/60 p-4"><h2 className="font-semibold">{title}</h2><p className="mt-0.5 text-xs text-muted-foreground">{rows.length} 个分类</p></div><div className="divide-y divide-border/50">{rows.map((c) => <div key={c.id} className="flex items-center gap-3 px-4 py-3"><div className="flex h-9 w-9 items-center justify-center rounded-xl bg-primary/10 text-sm text-primary">{c.icon || c.name.slice(0, 1)}</div><div className="min-w-0 flex-1"><div className="truncate text-sm font-medium">{c.name}</div><div className="text-xs text-muted-foreground">{c.parentName ? `${c.parentName} · ` : ""}{c.transactionCount ?? 0} 笔交易</div></div><span className="text-xs text-muted-foreground">#{c.sortOrder ?? 0}</span></div>)}</div></section>;
}

function TagsView({ snapshot, privacy }: { snapshot: BeeCountLedgerSnapshot; privacy: boolean }) {
  const rows = [...snapshot.tags].sort((a, b) => (b.transactionCount ?? 0) - (a.transactionCount ?? 0));
  return <div className="space-y-4"><EntityHeader title="标签" description={`${rows.length} 个 BeeCount 标签`} /><div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">{rows.map((tag) => <TagCard key={tag.id} tag={tag} currency={snapshot.ledger.currency} privacy={privacy}/>)}</div></div>;
}

function TagCard({ tag, currency, privacy }: { tag: BeeCountTag; currency: string; privacy: boolean }) {
  return <article className="bc-panel p-4"><div className="flex items-center gap-2"><span className="h-2.5 w-2.5 rounded-full bg-primary"/><strong className="truncate">{tag.name}</strong></div><div className="mt-4 text-xl font-semibold">{tag.transactionCount ?? 0} <span className="text-xs font-normal text-muted-foreground">笔</span></div><div className="mt-2 text-xs text-muted-foreground">支出 {formatMoney(tag.expenseTotalCents ?? 0, currency, privacy)}</div></article>;
}

function Budgets({ snapshot, privacy }: { snapshot: BeeCountLedgerSnapshot; privacy: boolean }) {
  const currentMonth = new Date().toISOString().slice(0, 7);
  const spentByCategory = new Map<string, number>();
  for (const tx of snapshot.transactions.items) {
    if (!txDate(tx).startsWith(currentMonth) || !["expense", "fee"].includes(tx.transactionType)) continue;
    const key = tx.categoryName || "总预算";
    spentByCategory.set(key, (spentByCategory.get(key) ?? 0) + tx.amountCents);
  }
  if (!snapshot.budgets.length) return <Empty title="暂无预算" description="当前账本还没有 BeeCount 预算。" />;
  return <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">{snapshot.budgets.map((budget) => { const spent = budget.categoryName ? spentByCategory.get(budget.categoryName) ?? 0 : [...spentByCategory.values()].reduce((a, b) => a + b, 0); const ratio = budget.amountCents > 0 ? Math.min(100, Math.round(spent / budget.amountCents * 100)) : 0; return <article className="bc-panel p-4" key={budget.id}><div className="flex items-center justify-between"><strong>{budget.categoryName || "总预算"}</strong><span className="rounded-full bg-primary/10 px-2 py-0.5 text-[10px] text-primary">{budget.period}</span></div><div className="mt-4 flex items-end justify-between gap-2"><div className="text-xl font-semibold">{formatMoney(spent, snapshot.ledger.currency, privacy)}</div><div className="text-xs text-muted-foreground">/ {formatMoney(budget.amountCents, snapshot.ledger.currency, privacy)}</div></div><div className="mt-3 h-2 overflow-hidden rounded-full bg-muted"><div className="h-full rounded-full bg-primary transition-all" style={{ width: `${ratio}%` }}/></div><div className="mt-2 text-xs text-muted-foreground">已使用 {ratio}% · 每期第 {budget.startDay} 日开始</div></article>; })}</div>;
}

function Ledgers({ rows, privacy, activeId, onSelect }: { rows: BeeCountLedger[]; privacy: boolean; activeId: string; onSelect: (id: string) => void }) {
  return <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">{rows.map((ledger) => <button type="button" key={ledger.id} onClick={() => onSelect(ledger.sourceId)} className={`bc-panel p-4 text-left transition ${ledger.sourceId === activeId ? "ring-2 ring-primary/50" : "hover:-translate-y-0.5"}`}><div className="flex items-center justify-between"><div className="flex items-center gap-2"><WalletCards size={17} className="text-primary"/><strong>{ledger.name}</strong></div><span className="text-[10px] text-muted-foreground">{ledger.isShared ? "共享" : "个人"}</span></div><div className="mt-5 text-2xl font-semibold">{formatMoney(ledger.balanceCents, ledger.currency, privacy)}</div><div className="mt-3 flex justify-between text-xs text-muted-foreground"><span>{ledger.transactionCount} 笔</span><span>{ledger.role ?? "owner"}</span></div></button>)}</div>;
}

function ImportView() {
  return <section className="bc-panel p-5"><div className="flex items-start gap-4"><div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl bg-primary/10 text-primary"><Upload size={20}/></div><div><h2 className="text-lg font-semibold">导入交易</h2><p className="mt-1 max-w-2xl text-sm leading-6 text-muted-foreground">该页面沿用 BeeCount Cloud 的导入入口位置。当前 LifeTrace Web 适配层仍保持只读，导入应从 BeeCount 客户端完成；同步后这里会读取同一 PostgreSQL 数据源。后续若开放 Web 写接口，应直接接 BeeCount contract，而不是恢复 LifeTrace 原生财务导入。</p><div className="mt-4 rounded-xl border border-dashed border-primary/35 bg-primary/5 p-6 text-center"><Upload className="mx-auto text-primary"/><div className="mt-2 text-sm font-medium">BeeCount Web 写入接口尚未开放</div><div className="mt-1 text-xs text-muted-foreground">请在 BeeCount App 中导入后刷新此页面</div></div></div></div></section>;
}

function EntityHeader({ title, description }: { title: string; description: string }) {
  return <div><h2 className="text-lg font-semibold">{title}</h2><p className="mt-0.5 text-xs text-muted-foreground">{description}</p></div>;
}

function CategoryDonut({ rows, currency, privacy }: { rows: CategoryRank[]; currency: string; privacy: boolean }) {
  const data = rows.filter((r) => r.expense > 0).map((r) => ({ name: r.name, value: r.expense }));
  const total = data.reduce((sum, row) => sum + row.value, 0);
  return <section className="bc-panel p-4"><div className="mb-3"><h3 className="font-semibold">本期支出分类</h3><p className="text-xs text-muted-foreground">分类构成</p></div>{data.length ? <div className="grid gap-4 sm:grid-cols-[180px_1fr]"><div className="h-44"><ResponsiveContainer width="100%" height="100%"><PieChart><Pie data={data} dataKey="value" nameKey="name" innerRadius={48} outerRadius={72} paddingAngle={2}>{data.map((_, i) => <Cell key={i} fill={`hsl(var(--primary) / ${Math.max(.28, 1 - i * .09)})`}/>)}</Pie><Tooltip formatter={(value) => formatMoney(Number(value), currency, privacy)}/></PieChart></ResponsiveContainer></div><div className="space-y-2 self-center">{data.slice(0, 6).map((row, i) => <div key={row.name} className="flex items-center justify-between gap-3 text-xs"><span className="truncate text-muted-foreground">{i + 1}. {row.name}</span><strong>{total ? Math.round(row.value / total * 100) : 0}%</strong></div>)}</div></div> : <Empty title="暂无支出" embedded />}</section>;
}

function YearHeatmap({ items }: { items: BeeCountTransaction[] }) {
  const days = new Map<string, number>();
  for (const tx of items) { const date = txDate(tx); days.set(date, (days.get(date) ?? 0) + 1); }
  const end = new Date();
  const cells = Array.from({ length: 126 }, (_, index) => { const d = new Date(end); d.setDate(end.getDate() - (125 - index)); const key = d.toISOString().slice(0, 10); return { key, count: days.get(key) ?? 0 }; });
  const max = Math.max(1, ...cells.map((c) => c.count));
  return <section className="bc-panel p-4"><h3 className="font-semibold">记账热力</h3><p className="mb-4 text-xs text-muted-foreground">最近 18 周</p><div className="grid grid-flow-col grid-rows-7 gap-1 overflow-hidden">{cells.map((cell) => <div key={cell.key} title={`${cell.key}: ${cell.count} 笔`} className="aspect-square min-w-2 rounded-[3px] bg-primary" style={{ opacity: cell.count ? .22 + .78 * cell.count / max : .07 }}/>)}</div></section>;
}

function AssetDonut({ accounts, currency, privacy }: { accounts: BeeCountAccount[]; currency: string; privacy: boolean }) {
  const data = accounts.filter((a) => (a.balanceCents ?? 0) !== 0).map((a) => ({ name: a.name, value: Math.abs(a.balanceCents ?? 0) })).sort((a, b) => b.value - a.value).slice(0, 8);
  return <section className="bc-panel p-4"><h3 className="font-semibold">资产构成</h3><p className="mb-3 text-xs text-muted-foreground">按账户余额</p>{data.length ? <div className="h-56"><ResponsiveContainer width="100%" height="100%"><PieChart><Pie data={data} dataKey="value" nameKey="name" innerRadius={55} outerRadius={85}>{data.map((_, i) => <Cell key={i} fill={`hsl(var(--primary) / ${Math.max(.25, 1 - i * .09)})`}/>)}</Pie><Tooltip formatter={(value) => formatMoney(Number(value), currency, privacy)}/></PieChart></ResponsiveContainer></div> : <Empty title="暂无资产" embedded />}</section>;
}

function MonthlyTrend({ rows, currency, privacy }: { rows: Array<{ month: string; income: number; expense: number }>; currency: string; privacy: boolean }) {
  return <section className="bc-panel p-4"><h3 className="font-semibold">月度趋势</h3><p className="mb-3 text-xs text-muted-foreground">最近 12 个月</p><div className="h-56"><ResponsiveContainer width="100%" height="100%"><BarChart data={rows}><CartesianGrid strokeDasharray="3 3" vertical={false} opacity={.2}/><XAxis dataKey="month" tick={{ fontSize: 10 }} axisLine={false} tickLine={false}/><YAxis hide/><Tooltip formatter={(value) => formatMoney(Number(value), currency, privacy)}/><Bar dataKey="expense" name="支出" fill="rgb(var(--expense-rgb))" radius={[3, 3, 0, 0]}/><Bar dataKey="income" name="收入" fill="rgb(var(--income-rgb))" radius={[3, 3, 0, 0]}/></BarChart></ResponsiveContainer></div></section>;
}

function RankPanel({ title, rows, currency, privacy, onOpen }: { title: string; rows: Array<{ name: string; value: number }>; currency: string; privacy: boolean; onOpen: () => void }) {
  const max = Math.max(1, ...rows.map((r) => Math.abs(r.value)));
  return <section className="bc-panel p-4"><div className="flex items-center justify-between"><h3 className="font-semibold">{title}</h3><button type="button" onClick={onOpen} className="text-xs font-medium text-primary hover:underline">查看全部</button></div><div className="mt-4 space-y-3">{rows.length ? rows.map((row, index) => <div key={`${row.name}-${index}`}><div className="flex items-center justify-between gap-3 text-xs"><span className="truncate text-muted-foreground">{index + 1}. {row.name || "未分类"}</span><strong>{formatMoney(row.value, currency, privacy)}</strong></div><div className="mt-1 h-1.5 overflow-hidden rounded-full bg-muted"><div className="h-full rounded-full bg-primary" style={{ width: `${Math.max(4, Math.abs(row.value) / max * 100)}%` }}/></div></div>) : <div className="py-8 text-center text-xs text-muted-foreground">暂无数据</div>}</div></section>;
}

function Empty({ title, description, embedded = false }: { title: string; description?: string; embedded?: boolean }) {
  return <div className={embedded ? "px-4 py-10 text-center" : "bc-panel px-5 py-16 text-center"}><div className="text-sm font-semibold">{title}</div>{description ? <div className="mx-auto mt-1 max-w-lg text-xs leading-5 text-muted-foreground">{description}</div> : null}</div>;
}

type CategoryRank = { name: string; income: number; expense: number; count: number };

function rankByCategory(items: BeeCountTransaction[]): CategoryRank[] {
  const map = new Map<string, CategoryRank>();
  for (const tx of items) {
    const name = tx.categoryName || "未分类";
    const row = map.get(name) ?? { name, income: 0, expense: 0, count: 0 };
    row.count += 1;
    if (tx.transactionType === "income") row.income += tx.amountCents;
    else if (["expense", "fee"].includes(tx.transactionType)) row.expense += tx.amountCents;
    map.set(name, row);
  }
  return [...map.values()].sort((a, b) => (b.expense + b.income) - (a.expense + a.income));
}

function rankTags(items: BeeCountTransaction[]) {
  const map = new Map<string, number>();
  for (const tx of items) for (const tag of tx.tags) map.set(tag, (map.get(tag) ?? 0) + Math.abs(tx.amountCents));
  return [...map.entries()].map(([name, value]) => ({ name, value })).sort((a, b) => b.value - a.value);
}

function summarize(items: BeeCountTransaction[]) {
  let income = 0; let expense = 0;
  for (const tx of items) {
    if (tx.excludeFromStats) continue;
    if (tx.transactionType === "income") income += tx.amountCents;
    else if (["expense", "fee"].includes(tx.transactionType)) expense += tx.amountCents;
  }
  return { income, expense, count: items.length };
}

function txDate(tx: BeeCountTransaction): string {
  return tx.localDate || tx.occurredAt.slice(0, 10);
}

function applyTxFilter(items: BeeCountTransaction[], filter: TxFilter): BeeCountTransaction[] {
  const q = filter.q.trim().toLocaleLowerCase("zh-CN");
  const min = filter.amountMin.trim() ? Number(filter.amountMin) * 100 : null;
  const max = filter.amountMax.trim() ? Number(filter.amountMax) * 100 : null;
  return items.filter((tx) => {
    const date = txDate(tx);
    if (filter.type !== "all" && tx.transactionType !== filter.type) return false;
    if (filter.dateFrom && date < filter.dateFrom) return false;
    if (filter.dateTo && date > filter.dateTo) return false;
    if (filter.category && tx.categoryName !== filter.category) return false;
    if (filter.tag && !tx.tags.includes(filter.tag)) return false;
    const amount = Math.abs(tx.amountCents);
    if (min !== null && Number.isFinite(min) && amount < min) return false;
    if (max !== null && Number.isFinite(max) && amount > max) return false;
    if (!q) return true;
    return [tx.note, tx.accountName, tx.fromAccountName, tx.toAccountName, tx.categoryName, date, ...tx.tags]
      .filter(Boolean).join(" ").toLocaleLowerCase("zh-CN").includes(q);
  });
}

function typeLabel(type: string): string {
  const labels: Record<string, string> = { expense: "支出", income: "收入", transfer: "转账", refund: "退款", fee: "手续费" };
  return labels[type] ?? type;
}

function accountTypeLabel(type?: string | null): string {
  const labels: Record<string, string> = { cash: "现金", bank_card: "银行卡", credit_card: "信用卡", alipay: "支付宝", wechat: "微信", investment: "投资" };
  return type ? labels[type] ?? type : "账户";
}

function moveMonth(month: string, delta: number): string {
  const [y, m] = month.split("-").map(Number);
  const d = new Date(y, m - 1 + delta, 1);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}`;
}

function calendarCells(month: string): Array<string | null> {
  const [year, mon] = month.split("-").map(Number);
  const first = new Date(year, mon - 1, 1);
  const days = new Date(year, mon, 0).getDate();
  const mondayIndex = (first.getDay() + 6) % 7;
  const cells: Array<string | null> = Array.from({ length: mondayIndex }, () => null);
  for (let day = 1; day <= days; day += 1) cells.push(`${month}-${String(day).padStart(2, "0")}`);
  while (cells.length % 7) cells.push(null);
  return cells;
}
