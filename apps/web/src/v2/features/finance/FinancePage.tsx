import { useState, type FormEvent } from "react";
import { Plus } from "lucide-react";
import { Button, Card, EmptyState, Input, Select, StatCard, Table, Textarea } from "../../design-system/ui";
import { financeSummary, isoDate, money, newId, type FinanceTransaction, type LifeTraceState } from "../../model";
import { PageHeader, Section, type SetLifeTraceState } from "../shared";

export const financeRoutes = [
  "/app/finance",
  "/app/finance/transactions",
  "/app/finance/calendar",
  "/app/finance/ledgers",
  "/app/finance/budgets",
  "/app/finance/accounts",
  "/app/finance/categories",
  "/app/finance/tags",
  "/app/finance/import"
] as const;

const financeTabs = [
  ["Overview", "/app/finance"], ["Transactions", "/app/finance/transactions"], ["Calendar", "/app/finance/calendar"], ["Ledgers", "/app/finance/ledgers"], ["Budgets", "/app/finance/budgets"], ["Accounts", "/app/finance/accounts"], ["Categories", "/app/finance/categories"], ["Tags", "/app/finance/tags"], ["Import", "/app/finance/import"]
] as const;

export function FinancePage({ state, setState, path, navigate }: { state: LifeTraceState; setState: SetLifeTraceState; path: string; navigate: (path: string) => void }) {
  const summary = financeSummary(state.transactions);
  const [title, setTitle] = useState(""); const [amount, setAmount] = useState(""); const [direction, setDirection] = useState<"expense" | "income">("expense"); const [csv, setCsv] = useState("");
  const add = (event: FormEvent) => { event.preventDefault(); const cents = Math.round(Number(amount) * 100); if (!title.trim() || !Number.isFinite(cents)) return; const item: FinanceTransaction = { id: newId("tx"), date: isoDate(), title: title.trim(), category: direction === "expense" ? "日常" : "收入", account: "默认账户", amountCents: Math.abs(cents), direction }; setState((current) => ({ ...current, transactions: [item, ...current.transactions] })); setTitle(""); setAmount(""); };
  const importCsv = () => { const parsed = csv.split(/\r?\n/).map((line) => line.trim()).filter(Boolean).map((line) => line.split(",")).filter((parts) => parts.length >= 3).map((parts) => ({ id: newId("tx"), date: parts[0] || isoDate(), title: parts[1] || "导入记录", category: parts[3] || "导入", account: parts[4] || "默认账户", amountCents: Math.round(Math.abs(Number(parts[2]) || 0) * 100), direction: Number(parts[2]) >= 0 ? "income" as const : "expense" as const })); if (parsed.length) setState((current) => ({ ...current, transactions: [...parsed, ...current.transactions] })); setCsv(""); };
  const placeholder = path !== "/app/finance" && path !== "/app/finance/transactions" && !path.endsWith("/import");
  return <><PageHeader title="Finance" detail="BeeCount 成熟账本心智模型适配到 LifeTrace V2 Token、Shell 与同步协议。" /><div style={{ overflowX: "auto", paddingBottom: 4 }}><div className="lt-segmented">{financeTabs.map(([label, tabPath]) => <button key={tabPath} className={path === tabPath ? "is-active" : ""} onClick={() => navigate(tabPath)}>{label}</button>)}</div></div><div className="lt-metrics lt-section"><StatCard label="Balance" value={money(summary.balance)} /><StatCard label="Income" value={money(summary.income)} /><StatCard label="Expense" value={money(summary.expense)} /></div>{path.endsWith("/import") ? <Section title="Import transactions"><Card><p className="lt-muted">CSV：日期,标题,金额,分类,账户。正数收入，负数支出。</p><Textarea value={csv} onChange={(event) => setCsv(event.target.value)} placeholder="2026-08-21,午餐,-35.5,餐饮,支付宝" /><Button onClick={importCsv} disabled={!csv.trim()} style={{ marginTop: 12 }}>导入</Button></Card></Section> : placeholder ? <Section title={financeTabs.find(([, tabPath]) => tabPath === path)?.[0] ?? "Finance workspace"}><Card><p className="lt-muted">该账本子工作区复用统一 Finance 数据层与 BeeCount 路由模型；当前摘要与交易数据来自 LifeTrace Cloud 同步实体。</p></Card></Section> : <><Section title="Quick record"><form className="lt-form-grid two" onSubmit={add}><Input value={title} onChange={(event) => setTitle(event.target.value)} placeholder="交易说明" /><Input value={amount} onChange={(event) => setAmount(event.target.value)} inputMode="decimal" placeholder="金额" /><Select value={direction} onChange={(event) => setDirection(event.target.value as "expense" | "income")}><option value="expense">Expense</option><option value="income">Income</option></Select><Button type="submit"><Plus size={17} />记录</Button></form></Section><Section title="Transactions">{state.transactions.length ? <Table><thead><tr><th>日期</th><th>说明</th><th>分类</th><th>账户</th><th>金额</th></tr></thead><tbody>{state.transactions.map((item) => <tr key={item.id}><td>{item.date}</td><td>{item.title}</td><td>{item.category}</td><td>{item.account}</td><td style={{ color: item.direction === "income" ? "var(--success)" : "var(--text-primary)" }}>{item.direction === "income" ? "+" : "−"}{money(item.amountCents)}</td></tr>)}</tbody></Table> : <EmptyState title="还没有账目" detail="从一条真实记录开始。" />}</Section></>}</>;
}
