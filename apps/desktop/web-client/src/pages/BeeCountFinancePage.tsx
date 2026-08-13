import { useEffect, useMemo, useState } from "react";
import { Cloud, RefreshCw, Tags, WalletCards } from "lucide-react";

import {
  BeeCountFinanceApi,
  formatMoney,
  type BeeCountAccount,
  type BeeCountBudget,
  type BeeCountCategory,
  type BeeCountIntegrationStatus,
  type BeeCountLedger,
  type BeeCountLedgerSnapshot,
  type BeeCountTag,
  type BeeCountTransaction,
} from "../core";
import { Empty, FinanceTabs, Metric, Notice, PageStack, Panel } from "../ui";

const PAGE_SIZE = 50;

type BeeCountView = "overview" | "transactions" | "accounts" | "categories" | "tags" | "budgets";

const VIEWS: Array<{ id: BeeCountView; label: string }> = [
  { id: "overview", label: "概览" },
  { id: "transactions", label: "交易" },
  { id: "accounts", label: "账户" },
  { id: "categories", label: "分类" },
  { id: "tags", label: "标签" },
  { id: "budgets", label: "预算" },
];

export function filterBeeCountTransactions(
  items: BeeCountTransaction[],
  query: string,
  transactionType: string,
): BeeCountTransaction[] {
  const normalized = query.trim().toLocaleLowerCase("zh-CN");
  return items.filter((item) => {
    const matchesType = transactionType === "all" || item.transactionType === transactionType;
    if (!matchesType || !normalized) return matchesType;
    const haystack = [
      transactionTitle(item), item.note, item.accountName, item.fromAccountName,
      item.toAccountName, item.categoryName, item.localDate, ...item.tags,
    ].filter(Boolean).join(" ").toLocaleLowerCase("zh-CN");
    return haystack.includes(normalized);
  });
}

export function BeeCountFinancePage({ privacy, online }: { privacy: boolean; online: boolean }) {
  const api = useMemo(() => new BeeCountFinanceApi(), []);
  const [status, setStatus] = useState<BeeCountIntegrationStatus | null>(null);
  const [ledgers, setLedgers] = useState<BeeCountLedger[]>([]);
  const [selectedLedgerId, setSelectedLedgerId] = useState("");
  const [snapshot, setSnapshot] = useState<BeeCountLedgerSnapshot | null>(null);
  const [view, setView] = useState<BeeCountView>("overview");
  const [offset, setOffset] = useState(0);
  const [refreshVersion, setRefreshVersion] = useState(0);
  const [loading, setLoading] = useState(false);
  const [snapshotLoading, setSnapshotLoading] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    let active = true;
    if (!online) {
      setLoading(false);
      return () => { active = false; };
    }
    setLoading(true);
    setError("");
    api.status()
      .then(async (nextStatus) => {
        if (!active) return;
        setStatus(nextStatus);
        if (!nextStatus.enabled || !nextStatus.upstreamReachable) {
          setLedgers([]);
          setSelectedLedgerId("");
          setSnapshot(null);
          return;
        }
        const response = await api.ledgers();
        if (!active) return;
        setLedgers(response.items);
        setSelectedLedgerId((current) => response.items.some((ledger) => ledger.sourceId === current)
          ? current
          : response.items[0]?.sourceId ?? "");
      })
      .catch((cause: unknown) => {
        if (active) setError(cause instanceof Error ? cause.message : "BeeCount 云账本加载失败");
      })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [api, online, refreshVersion]);

  useEffect(() => {
    let active = true;
    if (!online || !selectedLedgerId || status?.enabled !== true || !status.upstreamReachable) {
      setSnapshotLoading(false);
      return () => { active = false; };
    }
    setSnapshotLoading(true);
    setError("");
    api.snapshot(selectedLedgerId, PAGE_SIZE, offset)
      .then((value) => { if (active) setSnapshot(value); })
      .catch((cause: unknown) => {
        if (active) setError(cause instanceof Error ? cause.message : "BeeCount 账本快照加载失败");
      })
      .finally(() => { if (active) setSnapshotLoading(false); });
    return () => { active = false; };
  }, [api, online, selectedLedgerId, offset, refreshVersion, status?.enabled, status?.upstreamReachable]);

  function chooseLedger(sourceId: string) {
    setSelectedLedgerId(sourceId);
    setOffset(0);
    setSnapshot(null);
  }

  function refresh() {
    setRefreshVersion((value) => value + 1);
  }

  return <PageStack>
    <FinanceTabs />
    <section className="beecount-toolbar" aria-label="BeeCount 账本工具栏">
      <div className="beecount-source"><Cloud /><div><strong>BeeCount Cloud</strong><small>iOS、BeeCount Web 与 LifeTrace 共用云端账本视图</small></div><span>只读</span></div>
      <label>当前账本<select value={selectedLedgerId} disabled={!ledgers.length || loading} onChange={(event) => chooseLedger(event.target.value)}>{ledgers.length ? ledgers.map((ledger) => <option key={ledger.id} value={ledger.sourceId}>{ledger.name} · {ledger.currency}</option>) : <option value="">暂无账本</option>}</select></label>
      <button className="hx-btn secondary" disabled={!online || loading || snapshotLoading} onClick={refresh}><RefreshCw className={loading || snapshotLoading ? "spin" : ""} />刷新云账本</button>
    </section>

    {!online && <Notice kind="warning">BeeCount 云账本不缓存到浏览器，请联网后查看。</Notice>}
    {error && <Notice kind="error">{error}</Notice>}
    {status && !status.enabled && <AdapterUnavailable title="适配器尚未启用" description="部署管理员需要配置 BeeCount 服务账号与绑定的 LifeTrace 用户；启用后无需再次登录 BeeCount。" />}
    {status?.enabled && !status.upstreamReachable && <AdapterUnavailable title="BeeCount Cloud 暂时不可达" description="LifeTrace 登录仍然有效；请检查 BeeCount 容器健康状态和适配器账号。" />}
    {loading && !status && <Panel eyebrow="BEECOUNT CLOUD" title="正在连接云账本"><p className="hx-muted">正在验证适配器并读取可用账本…</p></Panel>}
    {status?.enabled && status.upstreamReachable && !loading && !ledgers.length && <Empty title="BeeCount 中还没有账本" description="先在 BeeCount iOS 或 BeeCount Web 创建账本并同步，随后回到这里刷新。" />}

    {status?.enabled && status.upstreamReachable && ledgers.length > 0 && <>
      <nav className="beecount-view-tabs" aria-label="BeeCount 数据视图">{VIEWS.map((item) => <button key={item.id} className={view === item.id ? "active" : ""} aria-current={view === item.id ? "page" : undefined} onClick={() => setView(item.id)}>{item.label}</button>)}</nav>
      {snapshotLoading && !snapshot && <Panel eyebrow="LEDGER SNAPSHOT" title="正在读取账本"><p className="hx-muted">交易、账户、分类、标签和预算正在汇总…</p></Panel>}
      {snapshot && <BeeCountViewContent snapshot={snapshot} view={view} privacy={privacy} offset={offset} onOffsetChange={setOffset} />}
    </>}
  </PageStack>;
}

function AdapterUnavailable({ title, description }: { title: string; description: string }) {
  return <Panel eyebrow="BEECOUNT CLOUD" title={title}><p className="hx-muted">{description}</p><p className="beecount-boundary">此页面不会把 BeeCount 数据复制进 LifeTrace 原生财务库，也不会显示或保存 BeeCount 密码。</p></Panel>;
}

function BeeCountViewContent({ snapshot, view, privacy, offset, onOffsetChange }: {
  snapshot: BeeCountLedgerSnapshot;
  view: BeeCountView;
  privacy: boolean;
  offset: number;
  onOffsetChange: (value: number) => void;
}) {
  if (view === "overview") return <BeeCountOverview snapshot={snapshot} privacy={privacy} />;
  if (view === "transactions") return <BeeCountTransactions snapshot={snapshot} privacy={privacy} offset={offset} onOffsetChange={onOffsetChange} />;
  if (view === "accounts") return <BeeCountAccounts items={snapshot.accounts} currency={snapshot.ledger.currency} privacy={privacy} />;
  if (view === "categories") return <BeeCountCategories items={snapshot.categories} />;
  if (view === "tags") return <BeeCountTags items={snapshot.tags} currency={snapshot.ledger.currency} privacy={privacy} />;
  return <BeeCountBudgets items={snapshot.budgets} currency={snapshot.ledger.currency} privacy={privacy} />;
}

function BeeCountOverview({ snapshot, privacy }: { snapshot: BeeCountLedgerSnapshot; privacy: boolean }) {
  const { ledger } = snapshot;
  const recent = snapshot.transactions.items.slice(0, 8);
  return <div className="beecount-overview">
    <div className="metric-grid">
      <Metric label="账本余额" value={formatMoney(ledger.balanceCents, ledger.currency, privacy)} detail={`${ledger.transactionCount} 笔交易`} positive />
      <Metric label="累计收入" value={formatMoney(ledger.incomeTotalCents, ledger.currency, privacy)} detail="BeeCount Cloud 汇总" positive />
      <Metric label="累计支出" value={formatMoney(ledger.expenseTotalCents, ledger.currency, privacy)} detail={`每月 ${ledger.monthStartDay || 1} 日起算`} />
      <Metric label="账户与预算" value={`${snapshot.accounts.length} / ${snapshot.budgets.length}`} detail={`${snapshot.categories.length} 个分类 · ${snapshot.tags.length} 个标签`} />
    </div>
    <div className="hx-content-grid two">
      <Panel eyebrow="RECENT" title="最近交易"><BeeCountTransactionRows items={recent} privacy={privacy} /></Panel>
      <Panel eyebrow="LEDGER" title={ledger.name}><div className="beecount-ledger-summary"><Cloud /><strong>{ledger.currency}</strong><p>{ledger.isShared ? `${ledger.memberCount ?? 1} 人共享账本` : "个人账本"} · 权限 {ledger.role ?? "viewer"}</p><small>读取于 {formatDateTime(snapshot.fetchedAt)}</small></div></Panel>
    </div>
  </div>;
}

function BeeCountTransactions({ snapshot, privacy, offset, onOffsetChange }: {
  snapshot: BeeCountLedgerSnapshot;
  privacy: boolean;
  offset: number;
  onOffsetChange: (value: number) => void;
}) {
  const [query, setQuery] = useState("");
  const [transactionType, setTransactionType] = useState("all");
  const items = useMemo(() => filterBeeCountTransactions(snapshot.transactions.items, query, transactionType), [snapshot.transactions.items, query, transactionType]);
  const end = Math.min(offset + snapshot.transactions.items.length, snapshot.transactions.total);
  return <>
    <div className="filter-row beecount-filter"><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="筛选当前页的备注、账户、分类或标签" /><select value={transactionType} onChange={(event) => setTransactionType(event.target.value)}><option value="all">全部类型</option><option value="expense">支出</option><option value="income">收入</option><option value="transfer">转账</option><option value="refund">退款</option><option value="fee">手续费</option></select></div>
    <Panel eyebrow="TRANSACTIONS" title="交易明细" actions={<strong>{snapshot.transactions.total}</strong>}><BeeCountTransactionRows items={items} privacy={privacy} />{snapshot.transactions.total > PAGE_SIZE && <div className="beecount-pagination"><span>{offset + 1}–{end} / {snapshot.transactions.total}</span><div><button className="small-button" disabled={offset === 0} onClick={() => onOffsetChange(Math.max(0, offset - PAGE_SIZE))}>上一页</button><button className="small-button" disabled={end >= snapshot.transactions.total} onClick={() => onOffsetChange(offset + PAGE_SIZE)}>下一页</button></div></div>}</Panel>
  </>;
}

function BeeCountTransactionRows({ items, privacy }: { items: BeeCountTransaction[]; privacy: boolean }) {
  if (!items.length) return <Empty title="没有匹配的交易" description="切换筛选条件或在 BeeCount 客户端新增一笔记录。" />;
  return <div className="beecount-transactions">{items.map((item) => {
    const direction = transactionDirection(item.transactionType);
    return <article key={item.id} className="beecount-transaction"><span className={`record-icon ${item.transactionType}`}>{direction.symbol}</span><div><strong>{transactionTitle(item)}</strong><small>{formatDateTime(item.occurredAt)} · {transactionMeta(item)}</small>{item.tags.length > 0 && <div className="beecount-chips">{item.tags.map((tag) => <span key={tag}>{tag}</span>)}</div>}</div><b className={direction.className}>{direction.prefix}{formatMoney(item.amountCents, item.currency, privacy)}</b></article>;
  })}</div>;
}

function BeeCountAccounts({ items, currency, privacy }: { items: BeeCountAccount[]; currency: string; privacy: boolean }) {
  return <div className="beecount-card-grid">{items.map((item) => <article className="beecount-entity-card" key={item.id}><header><span><WalletCards /></span><i>{item.hidden ? "已隐藏" : "账户"}</i></header><h3>{item.name}</h3><p>{item.accountType || "未分类"} · {item.currency || currency}</p><strong>{formatMoney(item.balanceCents ?? item.openingBalanceCents ?? 0, item.currency || currency, privacy)}</strong><footer><span>{item.transactionCount ?? 0} 笔</span><span>支出 {formatMoney(item.expenseTotalCents ?? 0, item.currency || currency, privacy)}</span></footer></article>)}{!items.length && <Empty title="暂无账户" description="BeeCount 账本还没有可显示的账户。" />}</div>;
}

function BeeCountCategories({ items }: { items: BeeCountCategory[] }) {
  return <div className="beecount-card-grid">{items.map((item) => <article className="beecount-entity-card" key={item.id}><header><span className={item.categoryType}>{item.icon || item.name.slice(0, 1)}</span><i>{item.categoryType === "income" ? "收入" : "支出"}</i></header><h3>{item.name}</h3><p>{item.parentName ? `上级：${item.parentName}` : `层级 ${item.level ?? 1}`}</p><footer><span>{item.transactionCount ?? 0} 笔交易</span><span>排序 {item.sortOrder ?? "—"}</span></footer></article>)}{!items.length && <Empty title="暂无分类" description="BeeCount 账本还没有可显示的分类。" />}</div>;
}

function BeeCountTags({ items, currency, privacy }: { items: BeeCountTag[]; currency: string; privacy: boolean }) {
  return <div className="beecount-card-grid">{items.map((item) => <article className="beecount-entity-card" key={item.id}><header><span style={{ background: item.color || undefined }}><Tags /></span><i>标签</i></header><h3>{item.name}</h3><p>{item.transactionCount ?? 0} 笔交易</p><footer><span>收入 {formatMoney(item.incomeTotalCents ?? 0, currency, privacy)}</span><span>支出 {formatMoney(item.expenseTotalCents ?? 0, currency, privacy)}</span></footer></article>)}{!items.length && <Empty title="暂无标签" description="BeeCount 账本还没有可显示的标签。" />}</div>;
}

function BeeCountBudgets({ items, currency, privacy }: { items: BeeCountBudget[]; currency: string; privacy: boolean }) {
  return <div className="beecount-card-grid">{items.map((item) => <article className="beecount-entity-card" key={item.id}><header><span className="budget">¥</span><i>{item.enabled ? "生效中" : "已停用"}</i></header><h3>{item.categoryName || (item.budgetType === "total" ? "总预算" : "分类预算")}</h3><strong>{formatMoney(item.amountCents, currency, privacy)}</strong><p>{budgetPeriod(item.period)} · 每期第 {item.startDay} 日开始</p></article>)}{!items.length && <Empty title="暂无预算" description="BeeCount 账本还没有设置预算。" />}</div>;
}

function transactionTitle(item: BeeCountTransaction): string {
  if (item.transactionType === "transfer") return `${item.fromAccountName || item.accountName || "账户"} → ${item.toAccountName || "账户"}`;
  return item.note || item.categoryName || item.accountName || ({ income: "收入", refund: "退款", fee: "手续费" }[item.transactionType] ?? "支出");
}

function transactionMeta(item: BeeCountTransaction): string {
  return [item.accountName || item.fromAccountName, item.categoryName].filter(Boolean).join(" · ") || "未指定账户和分类";
}

function transactionDirection(type: string): { symbol: string; prefix: string; className: string } {
  if (type === "income" || type === "refund") return { symbol: "+", prefix: "+", className: "income" };
  if (type === "transfer") return { symbol: "↔", prefix: "", className: "transfer" };
  return { symbol: "−", prefix: "−", className: "expense" };
}

function formatDateTime(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString("zh-CN", { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
}

function budgetPeriod(value: string): string {
  return ({ monthly: "每月", weekly: "每周", yearly: "每年" } as Record<string, string>)[value] ?? value;
}
