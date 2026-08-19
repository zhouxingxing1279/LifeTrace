import { useEffect, useMemo, useState, type FormEvent } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { Bar, BarChart, ResponsiveContainer, Tooltip, XAxis } from "recharts";
import { Cloud, Plus, RefreshCw, Tags, Upload, WalletCards } from "lucide-react";

import { useApp } from "../../app/AppContext";
import {
  Badge,
  Button,
  Card,
  CardContent,
  EmptyState,
  Input,
  MetricCard,
  PageHeader,
  Progress,
  Section,
  cn,
} from "../../components/ui";
import { entities, number, sum, text, todayKey } from "../../lib/entities";
import {
  createBudgetPreference,
  createFinanceAccount,
  createFinanceCategory,
  createTransaction,
  formatMoney,
  type BeeCountLedger,
  type BeeCountLedgerSnapshot,
  type BeeCountTransaction,
  type JsonEntity,
} from "../../services/core";
import {
  LifeTraceBeeCountAdapter,
  beeCountMonthSeries,
  filterBeeCountTransactions,
} from "./beecount/adapter";

const FINANCE_TABS = [
  ["overview", "概览"],
  ["transactions", "交易"],
  ["calendar", "日历"],
  ["ledgers", "账本"],
  ["budgets", "预算"],
  ["accounts", "账户"],
  ["categories", "分类"],
  ["tags", "标签"],
  ["import", "导入"],
] as const;

type FinanceView = (typeof FINANCE_TABS)[number][0];
type AppSession = ReturnType<typeof useApp>["session"];
type Upsert = ReturnType<typeof useApp>["upsert"];
type FinanceSource = "beecount" | "lifetrace";
type BudgetValue = { month: string; amountCents: number; categoryId: string | null };
type ImportRow = {
  date: string;
  type: "expense" | "income" | "refund" | "fee";
  amount: string;
  merchant: string;
  note: string;
};

function financeView(pathname: string): FinanceView {
  const last = pathname.split("/").filter(Boolean).at(-1) ?? "overview";
  return FINANCE_TABS.some(([id]) => id === last) ? (last as FinanceView) : "overview";
}

function transactionDate(entity: JsonEntity): string {
  return text(entity, "localDate") || text(entity, "occurredAt").slice(0, 10);
}

function nativeBalance(transactions: JsonEntity[]): number {
  return transactions.reduce((total, item) => {
    const type = text(item, "transactionType");
    const amount = number(item, "amountCents");
    if (type === "income") return total + amount;
    if (type === "expense" || type === "fee") return total - amount;
    return total;
  }, 0);
}

function readBudgetPreference(entity: JsonEntity): BudgetValue | null {
  if (!entity.value || typeof entity.value !== "object") return null;
  const value = entity.value as Record<string, unknown>;
  if (typeof value.month !== "string" || typeof value.amountCents !== "number") return null;
  return {
    month: value.month,
    amountCents: value.amountCents,
    categoryId: typeof value.categoryId === "string" ? value.categoryId : null,
  };
}

export function FinanceWorkspace() {
  const { state, session, upsert, privacy, online } = useApp();
  const location = useLocation();
  const navigate = useNavigate();
  const view = financeView(location.pathname);
  const adapter = useMemo(() => new LifeTraceBeeCountAdapter(), []);

  const accounts = entities(state, "finance.account");
  const categories = entities(state, "finance.category");
  const transactions = entities(state, "finance.transaction")
    .filter((item) => text(item, "status", "confirmed") === "confirmed")
    .sort((left, right) => text(right, "occurredAt").localeCompare(text(left, "occurredAt")));
  const budgetPreferences = entities(state, "user.preference")
    .filter((item) => text(item, "preferenceKey").startsWith("finance.budget."));

  const [source, setSource] = useState<FinanceSource>("beecount");
  const [ledgers, setLedgers] = useState<BeeCountLedger[]>([]);
  const [selectedLedgerId, setSelectedLedgerId] = useState("");
  const [snapshot, setSnapshot] = useState<BeeCountLedgerSnapshot | null>(null);
  const [enabled, setEnabled] = useState<boolean | null>(null);
  const [reachable, setReachable] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  async function loadBeeCount(preferredLedgerId = selectedLedgerId) {
    if (!online) return;
    setLoading(true);
    setError("");
    try {
      const status = await adapter.status();
      setEnabled(status.enabled);
      setReachable(status.upstreamReachable);
      if (!status.enabled || !status.upstreamReachable) {
        setLedgers([]);
        setSnapshot(null);
        setSource("lifetrace");
        return;
      }
      const response = await adapter.ledgers();
      setLedgers(response.items);
      const ledgerId = response.items.some((item) => item.sourceId === preferredLedgerId)
        ? preferredLedgerId
        : response.items[0]?.sourceId ?? "";
      setSelectedLedgerId(ledgerId);
      if (ledgerId) setSnapshot(await adapter.snapshot(ledgerId, 200, 0));
    } catch (cause) {
      setSource("lifetrace");
      setError(cause instanceof Error ? cause.message : "BeeCount Cloud 加载失败");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void loadBeeCount();
    // Adapter is stable for the lifetime of this page. Reload only on network recovery.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [online]);

  async function chooseLedger(ledgerId: string) {
    setSelectedLedgerId(ledgerId);
    setLoading(true);
    setError("");
    try {
      setSnapshot(await adapter.snapshot(ledgerId, 200, 0));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "BeeCount 账本加载失败");
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="page-shell">
      <PageHeader
        title="财务"
        description="页面信息架构来自 BeeCount Cloud Web 源码；LifeTrace 仅替换全局 AppShell、认证和 Design System。"
        action={
          <Button variant="outline" disabled={!online || loading} onClick={() => void loadBeeCount()}>
            <RefreshCw size={15} className={loading ? "animate-spin" : ""} />刷新
          </Button>
        }
      />

      <div className="mb-4 flex flex-col gap-3 rounded-lg border bg-card p-3 lg:flex-row lg:items-center lg:justify-between">
        <div className="flex flex-wrap items-center gap-2">
          <span className="flex items-center gap-2 text-sm font-semibold"><Cloud size={16} />数据源</span>
          <div className="flex rounded-md border p-0.5">
            <button
              className={cn("rounded px-3 py-1.5 text-xs", source === "beecount" && "bg-muted font-medium")}
              disabled={!enabled || !reachable}
              onClick={() => setSource("beecount")}
            >BeeCount Cloud</button>
            <button
              className={cn("rounded px-3 py-1.5 text-xs", source === "lifetrace" && "bg-muted font-medium")}
              onClick={() => setSource("lifetrace")}
            >LifeTrace Native</button>
          </div>
          {enabled === false ? <Badge>适配器未启用</Badge> : null}
          {enabled && !reachable ? <Badge className="text-warning">上游不可达</Badge> : null}
          {enabled && reachable ? <Badge className="text-success">BeeCount 已连接</Badge> : null}
        </div>
        {source === "beecount" ? (
          <label className="flex items-center gap-2 text-xs text-muted-foreground">
            当前账本
            <select
              className="h-9 max-w-60 rounded-md border bg-background px-2 text-sm text-foreground"
              value={selectedLedgerId}
              onChange={(event) => void chooseLedger(event.target.value)}
            >
              {ledgers.map((ledger) => <option key={ledger.id} value={ledger.sourceId}>{ledger.name} · {ledger.currency}</option>)}
            </select>
          </label>
        ) : <span className="text-xs text-muted-foreground">{transactions.length} 笔 LifeTrace Native 交易</span>}
      </div>

      {error ? <div className="mb-4 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">{error}</div> : null}

      <nav className="scrollbar-thin mb-5 flex gap-1 overflow-x-auto border-b pb-2" aria-label="财务导航">
        {FINANCE_TABS.map(([id, label]) => (
          <button
            key={id}
            className={cn("shrink-0 rounded-md px-3 py-2 text-sm text-muted-foreground hover:bg-muted", view === id && "bg-accent font-medium text-accent-foreground")}
            onClick={() => navigate(id === "overview" ? "/app/finance" : `/app/finance/${id}`)}
          >{label}</button>
        ))}
      </nav>

      {view === "overview" ? <Overview source={source} snapshot={snapshot} transactions={transactions} privacy={privacy} /> : null}
      {view === "transactions" ? <Transactions source={source} snapshot={snapshot} transactions={transactions} accounts={accounts} categories={categories} session={session} upsert={upsert} privacy={privacy} /> : null}
      {view === "calendar" ? <FinanceCalendar source={source} snapshot={snapshot} transactions={transactions} privacy={privacy} /> : null}
      {view === "ledgers" ? <Ledgers source={source} ledgers={ledgers} transactions={transactions} privacy={privacy} /> : null}
      {view === "budgets" ? <Budgets source={source} snapshot={snapshot} preferences={budgetPreferences} transactions={transactions} categories={categories} session={session} upsert={upsert} privacy={privacy} /> : null}
      {view === "accounts" ? <Accounts source={source} snapshot={snapshot} accounts={accounts} session={session} upsert={upsert} privacy={privacy} /> : null}
      {view === "categories" ? <Categories source={source} snapshot={snapshot} categories={categories} session={session} upsert={upsert} /> : null}
      {view === "tags" ? <FinanceTags source={source} snapshot={snapshot} /> : null}
      {view === "import" ? <ImportCsv accounts={accounts} categories={categories} session={session} upsert={upsert} /> : null}
    </div>
  );
}

function Overview({ source, snapshot, transactions, privacy }: {
  source: FinanceSource;
  snapshot: BeeCountLedgerSnapshot | null;
  transactions: JsonEntity[];
  privacy: boolean;
}) {
  const month = todayKey().slice(0, 7);
  const monthTx = transactions.filter((item) => transactionDate(item).startsWith(month));
  const nativeIncome = sum(monthTx.filter((item) => text(item, "transactionType") === "income").map((item) => number(item, "amountCents")));
  const nativeExpense = sum(monthTx.filter((item) => ["expense", "fee"].includes(text(item, "transactionType"))).map((item) => number(item, "amountCents")));
  const nativeSeries = useMemo(() => {
    const buckets = new Map<string, { month: string; income: number; expense: number }>();
    for (const item of transactions) {
      const key = transactionDate(item).slice(0, 7);
      const row = buckets.get(key) ?? { month: key, income: 0, expense: 0 };
      if (text(item, "transactionType") === "income") row.income += number(item, "amountCents");
      else if (["expense", "fee"].includes(text(item, "transactionType"))) row.expense += number(item, "amountCents");
      buckets.set(key, row);
    }
    return [...buckets.values()].sort((a, b) => a.month.localeCompare(b.month)).slice(-12);
  }, [transactions]);

  const bee = source === "beecount" ? snapshot : null;
  const income = bee?.ledger.incomeTotalCents ?? nativeIncome;
  const expense = bee?.ledger.expenseTotalCents ?? nativeExpense;
  const balance = bee?.ledger.balanceCents ?? nativeBalance(transactions);
  const count = bee?.ledger.transactionCount ?? transactions.length;
  const currency = bee?.ledger.currency ?? "CNY";
  const series = bee ? beeCountMonthSeries(bee) : nativeSeries;

  return <>
    <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
      <MetricCard label="余额" value={formatMoney(balance, currency, privacy)} hint={`${count} 笔交易`} />
      <MetricCard label="收入" value={formatMoney(income, currency, privacy)} hint={bee ? "BeeCount 汇总" : "本月"} />
      <MetricCard label="支出" value={formatMoney(expense, currency, privacy)} hint={bee ? "BeeCount 汇总" : "本月"} />
      <MetricCard label="结余率" value={income ? `${Math.round((income - expense) / income * 100)}%` : "—"} hint="收入 - 支出" />
    </div>
    <Section className="mt-6" title="收支趋势" description="Upstream Overview analytics pattern → LifeTrace adapter">
      <Card><CardContent className="pt-5"><div className="h-64">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={series}>
            <XAxis dataKey="month" tickLine={false} axisLine={false} tick={{ fontSize: 10 }} />
            <Tooltip />
            <Bar dataKey="expense" fill="hsl(var(--expense))" radius={[3, 3, 0, 0]} />
            <Bar dataKey="income" fill="hsl(var(--income))" radius={[3, 3, 0, 0]} />
          </BarChart>
        </ResponsiveContainer>
      </div></CardContent></Card>
    </Section>
  </>;
}

function Transactions({ source, snapshot, transactions, accounts, categories, session, upsert, privacy }: {
  source: FinanceSource;
  snapshot: BeeCountLedgerSnapshot | null;
  transactions: JsonEntity[];
  accounts: JsonEntity[];
  categories: JsonEntity[];
  session: AppSession;
  upsert: Upsert;
  privacy: boolean;
}) {
  const [query, setQuery] = useState("");
  const [filterType, setFilterType] = useState("all");
  const [showNew, setShowNew] = useState(false);
  const [amount, setAmount] = useState("");
  const [merchant, setMerchant] = useState("");
  const [type, setType] = useState<"expense" | "income">("expense");

  const beeRows = filterBeeCountTransactions(snapshot?.transactions.items ?? [], query, filterType);
  const nativeRows = transactions.filter((item) => {
    if (filterType !== "all" && text(item, "transactionType") !== filterType) return false;
    return `${text(item, "merchant")} ${text(item, "note")} ${transactionDate(item)}`.toLowerCase().includes(query.toLowerCase());
  });

  async function create(event: FormEvent) {
    event.preventDefault();
    if (!session) return;
    await upsert("finance.transaction", createTransaction(session.user.id, session.session.deviceId, {
      amount,
      type,
      merchant,
      accountId: accounts[0]?.meta.id ?? null,
      categoryId: categories.find((item) => text(item, "categoryType") === type)?.meta.id ?? null,
    }));
    setAmount("");
    setMerchant("");
    setShowNew(false);
  }

  return <>
    <div className="mb-4 flex flex-col gap-2 sm:flex-row">
      <Input placeholder="筛选备注、账户、分类或日期" value={query} onChange={(event) => setQuery(event.target.value)} />
      <select className="h-10 rounded-md border bg-background px-3 text-sm" value={filterType} onChange={(event) => setFilterType(event.target.value)}>
        <option value="all">全部类型</option><option value="expense">支出</option><option value="income">收入</option><option value="refund">退款</option><option value="fee">手续费</option>
      </select>
      {source === "lifetrace" ? <Button onClick={() => setShowNew(true)}><Plus size={15} />记一笔</Button> : <Badge className="h-10 px-3">BeeCount 适配器只读</Badge>}
    </div>

    {showNew ? <Card className="mb-4"><CardContent className="pt-5"><form className="grid gap-3 sm:grid-cols-[140px_180px_1fr_auto]" onSubmit={(event) => void create(event)}>
      <select className="h-10 rounded-md border bg-background px-3 text-sm" value={type} onChange={(event) => setType(event.target.value as "expense" | "income")}><option value="expense">支出</option><option value="income">收入</option></select>
      <Input type="number" step="0.01" min="0.01" value={amount} onChange={(event) => setAmount(event.target.value)} placeholder="金额" required />
      <Input value={merchant} onChange={(event) => setMerchant(event.target.value)} placeholder="商户 / 说明" />
      <div className="flex gap-2"><Button type="submit">保存</Button><Button variant="ghost" onClick={() => setShowNew(false)}>取消</Button></div>
    </form></CardContent></Card> : null}

    <Card>
      <div className="divide-y">
        {source === "beecount" ? beeRows.map((item) => <BeeTransactionRow key={item.id} item={item} privacy={privacy} />) : nativeRows.map((item) => (
          <div key={item.meta.id} className="flex items-center gap-3 px-4 py-3">
            <div className={cn("flex h-8 w-8 items-center justify-center rounded-md text-xs font-semibold", text(item, "transactionType") === "income" ? "bg-success/10 text-success" : "bg-destructive/10 text-destructive")}>{text(item, "transactionType") === "income" ? "收" : "支"}</div>
            <div className="min-w-0 flex-1"><div className="truncate text-sm font-medium">{text(item, "merchant", text(item, "note", "交易"))}</div><div className="mt-0.5 text-xs text-muted-foreground">{transactionDate(item)}</div></div>
            <strong className={text(item, "transactionType") === "income" ? "text-income" : "text-expense"}>{formatMoney(number(item, "amountCents"), text(item, "currency", "CNY"), privacy)}</strong>
          </div>
        ))}
        {source === "beecount" && !beeRows.length ? <CardContent><EmptyState title="没有匹配的 BeeCount 交易" /></CardContent> : null}
        {source === "lifetrace" && !nativeRows.length ? <CardContent><EmptyState title="没有匹配的 LifeTrace 交易" /></CardContent> : null}
      </div>
    </Card>
  </>;
}

function BeeTransactionRow({ item, privacy }: { item: BeeCountTransaction; privacy: boolean }) {
  const income = item.transactionType === "income";
  return <div className="flex items-center gap-3 px-4 py-3">
    <div className={cn("flex h-8 w-8 items-center justify-center rounded-md text-xs font-semibold", income ? "bg-success/10 text-success" : "bg-destructive/10 text-destructive")}>{income ? "收" : "支"}</div>
    <div className="min-w-0 flex-1">
      <div className="truncate text-sm font-medium">{item.note || item.categoryName || item.accountName || "BeeCount 交易"}</div>
      <div className="mt-0.5 truncate text-xs text-muted-foreground">{item.localDate || item.occurredAt.slice(0, 10)} · {[item.accountName, item.categoryName, ...item.tags].filter(Boolean).join(" · ")}</div>
    </div>
    <strong className={income ? "text-income" : "text-expense"}>{income ? "+" : "-"}{formatMoney(item.amountCents, item.currency, privacy)}</strong>
  </div>;
}

function FinanceCalendar({ source, snapshot, transactions, privacy }: { source: FinanceSource; snapshot: BeeCountLedgerSnapshot | null; transactions: JsonEntity[]; privacy: boolean }) {
  const rows = source === "beecount"
    ? (snapshot?.transactions.items ?? []).map((item) => ({ date: item.localDate || item.occurredAt.slice(0, 10), type: item.transactionType, amount: item.amountCents, currency: item.currency }))
    : transactions.map((item) => ({ date: transactionDate(item), type: text(item, "transactionType"), amount: number(item, "amountCents"), currency: text(item, "currency", "CNY") }));
  const dates = new Map<string, { income: number; expense: number; count: number; currency: string }>();
  for (const item of rows) {
    const value = dates.get(item.date) ?? { income: 0, expense: 0, count: 0, currency: item.currency };
    value.count += 1;
    if (item.type === "income") value.income += item.amount;
    else if (item.type === "expense" || item.type === "fee") value.expense += item.amount;
    dates.set(item.date, value);
  }
  if (!dates.size) return <EmptyState title="暂无财务日历数据" description="交易发生后会按日期聚合。" />;
  return <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">{[...dates.entries()].sort((a, b) => b[0].localeCompare(a[0])).slice(0, 60).map(([date, value]) => <Card key={date}><CardContent className="pt-5"><div className="flex justify-between"><strong>{date}</strong><Badge>{value.count} 笔</Badge></div><div className="mt-4 grid grid-cols-2 gap-3 text-sm"><div><div className="text-xs text-muted-foreground">收入</div><strong className="text-income">{formatMoney(value.income, value.currency, privacy)}</strong></div><div><div className="text-xs text-muted-foreground">支出</div><strong className="text-expense">{formatMoney(value.expense, value.currency, privacy)}</strong></div></div></CardContent></Card>)}</div>;
}

function Ledgers({ source, ledgers, transactions, privacy }: { source: FinanceSource; ledgers: BeeCountLedger[]; transactions: JsonEntity[]; privacy: boolean }) {
  if (source === "lifetrace") return <Card><CardContent className="pt-5"><div className="flex items-center gap-2 font-semibold"><WalletCards size={17} />LifeTrace Native</div><div className="mt-4 text-2xl font-semibold">{formatMoney(nativeBalance(transactions), "CNY", privacy)}</div><div className="mt-1 text-xs text-muted-foreground">{transactions.length} 笔交易</div></CardContent></Card>;
  return <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">{ledgers.map((ledger) => <Card key={ledger.id}><CardContent className="pt-5"><div className="flex justify-between"><strong>{ledger.name}</strong><Badge>{ledger.isShared ? "共享" : "个人"}</Badge></div><div className="mt-4 text-xl font-semibold">{formatMoney(ledger.balanceCents, ledger.currency, privacy)}</div><div className="mt-1 text-xs text-muted-foreground">{ledger.transactionCount} 笔 · {ledger.role ?? "viewer"}</div></CardContent></Card>)}</div>;
}

function Budgets({ source, snapshot, preferences, transactions, categories, session, upsert, privacy }: {
  source: FinanceSource;
  snapshot: BeeCountLedgerSnapshot | null;
  preferences: JsonEntity[];
  transactions: JsonEntity[];
  categories: JsonEntity[];
  session: AppSession;
  upsert: Upsert;
  privacy: boolean;
}) {
  const [amount, setAmount] = useState("");
  const month = todayKey().slice(0, 7);
  async function create(event: FormEvent) {
    event.preventDefault();
    if (!session) return;
    await upsert("user.preference", createBudgetPreference(session.user.id, session.session.deviceId, month, amount));
    setAmount("");
  }
  if (source === "beecount") {
    const budgets = snapshot?.budgets ?? [];
    if (!budgets.length) return <EmptyState title="BeeCount 暂无预算" description="当前 BeeCount 兼容接口只读展示预算。" />;
    return <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">{budgets.map((budget) => <Card key={budget.id}><CardContent className="pt-5"><div className="flex justify-between"><strong>{budget.categoryName || "总预算"}</strong><Badge>{budget.period}</Badge></div><div className="mt-4 text-xl font-semibold">{formatMoney(budget.amountCents, snapshot?.ledger.currency ?? "CNY", privacy)}</div><div className="mt-1 text-xs text-muted-foreground">每期第 {budget.startDay} 日开始 · {budget.enabled ? "启用" : "停用"}</div></CardContent></Card>)}</div>;
  }
  const rows = preferences.map((item) => ({ item, value: readBudgetPreference(item) })).filter((row) => row.value !== null) as Array<{ item: JsonEntity; value: BudgetValue }>;
  return <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_340px]">
    <div className="grid gap-4 md:grid-cols-2">{rows.map(({ item, value }) => {
      const spent = sum(transactions.filter((tx) => transactionDate(tx).startsWith(value.month) && (!value.categoryId || tx.categoryId === value.categoryId) && ["expense", "fee"].includes(text(tx, "transactionType"))).map((tx) => number(tx, "amountCents")));
      const percent = value.amountCents ? Math.round(spent / value.amountCents * 100) : 0;
      return <Card key={item.meta.id}><CardContent className="pt-5"><div className="flex justify-between"><strong>{value.categoryId ? text(categories.find((category) => category.meta.id === value.categoryId), "name", "分类预算") : "总预算"}</strong><Badge>{value.month}</Badge></div><div className="mt-4 text-sm">{formatMoney(spent, "CNY", privacy)} / {formatMoney(value.amountCents, "CNY", privacy)}</div><Progress className="mt-2" value={percent} /></CardContent></Card>;
    })}{!rows.length ? <EmptyState title="暂无预算" /> : null}</div>
    <Card className="h-fit"><CardContent className="pt-5"><div className="font-semibold">创建本月总预算</div><form className="mt-4 space-y-3" onSubmit={(event) => void create(event)}><Input type="number" min="0.01" step="0.01" value={amount} onChange={(event) => setAmount(event.target.value)} placeholder="预算金额" required /><Button className="w-full" type="submit">保存预算</Button></form></CardContent></Card>
  </div>;
}

function Accounts({ source, snapshot, accounts, session, upsert, privacy }: { source: FinanceSource; snapshot: BeeCountLedgerSnapshot | null; accounts: JsonEntity[]; session: AppSession; upsert: Upsert; privacy: boolean }) {
  const [name, setName] = useState("");
  async function create(event: FormEvent) { event.preventDefault(); if (!session) return; await upsert("finance.account", createFinanceAccount(session.user.id, session.session.deviceId, name)); setName(""); }
  const rows = source === "beecount"
    ? (snapshot?.accounts ?? []).map((item) => ({ id: item.id, name: item.name, type: item.accountType || "账户", balance: item.balanceCents ?? item.openingBalanceCents ?? 0, currency: item.currency || snapshot?.ledger.currency || "CNY" }))
    : accounts.map((item) => ({ id: item.meta.id, name: text(item, "name", "账户"), type: text(item, "accountType", "账户"), balance: number(item, "openingBalanceCents"), currency: text(item, "currency", "CNY") }));
  return <><div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">{rows.map((item) => <Card key={item.id}><CardContent className="pt-5"><div className="flex justify-between"><strong>{item.name}</strong><Badge>{item.type}</Badge></div><div className="mt-4 text-xl font-semibold">{formatMoney(item.balance, item.currency, privacy)}</div></CardContent></Card>)}</div>{source === "lifetrace" ? <Card className="mt-5"><CardContent className="pt-5"><form className="flex gap-2" onSubmit={(event) => void create(event)}><Input value={name} onChange={(event) => setName(event.target.value)} placeholder="新账户名称" required /><Button type="submit"><Plus size={15} />新建账户</Button></form></CardContent></Card> : null}</>;
}

function Categories({ source, snapshot, categories, session, upsert }: { source: FinanceSource; snapshot: BeeCountLedgerSnapshot | null; categories: JsonEntity[]; session: AppSession; upsert: Upsert }) {
  const [name, setName] = useState("");
  const [type, setType] = useState<"expense" | "income">("expense");
  async function create(event: FormEvent) { event.preventDefault(); if (!session) return; await upsert("finance.category", createFinanceCategory(session.user.id, session.session.deviceId, name, type)); setName(""); }
  const rows = source === "beecount" ? (snapshot?.categories ?? []).map((item) => ({ id: item.id, name: item.name, type: item.categoryType, count: item.transactionCount ?? 0 })) : categories.map((item) => ({ id: item.meta.id, name: text(item, "name"), type: text(item, "categoryType"), count: 0 }));
  return <><div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">{rows.map((item) => <Card key={item.id}><CardContent className="pt-5"><div className="flex justify-between"><strong>{item.name}</strong><Badge>{item.type === "income" ? "收入" : "支出"}</Badge></div><div className="mt-2 text-xs text-muted-foreground">{item.count} 笔交易</div></CardContent></Card>)}</div>{source === "lifetrace" ? <Card className="mt-5"><CardContent className="pt-5"><form className="grid gap-2 sm:grid-cols-[1fr_150px_auto]" onSubmit={(event) => void create(event)}><Input value={name} onChange={(event) => setName(event.target.value)} placeholder="分类名称" required /><select className="h-10 rounded-md border bg-background px-3 text-sm" value={type} onChange={(event) => setType(event.target.value as "expense" | "income")}><option value="expense">支出</option><option value="income">收入</option></select><Button type="submit">新建分类</Button></form></CardContent></Card> : null}</>;
}

function FinanceTags({ source, snapshot }: { source: FinanceSource; snapshot: BeeCountLedgerSnapshot | null }) {
  if (source === "lifetrace") return <EmptyState icon={<Tags size={24} />} title="LifeTrace Native 暂无财务标签实体" description="保留 BeeCount 标签视图，但不伪造 native 标签数据。" />;
  const tags = snapshot?.tags ?? [];
  if (!tags.length) return <EmptyState title="BeeCount 当前账本没有标签" />;
  return <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">{tags.map((tag) => <Card key={tag.id}><CardContent className="pt-5"><div className="flex items-center gap-2"><span className="h-3 w-3 rounded-full border" style={{ background: tag.color || undefined }} /><strong>{tag.name}</strong></div><div className="mt-2 text-xs text-muted-foreground">{tag.transactionCount ?? 0} 笔交易</div></CardContent></Card>)}</div>;
}

function ImportCsv({ accounts, categories, session, upsert }: { accounts: JsonEntity[]; categories: JsonEntity[]; session: AppSession; upsert: Upsert }) {
  const [rows, setRows] = useState<ImportRow[]>([]);
  const [message, setMessage] = useState("");
  async function readFile(file: File) {
    const lines = (await file.text()).split(/\r?\n/).filter(Boolean);
    if (lines.length < 2) { setRows([]); setMessage("CSV 没有数据行"); return; }
    const headers = lines[0].split(",").map((value) => value.trim().toLowerCase());
    const index = (name: string) => headers.indexOf(name);
    const parsed = lines.slice(1).map((line) => {
      const columns = line.split(",").map((value) => value.trim().replace(/^"|"$/g, ""));
      const rawType = columns[index("type")] || "expense";
      const type = (["expense", "income", "refund", "fee"].includes(rawType) ? rawType : "expense") as ImportRow["type"];
      return { date: columns[index("date")] || todayKey(), type, amount: columns[index("amount")] || "0", merchant: columns[index("merchant")] || "", note: columns[index("note")] || "" };
    }).filter((row) => Number(row.amount) > 0);
    setRows(parsed);
    setMessage(`${parsed.length} 行可导入`);
  }
  async function commit() {
    if (!session) return;
    try {
      for (const row of rows) {
        await upsert("finance.transaction", createTransaction(session.user.id, session.session.deviceId, {
          amount: row.amount,
          type: row.type,
          localDate: row.date,
          occurredAt: new Date(`${row.date}T12:00:00`).toISOString(),
          merchant: row.merchant,
          note: row.note,
          accountId: accounts[0]?.meta.id ?? null,
          categoryId: categories.find((item) => text(item, "categoryType") === row.type)?.meta.id ?? null,
          sourceType: "web_csv_import",
        }));
      }
      setMessage(`已导入 ${rows.length} 笔`);
      setRows([]);
    } catch (cause) {
      setMessage(cause instanceof Error ? cause.message : "导入失败");
    }
  }
  return <div className="grid gap-5 lg:grid-cols-[360px_minmax(0,1fr)]">
    <Card className="h-fit"><CardContent className="pt-5"><div className="flex items-center gap-2 font-semibold"><Upload size={17} />CSV 导入</div><p className="mt-2 text-xs leading-5 text-muted-foreground">表头：date,type,amount,merchant,note。文件仅在内存中解析。</p><label className="mt-4 flex cursor-pointer items-center justify-center rounded-md border border-dashed p-6 text-sm hover:bg-muted"><input className="hidden" type="file" accept=".csv,text/csv" onChange={(event) => { const file = event.target.files?.[0]; if (file) void readFile(file); }} />选择 CSV</label>{message ? <div className="mt-3 text-xs text-muted-foreground">{message}</div> : null}{rows.length ? <Button className="mt-4 w-full" onClick={() => void commit()}>确认导入 {rows.length} 笔</Button> : null}</CardContent></Card>
    <Card><div className="divide-y">{rows.length ? rows.slice(0, 100).map((row, index) => <div key={`${row.date}-${index}`} className="grid grid-cols-[90px_70px_90px_1fr] gap-2 px-4 py-2.5 text-xs"><span>{row.date}</span><span>{row.type}</span><span>{row.amount}</span><span className="truncate">{row.merchant || row.note || "—"}</span></div>) : <CardContent className="pt-5"><EmptyState title="等待选择文件" description="导入前先预览，确认后才写入 LifeTrace Cloud。" /></CardContent>}</div></Card>
  </div>;
}
