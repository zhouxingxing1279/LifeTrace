import { useEffect, useMemo, useState, type FormEvent } from "react";
import { useNavigate } from "react-router-dom";
import { Pencil, Plus, RefreshCw, Trash2 } from "lucide-react";
import { useApp } from "../../app/AppContext";
import {
  Badge,
  Button,
  Card,
  CardContent,
  Dialog,
  EmptyState,
  Input,
  PageHeader,
  Select,
  cn,
} from "../../components/ui";
import { entities, number, text } from "../../lib/entities";
import {
  amountToCents,
  createTransaction,
  formatMoney,
  type BeeCountLedgerSnapshot,
  type BeeCountTransaction,
  type JsonEntity,
} from "../../services/core";
import { LifeTraceBeeCountAdapter, filterBeeCountTransactions } from "./beecount/adapter";

const financeTabs = [
  ["/app/finance", "概览"], ["/app/finance/transactions", "交易"], ["/app/finance/calendar", "日历"],
  ["/app/finance/ledgers", "账本"], ["/app/finance/budgets", "预算"], ["/app/finance/accounts", "账户"],
  ["/app/finance/categories", "分类"], ["/app/finance/tags", "标签"], ["/app/finance/import", "导入"],
] as const;

type Source = "lifetrace" | "beecount";
type TransactionType = "expense" | "income" | "refund" | "fee";

function transactionDate(entity: JsonEntity) {
  return text(entity, "localDate") || text(entity, "occurredAt").slice(0, 10);
}

export function FinanceTransactionsPage() {
  const { state, session, upsert, remove, privacy, online } = useApp();
  const navigate = useNavigate();
  const adapter = useMemo(() => new LifeTraceBeeCountAdapter(), []);
  const accounts = entities(state, "finance.account");
  const categories = entities(state, "finance.category");
  const transactions = entities(state, "finance.transaction")
    .filter((item) => text(item, "status", "confirmed") === "confirmed")
    .sort((left, right) => text(right, "occurredAt").localeCompare(text(left, "occurredAt")));

  const [source, setSource] = useState<Source>("lifetrace");
  const [snapshot, setSnapshot] = useState<BeeCountLedgerSnapshot | null>(null);
  const [beeCountAvailable, setBeeCountAvailable] = useState(false);
  const [loadingBeeCount, setLoadingBeeCount] = useState(false);
  const [query, setQuery] = useState("");
  const [filterType, setFilterType] = useState("all");
  const [editorOpen, setEditorOpen] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [type, setType] = useState<TransactionType>("expense");
  const [amount, setAmount] = useState("");
  const [merchant, setMerchant] = useState("");
  const [note, setNote] = useState("");
  const [accountId, setAccountId] = useState("");
  const [categoryId, setCategoryId] = useState("");
  const [localDate, setLocalDate] = useState(() => new Date().toISOString().slice(0, 10));

  async function loadBeeCount() {
    if (!online) return;
    setLoadingBeeCount(true);
    try {
      const status = await adapter.status();
      if (!status.enabled || !status.upstreamReachable) {
        setBeeCountAvailable(false);
        setSnapshot(null);
        return;
      }
      const ledgers = await adapter.ledgers();
      const ledger = ledgers.items[0];
      if (!ledger) {
        setBeeCountAvailable(false);
        return;
      }
      setBeeCountAvailable(true);
      setSnapshot(await adapter.snapshot(ledger.sourceId, 200, 0));
    } catch {
      setBeeCountAvailable(false);
      setSnapshot(null);
    } finally {
      setLoadingBeeCount(false);
    }
  }

  useEffect(() => { void loadBeeCount(); }, [online]);

  const nativeRows = useMemo(() => transactions.filter((item) => {
    if (filterType !== "all" && text(item, "transactionType") !== filterType) return false;
    const haystack = `${text(item, "merchant")} ${text(item, "note")} ${transactionDate(item)}`.toLocaleLowerCase("zh-CN");
    return haystack.includes(query.trim().toLocaleLowerCase("zh-CN"));
  }), [filterType, query, transactions]);

  const beeRows = useMemo(
    () => filterBeeCountTransactions(snapshot?.transactions.items ?? [], query, filterType),
    [filterType, query, snapshot?.transactions.items],
  );

  function resetEditor() {
    setEditingId(null);
    setType("expense");
    setAmount("");
    setMerchant("");
    setNote("");
    setAccountId(accounts[0]?.meta.id ?? "");
    setCategoryId("");
    setLocalDate(new Date().toISOString().slice(0, 10));
  }

  function openCreate() {
    resetEditor();
    setEditorOpen(true);
  }

  function openEdit(item: JsonEntity) {
    setEditingId(item.meta.id);
    setType(text(item, "transactionType", "expense") as TransactionType);
    setAmount((number(item, "amountCents") / 100).toFixed(2));
    setMerchant(text(item, "merchant"));
    setNote(text(item, "note"));
    setAccountId(typeof item.accountId === "string" ? item.accountId : "");
    setCategoryId(typeof item.categoryId === "string" ? item.categoryId : "");
    setLocalDate(transactionDate(item));
    setEditorOpen(true);
  }

  async function save(event: FormEvent) {
    event.preventDefault();
    if (!session) return;
    const existing = editingId ? transactions.find((item) => item.meta.id === editingId) : null;
    if (existing) {
      await upsert("finance.transaction", {
        ...existing,
        transactionType: type,
        amountCents: amountToCents(amount),
        merchant: merchant.trim() || null,
        note: note.trim() || null,
        accountId: accountId || null,
        categoryId: categoryId || null,
        localDate,
        occurredAt: new Date(`${localDate}T12:00:00`).toISOString(),
      });
    } else {
      await upsert("finance.transaction", createTransaction(session.user.id, session.session.deviceId, {
        amount,
        type,
        merchant,
        note,
        accountId: accountId || null,
        categoryId: categoryId || null,
        localDate,
        occurredAt: new Date(`${localDate}T12:00:00`).toISOString(),
        sourceType: "web_manual",
      }));
    }
    setEditorOpen(false);
    resetEditor();
  }

  async function deleteTransaction(id: string) {
    await remove("finance.transaction", id);
  }

  return <div className="page-shell">
    <PageHeader
      title="财务 · 交易"
      description="BeeCount Cloud Web Transactions 信息架构 + LifeTrace Native 可编辑 Cloud entity。BeeCount 兼容接口保持只读。"
      action={<div className="flex gap-2"><Button variant="outline" onClick={() => void loadBeeCount()} disabled={!online || loadingBeeCount}><RefreshCw size={15} className={loadingBeeCount ? "animate-spin" : ""} />刷新</Button><Button onClick={openCreate}><Plus size={15} />记一笔</Button></div>}
    />

    <nav className="scrollbar-thin mb-5 flex gap-1 overflow-x-auto border-b pb-2" aria-label="财务导航">
      {financeTabs.map(([path, label]) => <button key={path} onClick={() => navigate(path)} className={cn("shrink-0 rounded-md px-3 py-2 text-sm text-muted-foreground hover:bg-muted", path.endsWith("/transactions") && "bg-accent font-medium text-accent-foreground")}>{label}</button>)}
    </nav>

    <div className="mb-4 flex flex-col gap-2 sm:flex-row">
      <div className="flex rounded-md border p-0.5">
        <button className={cn("rounded px-3 py-1.5 text-xs", source === "lifetrace" && "bg-muted font-medium")} onClick={() => setSource("lifetrace")}>LifeTrace Native</button>
        <button className={cn("rounded px-3 py-1.5 text-xs", source === "beecount" && "bg-muted font-medium")} disabled={!beeCountAvailable} onClick={() => setSource("beecount")}>BeeCount Cloud</button>
      </div>
      <Input placeholder="筛选商户、备注或日期" value={query} onChange={(event) => setQuery(event.target.value)} />
      <Select className="sm:max-w-40" value={filterType} onChange={(event) => setFilterType(event.target.value)}>
        <option value="all">全部类型</option><option value="expense">支出</option><option value="income">收入</option><option value="refund">退款</option><option value="fee">手续费</option>
      </Select>
      {source === "beecount" ? <Badge className="h-10 px-3">只读上游</Badge> : null}
    </div>

    <Card>
      {source === "lifetrace" ? nativeRows.length ? <div className="divide-y">{nativeRows.map((item) => <NativeRow key={item.meta.id} item={item} privacy={privacy} onEdit={() => openEdit(item)} onDelete={() => void deleteTransaction(item.meta.id)} />)}</div> : <CardContent className="pt-5"><EmptyState title="没有匹配的 LifeTrace 交易" action={<Button variant="outline" onClick={openCreate}>新增交易</Button>} /></CardContent> : beeRows.length ? <div className="divide-y">{beeRows.map((item) => <BeeRow key={item.id} item={item} privacy={privacy} />)}</div> : <CardContent className="pt-5"><EmptyState title="没有匹配的 BeeCount 交易" description={beeCountAvailable ? "调整筛选条件。" : "BeeCount 适配器未启用或上游不可达。"} /></CardContent>}
    </Card>

    <Dialog open={editorOpen} onOpenChange={setEditorOpen} title={editingId ? "编辑交易" : "新增交易"} description="保存后通过现有 Cloud sync contract 写入，不使用浏览器本地持久化。">
      <form className="space-y-3" onSubmit={(event) => void save(event)}>
        <div className="grid grid-cols-2 gap-3"><label className="text-sm font-medium">类型<Select className="mt-1.5" value={type} onChange={(event) => setType(event.target.value as TransactionType)}><option value="expense">支出</option><option value="income">收入</option><option value="refund">退款</option><option value="fee">手续费</option></Select></label><label className="text-sm font-medium">金额<Input className="mt-1.5" type="number" min="0.01" step="0.01" value={amount} onChange={(event) => setAmount(event.target.value)} required /></label></div>
        <label className="block text-sm font-medium">日期<Input className="mt-1.5" type="date" value={localDate} onChange={(event) => setLocalDate(event.target.value)} required /></label>
        <label className="block text-sm font-medium">商户 / 对象<Input className="mt-1.5" value={merchant} onChange={(event) => setMerchant(event.target.value)} /></label>
        <label className="block text-sm font-medium">备注<Input className="mt-1.5" value={note} onChange={(event) => setNote(event.target.value)} /></label>
        <div className="grid grid-cols-2 gap-3"><label className="text-sm font-medium">账户<Select className="mt-1.5" value={accountId} onChange={(event) => setAccountId(event.target.value)}><option value="">未指定</option>{accounts.map((item) => <option key={item.meta.id} value={item.meta.id}>{text(item, "name", "账户")}</option>)}</Select></label><label className="text-sm font-medium">分类<Select className="mt-1.5" value={categoryId} onChange={(event) => setCategoryId(event.target.value)}><option value="">未分类</option>{categories.filter((item) => text(item, "categoryType") === type || type === "refund").map((item) => <option key={item.meta.id} value={item.meta.id}>{text(item, "name", "分类")}</option>)}</Select></label></div>
        <div className="flex justify-end gap-2 pt-2"><Button variant="ghost" onClick={() => setEditorOpen(false)}>取消</Button><Button type="submit">{editingId ? "保存修改" : "新增交易"}</Button></div>
      </form>
    </Dialog>
  </div>;
}

function NativeRow({ item, privacy, onEdit, onDelete }: { item: JsonEntity; privacy: boolean; onEdit(): void; onDelete(): void }) {
  const income = text(item, "transactionType") === "income";
  return <div className="group flex items-center gap-3 px-4 py-3"><div className={cn("flex h-8 w-8 items-center justify-center rounded-md text-xs font-semibold", income ? "bg-success/10 text-success" : "bg-destructive/10 text-destructive")}>{income ? "收" : "支"}</div><div className="min-w-0 flex-1"><div className="truncate text-sm font-medium">{text(item, "merchant", text(item, "note", "交易"))}</div><div className="mt-0.5 truncate text-xs text-muted-foreground">{transactionDate(item)} · {text(item, "transactionType", "expense")}</div></div><strong className={income ? "text-income" : "text-expense"}>{income ? "+" : "-"}{formatMoney(number(item, "amountCents"), text(item, "currency", "CNY"), privacy)}</strong><div className="flex opacity-100 sm:opacity-0 sm:group-hover:opacity-100"><Button size="icon" variant="ghost" onClick={onEdit} aria-label="编辑交易"><Pencil size={14} /></Button><Button size="icon" variant="ghost" onClick={onDelete} aria-label="删除交易"><Trash2 size={14} /></Button></div></div>;
}

function BeeRow({ item, privacy }: { item: BeeCountTransaction; privacy: boolean }) {
  const income = item.transactionType === "income";
  return <div className="flex items-center gap-3 px-4 py-3"><div className={cn("flex h-8 w-8 items-center justify-center rounded-md text-xs font-semibold", income ? "bg-success/10 text-success" : "bg-destructive/10 text-destructive")}>{income ? "收" : "支"}</div><div className="min-w-0 flex-1"><div className="truncate text-sm font-medium">{item.note || item.categoryName || item.accountName || "BeeCount 交易"}</div><div className="mt-0.5 truncate text-xs text-muted-foreground">{item.localDate || item.occurredAt.slice(0, 10)} · {[item.accountName, item.categoryName, ...item.tags].filter(Boolean).join(" · ")}</div></div><strong className={income ? "text-income" : "text-expense"}>{income ? "+" : "-"}{formatMoney(item.amountCents, item.currency, privacy)}</strong></div>;
}
