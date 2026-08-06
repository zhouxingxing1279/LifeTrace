import { FormEvent, useMemo, useState } from "react";
import {
  createBudgetPreference,
  createFinanceAccount,
  createFinanceCategory,
  createTransaction,
  findProbableDuplicate,
  formatMoney,
  type JsonEntity,
} from "../core";
import { importBillFile, type ImportPreview } from "../importer";
import { CloudPageProps, Empty, FinanceTabs, Metric, Notice, PageStack, Panel, entities, number, text } from "../ui";

function transactionTitle(item: JsonEntity): string {
  return text(item, "merchant") || text(item, "item") || text(item, "note") || ({ income: "收入", refund: "退款", fee: "手续费" }[text(item, "transactionType")] ?? "支出");
}

function monthExpense(transactions: JsonEntity[], month: string): number {
  return transactions
    .filter((item) => text(item, "localDate").startsWith(month) && ["expense", "fee"].includes(text(item, "transactionType")) && text(item, "status") !== "ignored")
    .reduce((sum, item) => sum + number(item, "amountCents"), 0);
}

export function FinanceOverview(props: CloudPageProps) {
  const accounts = entities(props.state, "finance.account");
  const transactions = entities(props.state, "finance.transaction");
  const month = new Date().toISOString().slice(0, 7);
  const expense = monthExpense(transactions, month);
  const income = transactions.filter((item) => text(item, "localDate").startsWith(month) && text(item, "transactionType") === "income" && text(item, "status") !== "ignored").reduce((sum, item) => sum + number(item, "amountCents"), 0);
  const candidates = transactions.filter((item) => text(item, "status") === "candidate").length;
  const recent = [...transactions].sort((a, b) => text(b, "occurredAt").localeCompare(text(a, "occurredAt"))).slice(0, 8);
  return <PageStack><FinanceTabs /><div className="metric-grid"><Metric label="本月支出" value={formatMoney(expense, "CNY", props.privacy)} detail={`${transactions.length} 条账单`} /><Metric label="本月收入" value={formatMoney(income, "CNY", props.privacy)} detail={`结余 ${formatMoney(income - expense, "CNY", props.privacy)}`} /><Metric label="账户" value={String(accounts.length)} detail="云端资金账户" /><Metric label="待对账" value={String(candidates)} detail="导入后需要确认" /></div><QuickTransaction {...props} accounts={accounts} /><Panel title="最近账单" eyebrow="RECENT"><TransactionList items={recent} accounts={accounts} privacy={props.privacy} onDelete={(item) => props.run((store) => store.delete("finance.transaction", item.meta.id))} /></Panel></PageStack>;
}

function QuickTransaction(props: CloudPageProps & { accounts: JsonEntity[] }) {
  const categories = entities(props.state, "finance.category");
  const [amount, setAmount] = useState("");
  const [note, setNote] = useState("");
  const [type, setType] = useState<"expense" | "income">("expense");
  const [accountId, setAccountId] = useState("");
  const [categoryId, setCategoryId] = useState("");
  const [localError, setLocalError] = useState("");
  async function submit(event: FormEvent) {
    event.preventDefault(); setLocalError("");
    try {
      const entity = createTransaction(props.session.user.id, props.session.session.deviceId, { amount, type, note, accountId: accountId || null, categoryId: categoryId || null });
      await props.run((store) => store.upsert("finance.transaction", entity));
      setAmount(""); setNote("");
    } catch (cause) { setLocalError(cause instanceof Error ? cause.message : "保存失败"); }
  }
  return <Panel title="快速记账" eyebrow="QUICK ENTRY"><form className="form-grid" onSubmit={(event) => void submit(event)}><div className="segmented span-2"><button type="button" className={type === "expense" ? "active" : ""} onClick={() => setType("expense")}>支出</button><button type="button" className={type === "income" ? "active" : ""} onClick={() => setType("income")}>收入</button></div><label>金额（元）<input required inputMode="decimal" value={amount} onChange={(event) => setAmount(event.target.value)} placeholder="0.00" /></label><label>账户<select value={accountId} onChange={(event) => setAccountId(event.target.value)}><option value="">未指定</option>{props.accounts.map((item) => <option key={item.meta.id} value={item.meta.id}>{text(item, "name")}</option>)}</select></label><label>分类<select value={categoryId} onChange={(event) => setCategoryId(event.target.value)}><option value="">未分类</option>{categories.filter((item) => text(item, "categoryType") === type).map((item) => <option key={item.meta.id} value={item.meta.id}>{text(item, "name")}</option>)}</select></label><label>备注<input value={note} onChange={(event) => setNote(event.target.value)} placeholder="午餐、工资、交通…" /></label>{localError && <div className="span-2"><Notice kind="error">{localError}</Notice></div>}<button className="primary-button span-2" disabled={!props.online}>保存到云端</button></form></Panel>;
}

export function TransactionsPage(props: CloudPageProps) {
  const accounts = entities(props.state, "finance.account");
  const categories = entities(props.state, "finance.category");
  const [query, setQuery] = useState("");
  const [status, setStatus] = useState("all");
  const items = useMemo(() => entities(props.state, "finance.transaction").filter((item) => {
    const matchesQuery = `${transactionTitle(item)} ${text(item, "localDate")}`.toLowerCase().includes(query.toLowerCase());
    return matchesQuery && (status === "all" || text(item, "status") === status);
  }).sort((a, b) => text(b, "occurredAt").localeCompare(text(a, "occurredAt"))), [props.state, query, status]);
  async function updateStatus(item: JsonEntity, next: string) {
    await props.run((store) => store.upsert("finance.transaction", { ...item, status: next, meta: { ...item.meta } }));
  }
  return <PageStack><FinanceTabs /><div className="filter-row"><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索商户、备注或日期" /><select value={status} onChange={(event) => setStatus(event.target.value)}><option value="all">全部状态</option><option value="confirmed">已确认</option><option value="candidate">待对账</option><option value="ignored">已忽略</option></select></div><Panel title="账单列表" eyebrow="TRANSACTIONS" actions={<strong>{items.length}</strong>}><TransactionList items={items} accounts={accounts} categories={categories} privacy={props.privacy} onDelete={(item) => props.run((store) => store.delete("finance.transaction", item.meta.id))} onStatus={updateStatus} /></Panel></PageStack>;
}

function TransactionList({ items, accounts, categories = [], privacy, onDelete, onStatus }: { items: JsonEntity[]; accounts: JsonEntity[]; categories?: JsonEntity[]; privacy: boolean; onDelete: (item: JsonEntity) => Promise<unknown>; onStatus?: (item: JsonEntity, status: string) => Promise<void> }) {
  if (!items.length) return <Empty title="暂无账单" description="创建或导入一条账单后会显示在这里。" />;
  return <div className="record-list">{items.map((item) => {
    const type = text(item, "transactionType");
    const account = accounts.find((entry) => entry.meta.id === item.accountId);
    const category = categories.find((entry) => entry.meta.id === item.categoryId);
    return <div className="record-row" key={item.meta.id}><span className={`record-icon ${type}`}>{type === "income" ? "+" : "−"}</span><div className="record-main"><strong>{transactionTitle(item)}</strong><small>{text(item, "localDate")} · {account ? text(account, "name") : "未指定账户"}{category ? ` · ${text(category, "name")}` : ""}</small></div><span className={`status-pill ${text(item, "status")}`}>{text(item, "status")}</span><b className={type}>{type === "income" ? "+" : "−"}{formatMoney(number(item, "amountCents"), text(item, "currency") || "CNY", privacy)}</b>{onStatus && text(item, "status") === "candidate" && <><button className="small-button" onClick={() => void onStatus(item, "confirmed")}>确认</button><button className="small-button" onClick={() => void onStatus(item, "ignored")}>忽略</button></>}<button className="icon-button danger" aria-label="删除" onClick={() => void onDelete(item)}>×</button></div>;
  })}</div>;
}

export function AccountsPage(props: CloudPageProps) {
  const accounts = entities(props.state, "finance.account").sort((a, b) => text(a, "name").localeCompare(text(b, "name"), "zh-CN"));
  const [name, setName] = useState("");
  async function submit(event: FormEvent) { event.preventDefault(); await props.run((store) => store.upsert("finance.account", createFinanceAccount(props.session.user.id, props.session.session.deviceId, name))); setName(""); }
  return <PageStack><FinanceTabs /><Panel title="新增账户" eyebrow="ACCOUNTS"><form className="inline-form" onSubmit={(event) => void submit(event)}><input required value={name} onChange={(event) => setName(event.target.value)} placeholder="例如：微信零钱" /><button className="primary-button" disabled={!props.online}>保存到云端</button></form></Panel><div className="data-grid">{accounts.map((item) => <article className="data-card" key={item.meta.id}><span className="account-dot" /><h3>{text(item, "name")}</h3><p>{text(item, "accountType")} · {text(item, "currency")}</p><strong>{formatMoney(number(item, "openingBalanceCents"), text(item, "currency") || "CNY", props.privacy)}</strong><div className="card-actions"><button onClick={() => { const next = window.prompt("账户名称", text(item, "name")); if (next?.trim()) void props.run((store) => store.upsert("finance.account", { ...item, name: next.trim(), meta: { ...item.meta } })); }}>重命名</button><button className="danger" onClick={() => void props.run((store) => store.delete("finance.account", item.meta.id))}>删除</button></div></article>)}{!accounts.length && <Empty title="暂无账户" description="创建一个资金账户用于归集账单。" />}</div></PageStack>;
}

export function CategoriesPage(props: CloudPageProps) {
  const categories = entities(props.state, "finance.category");
  const [name, setName] = useState(""); const [type, setType] = useState<"expense" | "income">("expense");
  async function submit(event: FormEvent) { event.preventDefault(); await props.run((store) => store.upsert("finance.category", createFinanceCategory(props.session.user.id, props.session.session.deviceId, name, type))); setName(""); }
  return <PageStack><FinanceTabs /><Panel title="新增分类" eyebrow="CATEGORIES"><form className="inline-form" onSubmit={(event) => void submit(event)}><select value={type} onChange={(event) => setType(event.target.value as "expense" | "income")}><option value="expense">支出</option><option value="income">收入</option></select><input required value={name} onChange={(event) => setName(event.target.value)} placeholder="餐饮、交通、工资…" /><button className="primary-button" disabled={!props.online}>保存</button></form></Panel><div className="data-grid">{categories.map((item) => <article className="data-card" key={item.meta.id}><span className={`status-pill ${text(item, "categoryType")}`}>{text(item, "categoryType")}</span><h3>{text(item, "name")}</h3><p>{item.isSystem === true ? "系统分类" : "自定义分类"}</p>{item.isSystem !== true && <button className="danger small-button" onClick={() => void props.run((store) => store.delete("finance.category", item.meta.id))}>删除</button>}</article>)}</div></PageStack>;
}

export function BudgetsPage(props: CloudPageProps) {
  const month = new Date().toISOString().slice(0, 7);
  const [budgetMonth, setBudgetMonth] = useState(month); const [amount, setAmount] = useState("");
  const preferences = entities(props.state, "user.preference").filter((item) => text(item, "preferenceKey").startsWith("finance.budget."));
  const transactions = entities(props.state, "finance.transaction");
  async function submit(event: FormEvent) { event.preventDefault(); await props.run((store) => store.upsert("user.preference", createBudgetPreference(props.session.user.id, props.session.session.deviceId, budgetMonth, amount))); setAmount(""); }
  return <PageStack><FinanceTabs /><Panel title="设置月度预算" eyebrow="BUDGET"><form className="inline-form" onSubmit={(event) => void submit(event)}><input type="month" value={budgetMonth} onChange={(event) => setBudgetMonth(event.target.value)} required /><input inputMode="decimal" value={amount} onChange={(event) => setAmount(event.target.value)} placeholder="预算金额" required /><button className="primary-button" disabled={!props.online}>保存</button></form></Panel><div className="data-grid">{preferences.map((item) => { const value = item.value && typeof item.value === "object" ? item.value as Record<string, unknown> : {}; const target = Number(value.amountCents ?? 0); const used = monthExpense(transactions, String(value.month ?? "")); const ratio = target ? Math.min(1, used / target) : 0; return <article className="data-card" key={item.meta.id}><span>{String(value.month ?? "")}</span><h3>{formatMoney(target, "CNY", props.privacy)}</h3><p>已使用 {formatMoney(used, "CNY", props.privacy)}</p><progress max={1} value={ratio} /><button className="danger small-button" onClick={() => void props.run((store) => store.delete("user.preference", item.meta.id))}>删除</button></article>; })}{!preferences.length && <Empty title="暂无预算" description="设置预算后可查看月度使用进度。" />}</div></PageStack>;
}

export function ImportPage(props: CloudPageProps) {
  const accounts = entities(props.state, "finance.account");
  const existing = entities(props.state, "finance.transaction");
  const [accountId, setAccountId] = useState("");
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [fileName, setFileName] = useState("");
  const [result, setResult] = useState("");
  async function choose(file?: File) {
    if (!file) return;
    setFileName(file.name); setResult("");
    setPreview(await importBillFile(props.session.user.id, props.session.session.deviceId, file, accountId || null));
  }
  async function upload() {
    if (!preview) return;
    const deduplicated = preview.rows.filter((item) => !findProbableDuplicate(item, existing));
    const skipped = preview.rows.length - deduplicated.length;
    let saved = 0; let errors: string[] = [];
    await props.run(async (store) => { const response = await store.batchUpsert("finance.transaction", deduplicated); saved = response.saved; errors = response.errors; return response.state; });
    setResult(`云端已保存 ${saved} 条，跳过疑似重复 ${skipped} 条${errors.length ? `，失败 ${errors.length} 条` : ""}`);
  }
  return <PageStack><FinanceTabs /><Panel title="导入账单文件" eyebrow="CSV / XLSX"><div className="form-grid"><label>归属账户<select value={accountId} onChange={(event) => setAccountId(event.target.value)}><option value="">未指定</option>{accounts.map((item) => <option key={item.meta.id} value={item.meta.id}>{text(item, "name")}</option>)}</select></label><label>账单文件<input type="file" accept=".csv,.xlsx,.xls" onChange={(event) => void choose(event.target.files?.[0])} /></label></div>{fileName && <p className="muted">已选择：{fileName}</p>}{preview && <><div className="metric-grid compact-grid"><Metric label="可解析" value={String(preview.rows.length)} detail={preview.sourceType} /><Metric label="警告" value={String(preview.warnings.length)} detail="不会上传失败行" /></div>{preview.warnings.length > 0 && <Notice kind="warning">{preview.warnings.slice(0, 5).join("；")}</Notice>}<button className="primary-button" disabled={!props.online || !preview.rows.length} onClick={() => void upload()}>确认并上传云端</button></>}{result && <Notice kind="neutral">{result}</Notice>}</Panel><Panel title="对账规则" eyebrow="RECONCILIATION"><p className="muted">导入记录先标记为 candidate。交易单号相同，或同日、同金额、同商户的记录会被判定为疑似重复并跳过；其余记录可在账单页确认或忽略。</p></Panel></PageStack>;
}
