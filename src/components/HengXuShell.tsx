"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import {
  Activity as ActivityIcon, Archive, BarChart3, BookOpen, CalendarDays, Check, ChevronRight,
  CircleDollarSign, Download, Dumbbell, FileUp, Home, Menu, Pencil, Plus,
  Languages, NotebookPen, Settings, Shield, Trash2, UserRound, WalletCards, X,
} from "lucide-react";
import { useLifeStore } from "@/src/stores/useLifeStore";
import type { Activity, ActivityLog, FinanceAccount, Transaction, WorkoutHistory } from "@/src/types";
import DailyEnglish from "@/src/components/english/DailyEnglish";
import XunjiImportPanel from "@/src/components/XunjiImportPanel";
import NotesModule, { DashboardNotes } from "@/src/components/NotesModule";
import { noteApi } from "@/src/services/noteApi";

type PlatformView = "dashboard" | "habits" | "english" | "fitness" | "finance" | "transactions" | "accounts" | "import" | "calendar" | "review" | "notes" | "settings";
type Modal = null | { kind: "activity"; value?: Activity } | { kind: "record"; value: Activity } | { kind: "transaction"; value?: Transaction } | { kind: "account"; value?: FinanceAccount };
type ImportUploadItem = { id:string;kind:"fitness"|"bill";filename:string;contentType:string;size:number;status:"pending"|"parsed";createdAt:string };

const pad = (value: number) => String(value).padStart(2, "0");
const dayKey = (date = new Date()) => `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
const money = (value: number) => `¥${value.toLocaleString("zh-CN", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
const dateTimeLocal = (value?: string) => {
  const date = value ? new Date(value) : new Date();
  const local = new Date(date.getTime() - date.getTimezoneOffset() * 60000);
  return local.toISOString().slice(0, 16);
};
const notify = (message: string) => window.dispatchEvent(new CustomEvent("hengxu-toast", { detail: message }));
const escapeHtml = (value:string) => value.replace(/[&<>"]/g, character => ({ "&":"&amp;", "<":"&lt;", ">":"&gt;", '"':"&quot;" })[character]!);

const navGroups: { label: string; items: { id: PlatformView; label: string; icon: typeof Home }[] }[] = [
  { label: "工作台", items: [{ id: "dashboard", label: "总览", icon: Home }] },
  { label: "成长与健康", items: [{ id: "habits", label: "坚持项目", icon: Check }, { id: "english", label: "每日英语", icon: Languages }, { id: "fitness", label: "健身训练", icon: Dumbbell }, { id: "notes", label: "笔记", icon: NotebookPen }, { id: "calendar", label: "生活日历", icon: CalendarDays }, { id: "review", label: "每日复盘", icon: BookOpen }] },
  { label: "资产与账单", items: [{ id: "finance", label: "财务概览", icon: BarChart3 }, { id: "transactions", label: "账单管理", icon: CircleDollarSign }, { id: "accounts", label: "账户管理", icon: WalletCards }, { id: "import", label: "账单导入", icon: FileUp }] },
];

const pageCopy: Record<PlatformView, [string, string]> = {
  dashboard: ["个人总览", "把坚持、训练、财务和复盘放在同一个日常系统里。"],
  habits: ["坚持项目", "管理长期项目，关注完成率、总量与真实趋势。"],
  english: ["每日英语", "阅读、英文总结、AI 反馈与长期能力成长。"],
  fitness: ["健身数据", "导入训练截图，在电脑端统一解析并沉淀训练记录。"],
  finance: ["财务概览", "看清资产、收支和消费结构，不制造额外焦虑。"],
  transactions: ["账单管理", "搜索、筛选、编辑并维护全部收支记录。"],
  accounts: ["账户管理", "集中维护银行卡、电子钱包、投资账户和现金。"],
  import: ["账单导入", "从 CSV 文件批量导入账单，数据直接进入 SQLite。"],
  calendar: ["生活日历", "坚持、账单和复盘都落在具体的一天里。"],
  review: ["每日复盘", "每天两分钟，看清今天并为明天留一个重点。"],
  notes: ["笔记", "记录想法、复盘与知识，并与坚持、训练和账单建立联系。"],
  settings: ["数据与设置", "管理 SQLite 数据、备份、恢复和外观。"],
};

function Metric({ label, value, sub, positive }: { label: string; value: string; sub: string; positive?: boolean }) {
  return <div className="hx-metric"><span>{label}</span><strong>{value}</strong><small className={positive ? "positive" : ""}>{sub}</small></div>;
}

function Dashboard({ go, record }: { go: (view: PlatformView) => void; record: (value: Activity) => void }) {
  const { activities, logs, transactions, accounts, workoutHistory } = useLifeStore();
  const today = dayKey();
  const todayLogs = logs.filter((item) => item.createdAt.startsWith(today));
  const done = activities.filter((item) => todayLogs.some((log) => log.activityId === item.id && log.status !== "skipped")).length;
  const month = today.slice(0, 7);
  const monthExpense = transactions.filter((item) => item.type === "expense" && item.occurredAt.startsWith(month)).reduce((sum, item) => sum + item.amount, 0);
  const assets = accounts.reduce((sum, item) => sum + (item.balance ?? 0), 0);
  const days = Array.from({ length: 7 }, (_, index) => { const date = new Date(); date.setDate(date.getDate() - (6 - index)); return date; });
  const spend = days.map((date) => transactions.filter((item) => item.type === "expense" && item.occurredAt.startsWith(dayKey(date))).reduce((sum, item) => sum + item.amount, 0));
  const max = Math.max(...spend, 1);
  const recentWorkout = workoutHistory[0];

  return <div className="hx-view">
    <div className="hx-hero-grid">
      <article className="hx-hero-dark"><span className="hx-pill">今日</span><h2>今天，从最重要的一小步开始。</h2><p>你的坚持、训练与消费记录会在这里形成统一反馈。</p><div className="hx-hero-progress"><div><strong>{done} / {activities.length}</strong><small>今日坚持</small></div><i><b style={{ width: `${activities.length ? done / activities.length * 100 : 0}%` }} /></i></div></article>
      <article className="hx-quote"><span>“</span><h3>不要打断两次。</h3><p>允许偶尔错过，但下一次按计划回来。</p></article>
    </div>
    <div className="hx-metrics"><Metric label="今日完成" value={`${done} 项`} sub={`还有 ${Math.max(activities.length - done, 0)} 项等待完成`} /><Metric label="本周训练" value={`${workoutHistory.filter(item => Date.now() - new Date(item.occurredAt).getTime() < 7 * 86400000).length} 次`} sub="训练完成后自动同步坚持项目" positive /><Metric label="本月支出" value={money(monthExpense)} sub={`${transactions.filter(item => item.occurredAt.startsWith(month)).length} 笔收支记录`} /><Metric label="当前总资产" value={money(assets)} sub={`${accounts.length} 个账户`} positive /></div>
    <div className="hx-dashboard-grid">
      <article className="hx-panel"><PanelHead kicker="今日" title="今天的坚持" action="管理项目" onClick={() => go("habits")} /><div className="hx-panel-body hx-list">{activities.slice(0, 5).map((activity) => { const own = todayLogs.filter(log => log.activityId === activity.id); const value = own.reduce((sum, log) => log.status==="skipped"?sum:sum+(log.value??1), 0); return <div className="hx-row" key={activity.id}><span className="hx-row-icon">{activity.name.slice(0, 1)}</span><div><strong>{activity.name}</strong><small>{own.length ? `已记录 ${value} ${activity.unit}` : `${activity.targetPeriod === "weekly" ? "每周" : "每天"} · 目标 ${activity.normalTarget ?? 1} ${activity.unit}`}</small></div><button className={value>0 ? "done" : ""} onClick={() => record(activity)}>{value>0 ? "继续记录" : "记录"}</button></div>})}</div></article>
      <article className="hx-panel"><PanelHead kicker="财务" title="近 7 天支出" action="查看分析" onClick={() => go("finance")} /><div className="hx-panel-body"><div className="hx-bars">{spend.map((value, index) => <div key={days[index].toISOString()}><i style={{ height: `${Math.max(value / max * 100, value ? 8 : 2)}%` }} /><small>{pad(days[index].getMonth() + 1)}-{pad(days[index].getDate())}</small></div>)}</div></div></article>
      <article className="hx-panel"><PanelHead kicker="训练" title="最近训练" action="查看训练数据" onClick={() => go("fitness")} /><div className="hx-panel-body">{recentWorkout ? <div className="hx-row"><span className="hx-row-icon">训</span><div><strong>{recentWorkout.name}</strong><small>{recentWorkout.exerciseCount} 个动作 · {recentWorkout.setCount} 组 · {new Date(recentWorkout.occurredAt).toLocaleDateString("zh-CN")}</small></div><button onClick={() => go("fitness")}>查看</button></div> : <Empty text="导入训练截图后，这里会显示最近一次训练。"/>}</div></article>
      <article className="hx-panel"><PanelHead kicker="最近" title="最近账单" action="全部账单" onClick={() => go("transactions")} /><div className="hx-panel-body hx-list">{transactions.slice(0, 5).map(item => <div className="hx-row" key={item.id}><span className="hx-row-icon">{(item.counterparty || item.category).slice(0, 1)}</span><div><strong>{item.counterparty || item.category}</strong><small>{item.category} · {new Date(item.occurredAt).toLocaleDateString("zh-CN")}</small></div><b className={item.type}>{item.type === "expense" ? "-" : "+"}{money(item.amount)}</b></div>)}</div></article>
      <DashboardNotes openNotes={()=>go("notes")}/>
    </div>
  </div>;
}

function PanelHead({ kicker, title, action, onClick }: { kicker: string; title: string; action?: string; onClick?: () => void }) {
  return <header className="hx-panel-head"><div><span>{kicker}</span><h2>{title}</h2></div>{action && <button onClick={onClick}>{action} <ChevronRight /></button>}</header>;
}

function Empty({ text }: { text: string }) { return <div className="hx-empty"><span>—</span><p>{text}</p></div>; }

function Habits({ edit, record, note }: { edit: (value?: Activity) => void; record: (value: Activity) => void; note:(value:Activity)=>void }) {
  const { activities, logs, archiveActivity } = useLifeStore();
  const [filter, setFilter] = useState<"all" | "pending" | "done">("all");
  const today = dayKey();
  const shown = activities.filter(item => filter === "all" || (filter === "done") === logs.some(log => log.activityId === item.id && log.createdAt.startsWith(today) && log.status !== "skipped"));
  return <div className="hx-view"><div className="hx-toolbar"><div className="hx-segmented">{[["all","全部"],["pending","待完成"],["done","已完成"]].map(([id,label]) => <button key={id} className={filter === id ? "active" : ""} onClick={() => setFilter(id as typeof filter)}>{label}</button>)}</div><button className="hx-btn primary" onClick={() => edit()}><Plus /> 创建坚持项目</button></div><div className="hx-card-grid">{shown.map(item => { const itemLogs = logs.filter(log => log.activityId === item.id); const todayValue = itemLogs.filter(log => log.createdAt.startsWith(today)).reduce((sum, log) => log.status==="skipped"?sum:sum+(log.value??1), 0); const target = item.normalTarget ?? 1; return <article className="hx-habit-card" key={item.id}><div className="hx-card-actions"><span>{item.name.slice(0, 2)}</span><div><button onClick={() => edit(item)} aria-label={`编辑${item.name}`}><Pencil /></button><button onClick={() => archiveActivity(item.id)} aria-label={`归档${item.name}`}><Archive /></button></div></div><h3>{item.name}</h3><p>{item.description || "保持稳定节奏，关注长期积累。"}</p><div className="hx-progress-label"><span>今日进度</span><b>{todayValue} / {target} {item.unit}</b></div><i className="hx-track"><b style={{ width: `${Math.min(100, todayValue / target * 100)}%` }} /></i><footer><small>累计 {itemLogs.reduce((sum, log) => log.status==="skipped"?sum:sum+(log.value??1), 0)} {item.unit}</small><div><button className="note" onClick={()=>note(item)}><NotebookPen/>记录</button><button className={todayValue >= target ? "done" : ""} onClick={() => record(item)}>{todayValue >= target ? "继续记录" : "打卡"}</button></div></footer></article>})}</div><HabitAnalytics activities={activities} logs={logs}/></div>;
}

function HabitAnalytics({activities,logs}:{activities:Activity[];logs:ActivityLog[]}) {
  const [activityId,setActivityId]=useState(activities[0]?.id??"");
  const activity=activities.find(item=>item.id===activityId)??activities[0];
  const days=Array.from({length:84},(_,index)=>{const date=new Date();date.setHours(12,0,0,0);date.setDate(date.getDate()-(83-index));return date});
  const values=days.map(date=>logs.filter(item=>item.activityId===activity?.id&&item.createdAt.startsWith(dayKey(date))).reduce((sum,item)=>item.status==="skipped"?sum:sum+(item.value??1),0));
  const activeDays=values.filter(value=>value>0).length;const total=values.reduce((sum,value)=>sum+value,0);const rate=Math.round(activeDays/84*100);
  return <div className="hx-analytics"><article className="hx-panel"><PanelHead kicker="坚持趋势" title="过去 12 周坚持轨迹"/><div className="hx-panel-body"><select value={activity?.id??""} onChange={e=>setActivityId(e.target.value)}>{activities.map(item=><option key={item.id} value={item.id}>{item.name}</option>)}</select><div className="hx-heatmap">{values.map((value,index)=><i key={days[index].toISOString()} className={value<=0?"":value<(activity?.normalTarget??1)*.5?"l1":value<(activity?.normalTarget??1)?"l2":value<(activity?.normalTarget??1)*1.5?"l3":"l4"} title={`${dayKey(days[index])} · ${value} ${activity?.unit??""}`}/>)}</div><div className="hx-heat-legend"><span>少</span><i/><i className="l2"/><i className="l3"/><i className="l4"/><span>多</span></div></div></article><article className="hx-panel hx-insight"><span className="hx-kicker">数据洞察</span><div className="hx-ring" style={{"--p":`${rate}%`} as React.CSSProperties}><strong>{rate}%</strong></div><h3>{activity?.name??"坚持项目"}</h3><p>过去 12 周有 {activeDays} 天留下记录。稳定回来，比追求完美连续更重要。</p><div><span><b>{total}</b>累计 {activity?.unit}</span><span><b>{activeDays}</b>活跃天数</span></div></article></div>;
}

function Fitness({note}:{note:(value:WorkoutHistory)=>void}) {
  const { workoutHistory } = useLifeStore();
  const weekCount = workoutHistory.filter(item => Date.now() - new Date(item.occurredAt).getTime() < 7 * 86400000).length;
  return <div className="hx-view"><article className="hx-fitness-hero"><div><span className="hx-pill">训练数据中心</span><h2>导入训练截图，自动整理训练记录</h2><p>电脑端负责解析训练数据并长期保存，已有训练历史会继续保留。</p></div><div><span>本周训练</span><strong>{weekCount} / 4</strong><i className="hx-track"><b style={{ width: `${Math.min(100, weekCount / 4 * 100)}%` }} /></i></div></article><XunjiImportPanel/><article className="hx-panel hx-history"><PanelHead kicker="训练记录" title="训练历史" /><div>{workoutHistory.slice(0, 10).map(item => <div className="hx-history-row" key={item.id}><time>{new Date(item.occurredAt).toLocaleDateString("zh-CN", { month: "2-digit", day: "2-digit" })}</time><div><strong>{item.name}</strong><small>{item.exerciseCount} 个动作 · {item.setCount} 组 · {Math.max(1, Math.round(item.durationSeconds / 60))} 分钟</small></div><button className="hx-btn secondary" onClick={()=>note(item)}><NotebookPen/>训练复盘</button></div>)}{!workoutHistory.length&&<Empty text="导入训练截图后，解析结果会出现在这里。"/>}</div></article></div>;
}

function Finance() {
  const { transactions, accounts } = useLifeStore();
  const month = dayKey().slice(0, 7);
  const current = transactions.filter(item => item.occurredAt.startsWith(month));
  const expense = current.filter(item => item.type === "expense").reduce((sum, item) => sum + item.amount, 0);
  const income = current.filter(item => item.type === "income").reduce((sum, item) => sum + item.amount, 0);
  const categories = Object.entries(current.filter(item => item.type === "expense").reduce<Record<string, number>>((all, item) => ({ ...all, [item.category]: (all[item.category] ?? 0) + item.amount }), {})).sort((a,b) => b[1] - a[1]).slice(0, 7);
  const max = Math.max(...categories.map(item => item[1]), 1);
  return <div className="hx-view"><div className="hx-metrics"><Metric label="总资产" value={money(accounts.reduce((sum,item)=>sum+(item.balance??0),0))} sub={`${accounts.length} 个账户`} positive/><Metric label="本月收入" value={money(income)} sub={`${current.filter(item=>item.type==="income").length} 笔收入`} positive/><Metric label="本月支出" value={money(expense)} sub={`${current.filter(item=>item.type==="expense").length} 笔支出`}/><Metric label="本月结余" value={money(income-expense)} sub={income ? `储蓄率 ${Math.round((income-expense)/income*100)}%` : "等待收入数据"}/></div><div className="hx-finance-grid"><article className="hx-panel"><PanelHead kicker="CATEGORY" title="支出分类" /><div className="hx-panel-body hx-category-list">{categories.length ? categories.map(([name,value]) => <div key={name}><span>{name}</span><i><b style={{width:`${value/max*100}%`}}/></i><strong>{money(value)}</strong></div>) : <Empty text="记录支出后显示分类结构。"/>}</div></article><article className="hx-panel"><PanelHead kicker="ACCOUNTS" title="资产账户" /><div className="hx-panel-body hx-account-mini">{accounts.map(item => <div key={item.id}><i style={{background:item.color}}>{item.icon}</i><span><strong>{item.name}</strong><small>{item.type}</small></span><b>{item.balance === null ? "未设置" : money(item.balance)}</b></div>)}</div></article></div></div>;
}

function Transactions({ edit, note }: { edit: (value?: Transaction) => void; note:(value:Transaction)=>void }) {
  const { transactions, deleteTransaction } = useLifeStore();
  const [search,setSearch]=useState(""); const [direction,setDirection]=useState<"all"|"expense"|"income">("all");
  const rows=transactions
    .filter(item => (direction==="all"||item.type===direction) && `${item.counterparty??""}${item.category}${item.note??""}`.toLowerCase().includes(search.toLowerCase()))
    .sort((left,right) => new Date(right.occurredAt).getTime()-new Date(left.occurredAt).getTime());
  return <div className="hx-view"><div className="hx-toolbar hx-tx-tools"><input value={search} onChange={e=>setSearch(e.target.value)} placeholder="搜索交易对象、分类或备注"/><select value={direction} onChange={e=>setDirection(e.target.value as typeof direction)}><option value="all">全部收支</option><option value="expense">支出</option><option value="income">收入</option></select><button className="hx-btn primary" onClick={()=>edit()}><Plus/> 手动记账</button></div><article className="hx-panel hx-table-wrap"><table><thead><tr><th>时间</th><th>交易</th><th>分类</th><th>账户</th><th>类型</th><th>金额</th><th>操作</th></tr></thead><tbody>{rows.map(item=><tr key={item.id}><td>{new Date(item.occurredAt).toLocaleString("zh-CN",{month:"2-digit",day:"2-digit",hour:"2-digit",minute:"2-digit"})}</td><td><strong>{item.counterparty||item.category}</strong><small>{item.item||item.note||"手动记录"}</small></td><td><span className="hx-tag">{item.category}</span></td><td>{item.account}</td><td>{item.type==="expense"?"支出":"收入"}</td><td className={item.type}>{item.type==="expense"?"-":"+"}{money(item.amount)}</td><td><button title="消费笔记" onClick={()=>note(item)}><NotebookPen/></button><button onClick={()=>edit(item)}><Pencil/></button><button onClick={()=>deleteTransaction(item.id)}><Trash2/></button></td></tr>)}</tbody></table><footer>共 {rows.length} 笔记录</footer></article></div>;
}

function Accounts({ edit }: { edit: (value?: FinanceAccount) => void }) {
  const { accounts, transactions, deleteAccount } = useLifeStore();
  return <div className="hx-view"><div className="hx-toolbar"><span className="hx-tag">账户余额手动维护；记账不会直接改写余额</span><button className="hx-btn primary" onClick={()=>edit()}><Plus/> 添加账户</button></div><div className="hx-card-grid">{accounts.map(account=>{const rows=transactions.filter(item=>item.accountId===account.id||item.account===account.name);const income=rows.filter(item=>item.type==="income").reduce((s,i)=>s+i.amount,0);const expense=rows.filter(item=>item.type==="expense").reduce((s,i)=>s+i.amount,0);return <article className="hx-account-card" key={account.id}><header><i style={{background:account.color}}>{account.icon}</i><div><button onClick={()=>edit(account)}><Pencil/></button><button onClick={()=>deleteAccount(account.id)}><Trash2/></button></div></header><h3>{account.name}</h3><p>{account.type}{account.last4?` · 尾号 ${account.last4}`:""}</p><strong>{account.balance===null?"未设置":money(account.balance)}</strong><div><span>累计收入 <b>{money(income)}</b></span><span>累计支出 <b>{money(expense)}</b></span></div></article>})}</div></div>;
}

function ImportBills() {
  const { accounts, transactions, addTransaction } = useLifeStore();
  type ImportRow=Parameters<typeof addTransaction>[0]&{sourceId?:string};
  const input=useRef<HTMLInputElement>(null);
  const [rows,setRows]=useState<ImportRow[]>([]);
  const [message,setMessage]=useState("");
  const [summary,setSummary]=useState({source:0,neutral:0,duplicates:0,invalid:0});
  const [importing,setImporting]=useState(false);
  const [phoneUploads,setPhoneUploads]=useState<ImportUploadItem[]>([]);
  const [loadingUploads,setLoadingUploads]=useState(true);
  const loadPhoneUploads=async()=>{setLoadingUploads(true);try{const response=await fetch("/api/imports");const payload=await response.json() as {items?:ImportUploadItem[]};setPhoneUploads(payload.items??[])}finally{setLoadingUploads(false)}};
  useEffect(()=>{void loadPhoneUploads()},[]);
  const parseLine=(line:string)=>{const result:string[]=[];let cell="",quoted=false;for(let i=0;i<line.length;i++){const c=line[i];if(c==='"'&&line[i+1]==='"'){cell+='"';i++}else if(c==='"')quoted=!quoted;else if(c===","&&!quoted){result.push(cell);cell=""}else cell+=c}result.push(cell);return result};
  const cellText=(value:unknown)=>value instanceof Date?`${value.getFullYear()}-${pad(value.getMonth()+1)}-${pad(value.getDate())} ${pad(value.getHours())}:${pad(value.getMinutes())}:${pad(value.getSeconds())}`:String(value??"").trim();
  const inferCategory=(type:"income"|"expense",transactionType:string,counterparty:string,item:string)=>{
    const text=`${transactionType} ${counterparty} ${item}`;
    if(type==="income")return /退款|退还|退回/.test(text)?"退款":"其他收入";
    if(/餐饮|餐厅|饭|面|粉|水煮鱼|豆制品|咖啡|茶|奶茶|麦当劳|肯德基|食堂|亚惠/.test(text))return"餐饮";
    if(/地铁|公交|打车|滴滴|铁路|航空|加油|停车|充电/.test(text))return"交通";
    if(/拼多多|淘宝|京东|商户消费|超市|便利店|眼镜|百货/.test(text))return"购物";
    if(/医院|药房|诊所|医疗|体检/.test(text))return"医疗健康";
    if(/话费|电费|水费|燃气|宽带|物业/.test(text))return"生活缴费";
    if(/红包|群收款|转账|二维码付款|扫二维码/.test(text))return"转账与人情";
    return"日常消费";
  };
  const read=async(file:File)=>{
    try{
      setRows([]);setMessage("正在解析账单…");
      let matrix:unknown[][];
      if(file.name.toLowerCase().endsWith(".xlsx")){
        const {readSheet}=await import("read-excel-file/browser");
        matrix=await readSheet(file);
      }else{
        const lines=(await file.text()).replace(/^\uFEFF/,"").split(/\r?\n/).filter(Boolean);
        matrix=lines.map(parseLine);
      }
      const headerRow=matrix.findIndex(row=>{const text=row.map(cellText);return text.some(item=>item.includes("交易时间")||item.toLowerCase().includes("date"))&&text.some(item=>item.includes("金额")||item.toLowerCase().includes("amount"))});
      if(headerRow<0)throw new Error("没有找到微信账单明细表头，请确认文件是微信支付导出的账单");
      const headers=matrix[headerRow].map(value=>cellText(value).replace(/\s/g,""));
      const index=(...names:string[])=>headers.findIndex(header=>names.some(name=>header.toLowerCase().includes(name.toLowerCase())));
      const dateIndex=index("交易时间","时间","日期","date");
      const transactionTypeIndex=index("交易类型");
      const counterpartyIndex=index("交易对方","交易对象","商户","counterparty");
      const itemIndex=index("商品","说明","item");
      const directionIndex=index("收/支","收支","direction");
      const amountIndex=index("金额","amount");
      const accountIndex=index("支付方式","账户","account");
      const statusIndex=index("当前状态","状态");
      const sourceIdIndex=index("交易单号");
      const categoryIndex=index("分类","category");
      if(dateIndex<0||amountIndex<0||directionIndex<0)throw new Error("账单缺少交易时间、收/支或金额列");
      const existingIds=new Set(transactions.flatMap(item=>{const match=item.note?.match(/微信交易单号：([^\s·]+)/);return match?[match[1]]:[]}));
      let neutral=0,duplicates=0,invalid=0;
      const parsed:ImportRow[]=[];
      for(const rawRow of matrix.slice(headerRow+1)){
        const cells=rawRow.map(cellText);
        if(cells.every(cell=>!cell))continue;
        const direction=cells[directionIndex]??"";
        if(/中性|\/|不计收支/.test(direction)){neutral++;continue}
        const type=/收入|income|入账/i.test(direction)?"income" as const:/支出|expense/i.test(direction)?"expense" as const:null;
        const amount=Math.abs(Number((cells[amountIndex]??"").replace(/[¥￥,\s]/g,"")));
        const dateText=cells[dateIndex]??"";
        const parsedDate=/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/.test(dateText)?new Date(`${dateText.replace(" ","T")}+08:00`):new Date(dateText);
        if(!type||!Number.isFinite(amount)||amount<=0||!dateText||Number.isNaN(parsedDate.getTime())){invalid++;continue}
        const occurredAt=parsedDate.toISOString();
        const sourceId=(cells[sourceIdIndex]??"").trim();
        if(sourceId&&existingIds.has(sourceId)){duplicates++;continue}
        const rawAccount=(cells[accountIndex]??"").replace(/^\/$/,"").trim();
        const account=accounts.find(candidate=>candidate.name===rawAccount||(candidate.last4&&rawAccount.includes(candidate.last4))||(candidate.type==="wechat"&&rawAccount.includes("零钱"))||(candidate.type==="bank"&&/银行|信用卡/.test(rawAccount)));
        const transactionType=cells[transactionTypeIndex]??"";
        const counterparty=cells[counterpartyIndex]||"微信支付";
        const item=(cells[itemIndex]??"").replace(/^\/$/,"");
        const status=(cells[statusIndex]??"").replace(/^\/$/,"");
        parsed.push({type,amount,category:cells[categoryIndex]||inferCategory(type,transactionType,counterparty,item),account:account?.name||rawAccount||"微信支付",accountId:account?.id,counterparty,item,occurredAt,note:[transactionType,status,sourceId?`微信交易单号：${sourceId}`:""].filter(Boolean).join(" · "),sourceId});
        if(sourceId)existingIds.add(sourceId);
      }
      setRows(parsed);setSummary({source:matrix.length-headerRow-1,neutral,duplicates,invalid});
      setMessage(`已识别 ${parsed.length} 笔可导入记录${duplicates?`，自动跳过 ${duplicates} 笔重复账单`:""}`);
    }catch(error){setRows([]);setMessage(error instanceof Error?error.message:"文件解析失败")}
    finally{if(input.current)input.current.value=""}
  };
  const parsePhoneBill=async(item:ImportUploadItem)=>{
    setMessage(`正在从手机文件解析：${item.filename}`);
    const response=await fetch(`/api/imports?id=${encodeURIComponent(item.id)}`);
    if(!response.ok){setMessage("无法读取手机上传的账单文件");return}
    const file=new File([await response.blob()],item.filename,{type:item.contentType});
    await read(file);
    await fetch("/api/imports",{method:"PATCH",headers:{"content-type":"application/json"},body:JSON.stringify({id:item.id,status:"parsed"})});
    await loadPhoneUploads();
  };
  const deletePhoneUpload=async(id:string)=>{await fetch(`/api/imports?id=${encodeURIComponent(id)}`,{method:"DELETE"});await loadPhoneUploads()};
  const commit=async()=>{setImporting(true);try{for(const row of rows){const {sourceId:_,...transaction}=row;await addTransaction(transaction)}setMessage(`已导入 ${rows.length} 笔微信账单到 SQLite`);setRows([]);notify("微信账单导入完成")}catch(error){setMessage(error instanceof Error?error.message:"账单导入失败")}finally{setImporting(false)}};
  return <div className="hx-view">
    <article className="hx-panel hx-phone-imports">
      <PanelHead kicker="手机传输" title="待处理导入文件"/>
      <div className="hx-panel-body">
        <div className="hx-phone-import-head"><p>手机上传的健身数据图和账单文件保存在电脑本地。</p><button className="hx-btn secondary" onClick={()=>void loadPhoneUploads()}>刷新列表</button></div>
        <div className="hx-phone-import-list">
          {phoneUploads.map(item=><article key={item.id}>
            <span>{item.kind==="fitness"?<Dumbbell/>:<FileUp/>}</span>
            <div><strong>{item.filename}</strong><small>{item.kind==="fitness"?"健身数据图":"账单文件"} · {(item.size/1024/1024).toFixed(2)} MB · {new Date(item.createdAt).toLocaleString("zh-CN")}</small></div>
            <div>
              <a className="hx-btn secondary" href={`/api/imports?id=${encodeURIComponent(item.id)}`} target="_blank" rel="noreferrer">{item.kind==="fitness"?"查看":"下载"}</a>
              {item.kind==="bill"&&/\.(xlsx|csv)$/i.test(item.filename)&&<button className="hx-btn primary" onClick={()=>void parsePhoneBill(item)}>{item.status==="parsed"?"重新解析":"电脑解析"}</button>}
              <button className="hx-icon-btn" aria-label={`删除${item.filename}`} onClick={()=>void deletePhoneUpload(item.id)}><Trash2/></button>
            </div>
          </article>)}
          {!phoneUploads.length&&!loadingUploads&&<Empty text="手机还没有上传文件。"/>}
          {loadingUploads&&<Empty text="正在读取手机上传文件…"/>}
        </div>
      </div>
    </article>
    <div className="hx-import-grid">
      <article className="hx-panel"><PanelHead kicker="微信账单" title="导入支付流水"/><div className="hx-panel-body"><div className="hx-drop" onClick={()=>input.current?.click()}><FileUp/><h3>选择微信支付账单</h3><p>支持微信导出的 Excel（.xlsx）以及通用 CSV 文件。</p><input ref={input} type="file" hidden accept=".xlsx,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet,.csv,text/csv" onChange={e=>{const file=e.target.files?.[0];if(file)void read(file)}}/><button className="hx-btn primary">选择账单文件</button></div>{message&&<p className="hx-inline-message">{message}</p>}{rows.length>0&&<div className="hx-import-preview"><div>{rows.slice(0,12).map((row,index)=><span key={`${row.sourceId??index}`}><b>{row.counterparty}</b><small>{row.category} · {row.account} · {new Date(row.occurredAt??"").toLocaleDateString("zh-CN")}</small><strong>{row.type==="expense"?"-":"+"}{money(row.amount)}</strong></span>)}</div>{rows.length>12&&<small>另有 {rows.length-12} 笔记录将在确认后一起导入</small>}<button className="hx-btn primary" disabled={importing} onClick={commit}>{importing?"正在导入…":`确认导入 ${rows.length} 笔`}</button></div>}</div></article>
      <aside className="hx-panel"><PanelHead kicker="识别结果" title="微信格式规则"/><div className="hx-panel-body hx-rules"><p><b>1</b> 自动跳过文件顶部的汇总说明，定位“交易时间”表头。</p><p><b>2</b> 收入与支出正常导入，中性交易不会误记为消费。</p><p><b>3</b> 使用微信交易单号去重，重复上传不会重复入账。</p><p><b>4</b> 自动识别支付方式并推断餐饮、交通、购物等分类。</p>{summary.source>0&&<div className="hx-import-stats"><span>明细行 <b>{summary.source}</b></span><span>中性交易 <b>{summary.neutral}</b></span><span>重复账单 <b>{summary.duplicates}</b></span><span>无效记录 <b>{summary.invalid}</b></span></div>}<hr/><small>当前已有 {transactions.length} 笔账单，{accounts.length} 个账户。</small></div></aside>
    </div>
  </div>;
}

function CalendarView() {
  const { activities, logs, transactions, reviews }=useLifeStore();const now=new Date();const [selected,setSelected]=useState(now.getDate());const first=(new Date(now.getFullYear(),now.getMonth(),1).getDay()+6)%7;const count=new Date(now.getFullYear(),now.getMonth()+1,0).getDate();const key=(day:number)=>`${now.getFullYear()}-${pad(now.getMonth()+1)}-${pad(day)}`;const matchesDay=(value:string,date:string)=>dayKey(new Date(value))===date;const selectedKey=key(selected);const selectedLogs=logs.filter(item=>matchesDay(item.createdAt,selectedKey)).sort((left,right)=>new Date(right.createdAt).getTime()-new Date(left.createdAt).getTime());const selectedTx=transactions.filter(item=>matchesDay(item.occurredAt,selectedKey));const review=reviews.find(item=>item.reviewDate===selectedKey);
  return <div className="hx-view"><div className="hx-calendar"><div className="hx-week">{"一二三四五六日".split("").map(item=><span key={item}>周{item}</span>)}</div><div className="hx-days">{Array.from({length:first}).map((_,i)=><i key={i}/>)}{Array.from({length:count},(_,i)=>i+1).map(day=><button className={selected===day?"selected":""} onClick={()=>setSelected(day)} key={day}><b>{day}</b><span>{logs.some(item=>matchesDay(item.createdAt,key(day)))&&<i/>}{transactions.some(item=>matchesDay(item.occurredAt,key(day)))&&<i/>}{reviews.some(item=>item.reviewDate===key(day))&&<i/>}</span></button>)}</div></div><article className="hx-panel hx-day-detail"><PanelHead kicker={`${now.getMonth()+1}月${selected}日`} title="当天详情"/><div className="hx-panel-body"><div className="hx-metrics"><Metric label="项目记录" value={`${selectedLogs.length}`} sub={`${activities.filter(a=>selectedLogs.some(l=>l.activityId===a.id)).length} 个项目`}/><Metric label="当日支出" value={money(selectedTx.filter(i=>i.type==="expense").reduce((s,i)=>s+i.amount,0))} sub={`${selectedTx.length} 笔收支`}/><Metric label="每日复盘" value={review?"已完成":"—"} sub={review?.tomorrowPriority?`明日：${review.tomorrowPriority}`:"尚未填写"}/></div><section className="hx-day-log"><header><div><span className="hx-kicker">生活日志</span><h3>当天做了什么</h3></div><small>{selectedLogs.length} 条记录</small></header>{selectedLogs.length?selectedLogs.map(log=>{const activity=activities.find(item=>item.id===log.activityId);return <article key={log.id}><time>{new Date(log.createdAt).toLocaleTimeString("zh-CN",{hour:"2-digit",minute:"2-digit"})}</time><div><strong>{activity?.name??"生活记录"}</strong><p>{log.note||`${log.status==="partial"?"部分完成":"完成"} ${log.value??1} ${activity?.unit??"次"}`}</p></div><span className={log.status==="partial"?"partial":""}>{log.status==="partial"?"部分完成":"已完成"}</span></article>}):<Empty text="当天还没有生活日志。"/>}</section></div></article></div>;
}

function ReviewView() {
  const {reviews,saveReview}=useLifeStore();const current=reviews.find(item=>item.reviewDate===dayKey());const [energy,setEnergy]=useState(current?.energy??7);const [mood,setMood]=useState(current?.mood??7);const [best,setBest]=useState(current?.bestThing??"");const [problem,setProblem]=useState(current?.problem??"");const [priority,setPriority]=useState(current?.tomorrowPriority??"");const [note,setNote]=useState(current?.note??"");const [saved,setSaved]=useState(false);
  return <div className="hx-view hx-review"><form onSubmit={async e=>{e.preventDefault();await saveReview({energy,mood,bestThing:best,problem,tomorrowPriority:priority,note});setSaved(true)}}><Range label="今天的精力" value={energy} set={setEnergy}/><Range label="今天的心情" value={mood} set={setMood}/><label>今天做得最好的一件事<textarea value={best} onChange={e=>setBest(e.target.value)}/></label><label>今天遇到的问题<textarea value={problem} onChange={e=>setProblem(e.target.value)}/></label><label>明天最重要的一件事<input value={priority} onChange={e=>setPriority(e.target.value)}/></label><label>补充备注<textarea value={note} onChange={e=>setNote(e.target.value)}/></label><button className="hx-btn primary">{saved?<><Check/> 已保存今天</>:"保存今日复盘"}</button></form></div>;
}
function Range({label,value,set}:{label:string;value:number;set:(value:number)=>void}){return <label className="hx-range"><span>{label}<b>{value}/10</b></span><input type="range" min="1" max="10" value={value} onChange={e=>set(Number(e.target.value))}/></label>}

function SettingsView() {
  const store=useLifeStore();const input=useRef<HTMLInputElement>(null);const [message,setMessage]=useState("");const download=(text:string,name:string,type="application/json")=>{const url=URL.createObjectURL(new Blob([text],{type}));const link=document.createElement("a");link.href=url;link.download=name;link.click();URL.revokeObjectURL(url)};const backup={exportedAt:new Date().toISOString(),activities:store.activities,logs:store.logs,transactions:store.transactions,reviews:store.reviews,accounts:store.accounts,workoutHistory:store.workoutHistory};
  return <div className="hx-view"><div className="hx-settings-grid"><article className="hx-panel"><PanelHead kicker="数据备份" title="数据备份"/><div className="hx-panel-body"><p>导出完整 JSON 备份，包含坚持、复盘、训练、账户、账单、笔记、标签、关联和版本历史。</p><div className="hx-settings-actions"><button className="hx-btn primary" onClick={async()=>{try{const notesBackup=await noteApi.backup();download(JSON.stringify({...backup,notesBackup},null,2),"life-trace-backup.json")}catch(error){setMessage(error instanceof Error?error.message:"导出失败")}}}>导出备份</button><button className="hx-btn secondary" onClick={()=>input.current?.click()}>恢复备份</button><input ref={input} hidden type="file" accept=".json,application/json" onChange={async e=>{const file=e.target.files?.[0];if(!file)return;try{const data=JSON.parse(await file.text()) as Record<string,unknown>;await store.restoreBackup(data);if(data.notesBackup)await noteApi.restoreBackup(data.notesBackup as Record<string,unknown>);setMessage("完整备份已恢复到 SQLite")}catch(error){setMessage(error instanceof Error?error.message:"恢复失败")}}}/></div>{message&&<p className="hx-inline-message">{message}</p>}</div></article><article className="hx-panel"><PanelHead kicker="本地存储" title="SQLite 存储状态"/><div className="hx-panel-body hx-storage"><span>坚持项目 <b>{store.activities.length} 个</b></span><span>坚持记录 <b>{store.logs.length} 条</b></span><span>训练历史 <b>{store.workoutHistory.length} 条</b></span><span>账户 / 账单 <b>{store.accounts.length} / {store.transactions.length}</b></span><span>笔记数据库 <b className="positive">已纳入备份</b></span><span>数据库连接 <b className="positive">正常</b></span></div></article></div></div>;
}

function EditorModal({ modal, close }: { modal: Exclude<Modal,null>; close: () => void }) {
  if(modal.kind==="record")return <RecordForm activity={modal.value} close={close}/>;
  if(modal.kind==="activity")return <ActivityForm value={modal.value} close={close}/>;
  if(modal.kind==="transaction")return <TransactionForm value={modal.value} close={close}/>;
  if(modal.kind==="account")return <AccountForm value={modal.value} close={close}/>;
  return null;
}
function ModalFrame({title,close,children}:{title:string;close:()=>void;children:React.ReactNode}){return <div className="hx-overlay" onMouseDown={e=>{if(e.target===e.currentTarget)close()}}><div className="hx-modal"><header><div><span className="hx-kicker">编辑内容</span><h2>{title}</h2></div><button onClick={close}><X/></button></header>{children}</div></div>}
function RecordForm({activity,close}:{activity:Activity;close:()=>void}) {
  const {addLog}=useLifeStore();
  const [value,setValue]=useState(activity.normalTarget??1);
  const [status,setStatus]=useState<NonNullable<ActivityLog["status"]>>("completed");
  const [state,setState]=useState<NonNullable<ActivityLog["metadata"]>["state"]>("stable");
  const [urgeLevel,setUrgeLevel]=useState(5);
  const [triggers,setTriggers]=useState<string[]>([]);
  const [actions,setActions]=useState<string[]>([]);
  const [note,setNote]=useState("");
  const triggerOptions=["压力","疲劳","无聊","社交场景","环境诱因"];
  const actionOptions=["离开现场","喝水","短暂散步","呼吸放松","联系支持者"];
  const toggle=(list:string[],item:string,set:(value:string[])=>void)=>set(list.includes(item)?list.filter(value=>value!==item):[...list,item]);
  const submit=async(e:React.FormEvent)=>{
    e.preventDefault();
    if(activity.type==="control"){
      await addLog(activity.id,undefined,"completed",{state,urgeLevel:state==="stable"?undefined:urgeLevel,triggers:state==="stable"?[]:triggers,actions:state==="stable"?[]:actions},note);
    }else if(activity.type==="completion"){
      await addLog(activity.id,undefined,status,undefined,note);
    }else{
      await addLog(activity.id,value,"completed",undefined,note);
    }
    notify(`${activity.name}已记录`);
    close();
  };
  return <ModalFrame title={`记录：${activity.name}`} close={close}><form className="hx-form hx-record-form" onSubmit={submit}>
    {activity.type==="control"?<>
      <label>当前状态<div className="hx-choice-row">{[["stable","保持稳定"],["urge","出现冲动"],["relapse","发生偏离"]].map(([id,label])=><button type="button" key={id} className={state===id?"active":""} onClick={()=>setState(id as typeof state)}>{label}</button>)}</div></label>
      {state!=="stable"&&<><label className="hx-range"><span>冲动强度<b>{urgeLevel}/10</b></span><input type="range" min="1" max="10" value={urgeLevel} onChange={e=>setUrgeLevel(Number(e.target.value))}/></label>
      <fieldset className="hx-choice-group"><legend>可能诱因（可多选）</legend><div>{triggerOptions.map(item=><button type="button" key={item} className={triggers.includes(item)?"active":""} onClick={()=>toggle(triggers,item,setTriggers)}>{item}</button>)}</div></fieldset>
      <fieldset className="hx-choice-group"><legend>已采取行动（可多选）</legend><div>{actionOptions.map(item=><button type="button" key={item} className={actions.includes(item)?"active":""} onClick={()=>toggle(actions,item,setActions)}>{item}</button>)}</div></fieldset></>}
    </>:activity.type==="completion"?<label>完成情况<div className="hx-choice-row">{[["completed","已完成"],["partial","部分完成"],["skipped","今天跳过"]].map(([id,label])=><button type="button" key={id} className={status===id?"active":""} onClick={()=>setStatus(id as typeof status)}>{label}</button>)}</div></label>
    :<label>本次完成量<input autoFocus required type="number" min="0" step={activity.type==="duration"?"1":"0.1"} value={value} onChange={e=>setValue(Number(e.target.value))}/><small>单位：{activity.unit}，目标：{activity.normalTarget??1} {activity.unit}</small></label>}
    <label>备注（可选）<textarea value={note} onChange={e=>setNote(e.target.value)} placeholder="记录当时的情况或感受"/></label>
    <footer><button type="button" className="hx-btn secondary" onClick={close}>取消</button><button className="hx-btn primary">保存记录</button></footer>
  </form></ModalFrame>;
}
function ActivityForm({value,close}:{value?:Activity;close:()=>void}){const {addActivity,updateActivity}=useLifeStore();const [name,setName]=useState(value?.name??"");const [type,setType]=useState<Activity["type"]>(value?.type??"duration");const [unit,setUnit]=useState(value?.unit??"分钟");const [target,setTarget]=useState(value?.normalTarget??30);const [description,setDescription]=useState(value?.description??"");return <ModalFrame title={value?"编辑坚持项目":"创建坚持项目"} close={close}><form className="hx-form" onSubmit={async e=>{e.preventDefault();if(value)await updateActivity(value.id,{name,type,unit,normalTarget:target,description});else await addActivity({name,type,unit,normalTarget:target,targetPeriod:type==="weekly"?"weekly":"daily",description});close()}}><label>项目名称<input required value={name} onChange={e=>setName(e.target.value)}/></label><label>类型<select value={type} onChange={e=>setType(e.target.value as Activity["type"])}><option value="duration">时长型</option><option value="count">次数型</option><option value="completion">完成型</option><option value="weekly">每周型</option><option value="control">行为管理</option></select></label><div><label>目标值<input type="number" min="1" value={target} onChange={e=>setTarget(Number(e.target.value))}/></label><label>单位<input value={unit} onChange={e=>setUnit(e.target.value)}/></label></div><label>项目说明<textarea value={description} onChange={e=>setDescription(e.target.value)}/></label><footer><button type="button" className="hx-btn secondary" onClick={close}>取消</button><button className="hx-btn primary">保存</button></footer></form></ModalFrame>}
function TransactionForm({value,close}:{value?:Transaction;close:()=>void}){const {accounts,addTransaction,updateTransaction}=useLifeStore();const [type,setType]=useState<Transaction["type"]>(value?.type??"expense");const [amount,setAmount]=useState(value?.amount??0);const [category,setCategory]=useState(value?.category??"餐饮");const [accountId,setAccountId]=useState(value?.accountId??accounts[0]?.id??"");const [counterparty,setCounterparty]=useState(value?.counterparty??"");const [item,setItem]=useState(value?.item??"");const [occurredAt,setOccurredAt]=useState(dateTimeLocal(value?.occurredAt));const account=accounts.find(i=>i.id===accountId);return <ModalFrame title={value?"编辑账单":"手动记账"} close={close}><form className="hx-form" onSubmit={async e=>{e.preventDefault();const data={type,amount,category,account:account?.name??"未分配",accountId,counterparty,item,occurredAt:new Date(occurredAt).toISOString()};if(value)await updateTransaction(value.id,data);else await addTransaction(data);close()}}><div><label>收支类型<select value={type} onChange={e=>setType(e.target.value as Transaction["type"])}><option value="expense">支出</option><option value="income">收入</option></select></label><label>金额<input required min="0.01" step="0.01" type="number" value={amount} onChange={e=>setAmount(Number(e.target.value))}/></label></div><div><label>分类<input required value={category} onChange={e=>setCategory(e.target.value)}/></label><label>账户<select value={accountId} onChange={e=>setAccountId(e.target.value)}>{accounts.map(i=><option value={i.id} key={i.id}>{i.name}</option>)}</select></label></div><label>交易对象<input required value={counterparty} onChange={e=>setCounterparty(e.target.value)}/></label><label>商品 / 说明<input value={item} onChange={e=>setItem(e.target.value)}/></label><label>交易时间<input type="datetime-local" value={occurredAt} onChange={e=>setOccurredAt(e.target.value)}/></label><footer><button type="button" className="hx-btn secondary" onClick={close}>取消</button><button className="hx-btn primary">保存</button></footer></form></ModalFrame>}
function AccountForm({value,close}:{value?:FinanceAccount;close:()=>void}){const {saveAccount}=useLifeStore();const [name,setName]=useState(value?.name??"");const [type,setType]=useState<FinanceAccount["type"]>(value?.type??"bank");const [balance,setBalance]=useState(value?.balance??0);const [last4,setLast4]=useState(value?.last4??"");const [color,setColor]=useState(value?.color??"#2a7a5e");const [icon,setIcon]=useState(value?.icon??"账");return <ModalFrame title={value?"编辑账户":"添加账户"} close={close}><form className="hx-form" onSubmit={async e=>{e.preventDefault();await saveAccount({id:value?.id,name,type,balance,last4,color,icon});close()}}><label>账户名称<input required value={name} onChange={e=>setName(e.target.value)}/></label><div><label>账户类型<select value={type} onChange={e=>setType(e.target.value as FinanceAccount["type"])}><option value="bank">银行卡</option><option value="wechat">微信</option><option value="alipay">支付宝</option><option value="cash">现金</option><option value="investment">投资账户</option><option value="other">其他</option></select></label><label>当前余额<input type="number" step="0.01" value={balance??0} onChange={e=>setBalance(Number(e.target.value))}/></label></div><div><label>尾号<input maxLength={4} value={last4} onChange={e=>setLast4(e.target.value)}/></label><label>标识<input maxLength={2} value={icon} onChange={e=>setIcon(e.target.value)}/></label></div><label>颜色<input type="color" value={color} onChange={e=>setColor(e.target.value)}/></label><footer><button type="button" className="hx-btn secondary" onClick={close}>取消</button><button className="hx-btn primary">保存</button></footer></form></ModalFrame>}
export default function HengXuShell() {
  const {ready,storageError,initialize}=useLifeStore();const [view,setView]=useState<PlatformView>("dashboard");const [modal,setModal]=useState<Modal>(null);const [menu,setMenu]=useState(false);const [toast,setToast]=useState("");
  const makeLinkedNote=async(noteType:"habit_log"|"workout_review"|"expense_note",title:string,entityType:"habit"|"workout"|"transaction",entityId:string,content:string)=>{
    const created=await noteApi.create({title,noteType,folderId:null,contentJson:{type:"doc",content:[{type:"paragraph",content:[{type:"text",text:content}]}]},contentHtml:`<p>${escapeHtml(content).replace(/\n/g,"<br>")}</p>`,contentText:content,contentMarkdown:content,summary:content.replace(/\s+/g," ").slice(0,160),isPinned:false,isFavorite:false,isArchived:false,tagIds:[],relations:[{id:crypto.randomUUID(),noteId:"pending",entityType,entityId,relationType:"created_from",createdAt:new Date().toISOString()}]});
    window.localStorage.setItem("lifetrace:last-note",created.id);setView("notes");notify("关联笔记已创建");
  };
  useEffect(()=>{void initialize()},[initialize]);
  useEffect(()=>{
    const params=new URLSearchParams(window.location.search);
    const requestedView=params.get("view");
    if(requestedView&&requestedView in pageCopy)setView(requestedView as PlatformView);
  },[]);
  useEffect(()=>{const receive=(event:Event)=>setToast((event as CustomEvent<string>).detail);window.addEventListener("hengxu-toast",receive);return()=>window.removeEventListener("hengxu-toast",receive)},[]);
  useEffect(()=>{if(!toast)return;const timer=window.setTimeout(()=>setToast(""),2200);return()=>window.clearTimeout(timer)},[toast]);
  if(!ready)return <div className="hx-loading"><span>LT</span><p>正在连接 SQLite 个人系统…</p></div>;
  if(storageError)return <div className="hx-loading"><span>!</span><h1>SQLite 暂时无法连接</h1><p>{storageError}</p><button className="hx-btn primary" onClick={()=>initialize()}>重新连接</button></div>;
  const [title,subtitle]=pageCopy[view];
  return <main className="hx-shell"><aside className={menu?"open":""} aria-label="主导航"><div className="hx-brand"><span>LT</span><div><strong>Life trace</strong><small>个人管理系统</small></div></div><nav>{navGroups.map(group=><div key={group.label}><label>{group.label}</label>{group.items.map(({id,label,icon:Icon})=><button className={view===id?"active":""} aria-current={view===id?"page":undefined} key={id} onClick={()=>{setView(id);setMenu(false)}}><span><Icon/>{label}</span><ChevronRight/></button>)}</div>)}</nav><div className="hx-sidebar-foot"><div><i/><strong>本地 SQLite 模式</strong><p>数据保存在本机数据库，不依赖浏览器存储。</p></div><button className={view==="settings"?"active":""} aria-current={view==="settings"?"page":undefined} onClick={()=>{setView("settings");setMenu(false)}}><span><Settings/>数据与设置</span><ChevronRight/></button><section><span>LT</span><div><strong>个人空间</strong><small>本地账户</small></div></section></div></aside>{menu&&<button className="hx-nav-scrim" aria-label="关闭导航" onClick={()=>setMenu(false)}/>}<div className="hx-main"><header className="hx-topbar"><button className="hx-menu" aria-label={menu?"关闭导航":"打开导航"} aria-expanded={menu} onClick={()=>setMenu(!menu)}><Menu/></button><div><span className="hx-kicker">{new Intl.DateTimeFormat("zh-CN",{month:"long",day:"numeric",weekday:"long"}).format(new Date())}</span><h1>{title}</h1><p>{subtitle}</p></div></header>{view==="dashboard"&&<Dashboard go={setView} record={value=>setModal({kind:"record",value})}/>} {view==="habits"&&<Habits edit={value=>setModal({kind:"activity",value})} record={value=>setModal({kind:"record",value})} note={value=>void makeLinkedNote("habit_log",`${value.name}练习记录 - ${dayKey()}`,"habit",value.id,`今天的记录：\n\n问题：\n\n下次重点：`)}/>} {view==="english"&&<DailyEnglish/>} {view==="fitness"&&<Fitness note={value=>void makeLinkedNote("workout_review",`训练复盘 - ${dayKey(new Date(value.occurredAt))}`,"workout",value.id,`训练名称：${value.name}\n训练日期：${dayKey(new Date(value.occurredAt))}\n训练时长：${Math.max(1,Math.round(value.durationSeconds/60))} 分钟\n总容量：${value.volumeKg??"未记录"}\n动作数量：${value.exerciseCount}\n训练来源：${value.source}`)}/>} {view==="notes"&&<NotesModule/>} {view==="finance"&&<Finance/>} {view==="transactions"&&<Transactions edit={value=>setModal({kind:"transaction",value})} note={value=>void makeLinkedNote("expense_note",`消费记录 - ${value.counterparty||value.category}`,"transaction",value.id,`日期：${dayKey(new Date(value.occurredAt))}\n金额：¥${value.amount.toFixed(2)}\n分类：${value.category}\n账户：${value.account}\n商户：${value.counterparty||"未填写"}\n消费目的：`)}/>} {view==="accounts"&&<Accounts edit={value=>setModal({kind:"account",value})}/>} {view==="import"&&<ImportBills/>} {view==="calendar"&&<CalendarView/>} {view==="review"&&<ReviewView/>} {view==="settings"&&<SettingsView/>}</div>{modal&&<EditorModal modal={modal} close={()=>setModal(null)}/>} {toast&&<div className="hx-toast" role="status"><Check/>{toast}</div>}</main>;
}
