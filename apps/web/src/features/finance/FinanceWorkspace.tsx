import { useEffect, useMemo, useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { Bar, BarChart, ResponsiveContainer, Tooltip, XAxis } from "recharts";
import { Cloud, RefreshCw, Upload, WalletCards } from "lucide-react";

import { useApp } from "../../app/AppContext";
import { Badge, Button, Card, CardContent, EmptyState, Input, MetricCard, PageHeader, Progress, Section, cn } from "../../components/ui";
import { formatMoney, type BeeCountLedger, type BeeCountLedgerSnapshot, type BeeCountTransaction } from "../../services/core";
import { LifeTraceBeeCountAdapter, beeCountMonthSeries, filterBeeCountTransactions } from "./beecount/adapter";

const FINANCE_TABS = [
  ["overview", "概览"], ["transactions", "交易"], ["calendar", "日历"], ["ledgers", "账本"],
  ["budgets", "预算"], ["accounts", "账户"], ["categories", "分类"], ["tags", "标签"], ["import", "导入"],
] as const;
type FinanceView = (typeof FINANCE_TABS)[number][0];

function financeView(pathname: string): FinanceView {
  const last = pathname.split("/").filter(Boolean).at(-1) ?? "overview";
  return FINANCE_TABS.some(([id]) => id === last) ? last as FinanceView : "overview";
}

export function FinanceWorkspace() {
  const { privacy, online } = useApp();
  const location = useLocation();
  const navigate = useNavigate();
  const view = financeView(location.pathname);
  const adapter = useMemo(() => new LifeTraceBeeCountAdapter(), []);
  const [ledgers, setLedgers] = useState<BeeCountLedger[]>([]);
  const [selectedLedgerId, setSelectedLedgerId] = useState("");
  const [snapshot, setSnapshot] = useState<BeeCountLedgerSnapshot | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  async function load(preferredLedgerId = selectedLedgerId) {
    if (!online) { setError("当前离线，无法读取 BeeCount 财务数据"); return; }
    setLoading(true); setError("");
    try {
      const status = await adapter.status();
      if (!status.enabled) throw new Error("BeeCount 财务服务未启用");
      if (!status.upstreamReachable) throw new Error("BeeCount 财务服务暂时不可用");
      const response = await adapter.ledgers();
      setLedgers(response.items);
      const ledgerId = response.items.some((item) => item.sourceId === preferredLedgerId) ? preferredLedgerId : response.items[0]?.sourceId ?? "";
      setSelectedLedgerId(ledgerId);
      setSnapshot(ledgerId ? await adapter.snapshot(ledgerId, 500, 0) : null);
    } catch (cause) {
      setLedgers([]); setSnapshot(null);
      setError(cause instanceof Error ? cause.message : "BeeCount 财务加载失败");
    } finally { setLoading(false); }
  }

  useEffect(() => { void load(); /* eslint-disable-next-line react-hooks/exhaustive-deps */ }, [online]);

  async function chooseLedger(ledgerId: string) {
    setSelectedLedgerId(ledgerId); setLoading(true); setError("");
    try { setSnapshot(await adapter.snapshot(ledgerId, 500, 0)); }
    catch (cause) { setError(cause instanceof Error ? cause.message : "BeeCount 账本加载失败"); }
    finally { setLoading(false); }
  }

  return <div className="page-shell">
    <PageHeader title="财务" description="财务域由 BeeCount 实现；LifeTrace 只提供统一登录、导航和页面外壳。" action={<Button variant="outline" disabled={!online || loading} onClick={() => void load()}><RefreshCw size={15} className={loading ? "animate-spin" : ""}/>刷新</Button>}/>
    <div className="mb-4 flex flex-col gap-3 rounded-lg border bg-card p-3 lg:flex-row lg:items-center lg:justify-between">
      <div className="flex items-center gap-2"><Cloud size={16}/><span className="text-sm font-semibold">BeeCount</span><Badge className="text-success">唯一财务数据源</Badge></div>
      <label className="flex items-center gap-2 text-xs text-muted-foreground">当前账本<select className="h-9 max-w-64 rounded-md border bg-background px-2 text-sm text-foreground" value={selectedLedgerId} onChange={(event) => void chooseLedger(event.target.value)}>{ledgers.map((ledger) => <option key={ledger.id} value={ledger.sourceId}>{ledger.name} · {ledger.currency}</option>)}</select></label>
    </div>
    {error ? <div className="mb-4 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">{error}。财务模块不会回退到 LifeTrace 原生财务数据。</div> : null}
    <nav className="scrollbar-thin mb-5 flex gap-1 overflow-x-auto border-b pb-2" aria-label="财务导航">{FINANCE_TABS.map(([id,label]) => <button key={id} className={cn("shrink-0 rounded-md px-3 py-2 text-sm text-muted-foreground hover:bg-muted", view===id && "bg-accent font-medium text-accent-foreground")} onClick={() => navigate(id === "overview" ? "/app/finance" : `/app/finance/${id}`)}>{label}</button>)}</nav>
    {!snapshot && !loading ? <EmptyState title="暂无 BeeCount 财务数据" description="请确认 BeeCount 服务已连接且至少存在一个账本。"/> : null}
    {snapshot && view === "overview" ? <Overview snapshot={snapshot} privacy={privacy}/> : null}
    {snapshot && view === "transactions" ? <Transactions snapshot={snapshot} privacy={privacy}/> : null}
    {snapshot && view === "calendar" ? <Calendar snapshot={snapshot} privacy={privacy}/> : null}
    {view === "ledgers" ? <Ledgers ledgers={ledgers} privacy={privacy}/> : null}
    {snapshot && view === "budgets" ? <Budgets snapshot={snapshot} privacy={privacy}/> : null}
    {snapshot && view === "accounts" ? <Accounts snapshot={snapshot} privacy={privacy}/> : null}
    {snapshot && view === "categories" ? <Categories snapshot={snapshot}/> : null}
    {snapshot && view === "tags" ? <TagsView snapshot={snapshot} privacy={privacy}/> : null}
    {view === "import" ? <ImportView/> : null}
  </div>;
}

function Overview({ snapshot, privacy }: { snapshot: BeeCountLedgerSnapshot; privacy: boolean }) {
  const { ledger } = snapshot; const series = beeCountMonthSeries(snapshot); const rate = ledger.incomeTotalCents ? Math.round((ledger.incomeTotalCents-ledger.expenseTotalCents)/ledger.incomeTotalCents*100) : 0;
  return <><div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4"><MetricCard label="余额" value={formatMoney(ledger.balanceCents,ledger.currency,privacy)} hint={`${ledger.transactionCount} 笔交易`}/><MetricCard label="收入" value={formatMoney(ledger.incomeTotalCents,ledger.currency,privacy)} hint="BeeCount 汇总"/><MetricCard label="支出" value={formatMoney(ledger.expenseTotalCents,ledger.currency,privacy)} hint="BeeCount 汇总"/><MetricCard label="结余率" value={`${rate}%`} hint="收入 - 支出"/></div><Section className="mt-6" title="收支趋势" description="BeeCount 账本月度趋势"><Card><CardContent className="pt-5"><div className="h-64"><ResponsiveContainer width="100%" height="100%"><BarChart data={series}><XAxis dataKey="month" tickLine={false} axisLine={false} tick={{fontSize:10}}/><Tooltip/><Bar dataKey="expense" fill="hsl(var(--expense))" radius={[3,3,0,0]}/><Bar dataKey="income" fill="hsl(var(--income))" radius={[3,3,0,0]}/></BarChart></ResponsiveContainer></div></CardContent></Card></Section></>;
}

function Transactions({ snapshot, privacy }: { snapshot: BeeCountLedgerSnapshot; privacy: boolean }) {
  const [query,setQuery]=useState(""); const [type,setType]=useState("all"); const rows=filterBeeCountTransactions(snapshot.transactions.items,query,type);
  return <><div className="mb-4 flex flex-col gap-2 sm:flex-row"><Input placeholder="筛选备注、账户、分类、标签或日期" value={query} onChange={(event)=>setQuery(event.target.value)}/><select className="h-10 rounded-md border bg-background px-3 text-sm" value={type} onChange={(event)=>setType(event.target.value)}><option value="all">全部类型</option><option value="expense">支出</option><option value="income">收入</option><option value="refund">退款</option><option value="fee">手续费</option></select></div><Card><div className="divide-y">{rows.map((item)=><TransactionRow key={item.id} item={item} privacy={privacy}/>)}{!rows.length?<CardContent className="pt-5"><EmptyState title="没有匹配的 BeeCount 交易"/></CardContent>:null}</div></Card></>;
}

function TransactionRow({item,privacy}:{item:BeeCountTransaction;privacy:boolean}) { const income=item.transactionType==="income"; return <div className="flex items-center gap-3 px-4 py-3"><div className={cn("flex h-8 w-8 items-center justify-center rounded-md text-xs font-semibold",income?"bg-success/10 text-success":"bg-destructive/10 text-destructive")}>{income?"收":"支"}</div><div className="min-w-0 flex-1"><div className="truncate text-sm font-medium">{item.note||item.categoryName||item.accountName||"BeeCount 交易"}</div><div className="mt-0.5 truncate text-xs text-muted-foreground">{item.localDate||item.occurredAt.slice(0,10)} · {[item.accountName,item.categoryName,...item.tags].filter(Boolean).join(" · ")}</div></div><strong className={income?"text-income":"text-expense"}>{income?"+":"-"}{formatMoney(item.amountCents,item.currency,privacy)}</strong></div>; }

function Calendar({snapshot,privacy}:{snapshot:BeeCountLedgerSnapshot;privacy:boolean}) { const dates=new Map<string,{income:number;expense:number;count:number}>(); for(const item of snapshot.transactions.items){const date=item.localDate||item.occurredAt.slice(0,10);const row=dates.get(date)??{income:0,expense:0,count:0};row.count++;if(item.transactionType==="income")row.income+=item.amountCents;else if(item.transactionType==="expense"||item.transactionType==="fee")row.expense+=item.amountCents;dates.set(date,row);} return <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">{[...dates.entries()].sort((a,b)=>b[0].localeCompare(a[0])).slice(0,60).map(([date,row])=><Card key={date}><CardContent className="pt-5"><div className="flex justify-between"><strong>{date}</strong><Badge>{row.count} 笔</Badge></div><div className="mt-4 grid grid-cols-2 gap-3 text-sm"><div><div className="text-xs text-muted-foreground">收入</div><strong className="text-income">{formatMoney(row.income,snapshot.ledger.currency,privacy)}</strong></div><div><div className="text-xs text-muted-foreground">支出</div><strong className="text-expense">{formatMoney(row.expense,snapshot.ledger.currency,privacy)}</strong></div></div></CardContent></Card>)}</div>; }

function Ledgers({ledgers,privacy}:{ledgers:BeeCountLedger[];privacy:boolean}) { return <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">{ledgers.map((ledger)=><Card key={ledger.id}><CardContent className="pt-5"><div className="flex justify-between"><div className="flex items-center gap-2"><WalletCards size={16}/><strong>{ledger.name}</strong></div><Badge>{ledger.isShared?"共享":"个人"}</Badge></div><div className="mt-4 text-xl font-semibold">{formatMoney(ledger.balanceCents,ledger.currency,privacy)}</div><div className="mt-1 text-xs text-muted-foreground">{ledger.transactionCount} 笔 · {ledger.role??"member"}</div></CardContent></Card>)}</div>; }

function Budgets({snapshot,privacy}:{snapshot:BeeCountLedgerSnapshot;privacy:boolean}) { if(!snapshot.budgets.length)return <EmptyState title="BeeCount 暂无预算"/>; return <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">{snapshot.budgets.map((budget)=><Card key={budget.id}><CardContent className="pt-5"><div className="flex justify-between"><strong>{budget.categoryName||"总预算"}</strong><Badge>{budget.period}</Badge></div><div className="mt-4 text-xl font-semibold">{formatMoney(budget.amountCents,snapshot.ledger.currency,privacy)}</div><Progress className="mt-3" value={budget.enabled?100:0}/><div className="mt-2 text-xs text-muted-foreground">每期第 {budget.startDay} 日开始 · {budget.enabled?"启用":"停用"}</div></CardContent></Card>)}</div>; }

function Accounts({snapshot,privacy}:{snapshot:BeeCountLedgerSnapshot;privacy:boolean}) { return <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">{snapshot.accounts.map((account)=><Card key={account.id}><CardContent className="pt-5"><div className="flex justify-between"><strong>{account.name}</strong><Badge>{account.accountType||"账户"}</Badge></div><div className="mt-4 text-xl font-semibold">{formatMoney(account.balanceCents??account.openingBalanceCents??0,account.currency||snapshot.ledger.currency,privacy)}</div><div className="mt-1 text-xs text-muted-foreground">{account.transactionCount??0} 笔交易</div></CardContent></Card>)}</div>; }

function Categories({snapshot}:{snapshot:BeeCountLedgerSnapshot}) { return <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">{snapshot.categories.map((category)=><Card key={category.id}><CardContent className="pt-5"><div className="flex justify-between"><strong>{category.name}</strong><Badge>{category.categoryType==="income"?"收入":"支出"}</Badge></div><div className="mt-2 text-xs text-muted-foreground">{category.parentName?`${category.parentName} · `:""}{category.transactionCount??0} 笔交易</div></CardContent></Card>)}</div>; }

function TagsView({snapshot,privacy}:{snapshot:BeeCountLedgerSnapshot;privacy:boolean}) { if(!snapshot.tags.length)return <EmptyState title="BeeCount 暂无标签"/>; return <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">{snapshot.tags.map((tag)=><Card key={tag.id}><CardContent className="pt-5"><strong>{tag.name}</strong><div className="mt-2 text-xs text-muted-foreground">{tag.transactionCount??0} 笔 · 支出 {formatMoney(tag.expenseTotalCents??0,snapshot.ledger.currency,privacy)}</div></CardContent></Card>)}</div>; }

function ImportView(){ return <Card><CardContent className="pt-6"><div className="flex items-start gap-3"><Upload size={20}/><div><div className="font-semibold">BeeCount 导入</div><p className="mt-1 text-sm leading-6 text-muted-foreground">财务导入属于 BeeCount 数据域。LifeTrace Web 不再把 CSV 转换成另一套 LifeTrace 财务实体，避免产生双数据源；请通过 BeeCount 的导入流程写入同一账本，写入后这里会直接读取结果。</p></div></div></CardContent></Card>; }
