import { useEffect, useMemo, useState, type Dispatch, type FormEvent, type ReactNode, type SetStateAction } from "react";
import {
  Activity,
  BookOpen,
  CalendarDays,
  Check,
  CircleDollarSign,
  Command,
  Dumbbell,
  FileText,
  Home,
  ListTodo,
  Moon,
  NotebookPen,
  PanelLeft,
  Plus,
  RefreshCw,
  Repeat2,
  Search,
  Settings,
  Sun,
  WalletCards
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { initialState, isoDate, money, newId, financeSummary, reviewMetrics, searchState, type FinanceTransaction, type LifeTraceState } from "./model";
import type { PlatformAdapter } from "./platform";
import {
  Badge,
  Button,
  Card,
  ChartContainer,
  Checkbox,
  CommandPalette,
  EmptyState,
  IconButton,
  Input,
  List,
  ListItem,
  Modal,
  Progress,
  SearchField,
  SegmentedControl,
  Select,
  StatCard,
  Switch,
  Table,
  Textarea
} from "./design-system/ui";

interface NavigationItem {
  label: string;
  path: string;
  icon: LucideIcon;
  mobile?: boolean;
}

const navigation: NavigationItem[] = [
  { label: "Today", path: "/app/today", icon: Home, mobile: true },
  { label: "Plan", path: "/app/execution", icon: ListTodo, mobile: true },
  { label: "Calendar", path: "/app/calendar", icon: CalendarDays, mobile: true },
  { label: "Habits", path: "/app/habits", icon: Repeat2 },
  { label: "Fitness", path: "/app/fitness", icon: Dumbbell, mobile: true },
  { label: "Finance", path: "/app/finance", icon: CircleDollarSign, mobile: true },
  { label: "Reading", path: "/app/english/articles", icon: BookOpen },
  { label: "Notes", path: "/app/notes", icon: NotebookPen },
  { label: "Review", path: "/app/review", icon: Activity },
  { label: "Search", path: "/app/search", icon: Search },
  { label: "Settings", path: "/app/settings", icon: Settings }
];

const routeTitle = (path: string) => {
  if (path.startsWith("/app/finance")) return "Finance";
  if (path.startsWith("/app/english")) return "Reading";
  if (path === "/app/health") return "Fitness / Health";
  return navigation.find((item) => path.startsWith(item.path))?.label ?? "LifeTrace";
};

const isActive = (path: string, item: NavigationItem) => item.path === "/app/finance" ? path.startsWith("/app/finance") : item.path === "/app/english/articles" ? path.startsWith("/app/english") : path === item.path;

type SetState = Dispatch<SetStateAction<LifeTraceState>>;

function PageHeader({ title, detail, action }: { title: string; detail: string; action?: ReactNode }) {
  return <header className="lt-page-header"><div className="lt-row-between"><div><h1>{title}</h1><p>{detail}</p></div>{action}</div></header>;
}

function Section({ title, action, children }: { title: string; action?: ReactNode; children: ReactNode }) {
  return <section className="lt-section"><div className="lt-section-header"><h2>{title}</h2>{action}</div>{children}</section>;
}

function TodayPage({ state, setState, openQuickCapture }: { state: LifeTraceState; setState: SetState; openQuickCapture: () => void }) {
  const today = isoDate();
  const tasks = state.tasks.filter((task) => task.dueDate === today);
  const metrics = reviewMetrics(state, today);
  const priority = tasks.find((task) => !task.completed && task.priority === "high") ?? tasks.find((task) => !task.completed);
  const workouts = state.workouts.filter((workout) => workout.date === today);

  const toggleTask = (id: string) => setState((current) => ({ ...current, tasks: current.tasks.map((task) => task.id === id ? { ...task, completed: !task.completed } : task) }));
  const toggleHabit = (id: string) => setState((current) => ({ ...current, habits: current.habits.map((habit) => habit.id !== id ? habit : { ...habit, completedDates: habit.completedDates.includes(today) ? habit.completedDates.filter((date) => date !== today) : [...habit.completedDates, today], streak: habit.completedDates.includes(today) ? Math.max(0, habit.streak - 1) : habit.streak + 1 }) }));

  return <>
    <PageHeader title="Today" detail={`${new Intl.DateTimeFormat("zh-CN", { month: "long", day: "numeric", weekday: "long" }).format(new Date())} · 只保留今天真正需要行动的信息。`} action={<Button onClick={openQuickCapture}><Plus size={17} /> Quick Capture</Button>} />
    <div className="lt-metrics">
      <StatCard label="今日任务" value={`${metrics.completedTasks}/${metrics.totalTasks}`} detail="完成 / 计划" />
      <StatCard label="习惯" value={`${metrics.completedHabits}/${metrics.totalHabits}`} detail="今日打卡" />
      <StatCard label="训练" value={workouts.reduce((sum, item) => sum + item.durationMinutes, 0)} detail="分钟" />
    </div>
    <div className="lt-grid-2 lt-section">
      <Card>
        <div className="lt-caption">TODAY FOCUS</div>
        <h2 style={{ margin: "10px 0 8px" }}>{priority?.title ?? "为今天设定一个重点"}</h2>
        <p className="lt-muted" style={{ margin: 0 }}>{priority ? `${priority.project || "Inbox"} · ${priority.priority === "high" ? "高优先级" : "下一步行动"}` : "用 Quick Capture 添加今天最重要的行动。"}</p>
      </Card>
      <Card>
        <div className="lt-caption">NEXT SIGNAL</div>
        <h2 style={{ margin: "10px 0 8px" }}>{state.reading.find((item) => !item.completed)?.title ?? "保持输入与输出平衡"}</h2>
        <p className="lt-muted" style={{ margin: 0 }}>{state.reading.length ? "继续最近的阅读记录，完成后留下简短笔记。" : "添加一篇阅读材料，记录高亮和学习结论。"}</p>
      </Card>
    </div>
    <Section title="Tasks">
      {tasks.length ? <List>{tasks.map((task) => <ListItem key={task.id}><Checkbox checked={task.completed} onChange={() => toggleTask(task.id)} aria-label={`完成 ${task.title}`} /><div style={{ flex: 1 }}><div style={{ textDecoration: task.completed ? "line-through" : undefined }}>{task.title}</div><span className="lt-caption">{task.project || "Inbox"}</span></div>{task.priority === "high" ? <Badge tone="danger">High</Badge> : null}</ListItem>)}</List> : <EmptyState title="今天还没有任务" detail="保持克制：只把今天真正要推进的事项放进来。" action={<Button className="secondary" onClick={openQuickCapture}>添加任务</Button>} />}
    </Section>
    <Section title="Habits">
      {state.habits.length ? <List>{state.habits.map((habit) => { const done = habit.completedDates.includes(today); return <ListItem key={habit.id}><Checkbox checked={done} onChange={() => toggleHabit(habit.id)} aria-label={`打卡 ${habit.name}`} /><div style={{ flex: 1 }}>{habit.name}<div className="lt-caption">连续 {habit.streak} 天</div></div><Badge tone={done ? "success" : "neutral"}>{done ? "已完成" : "待打卡"}</Badge></ListItem>; })}</List> : <EmptyState title="建立第一个习惯" detail="习惯页面负责趋势，Today 只负责今天是否完成。" />}
    </Section>
  </>;
}

function PlanPage({ state, setState }: { state: LifeTraceState; setState: SetState }) {
  const [scope, setScope] = useState<"inbox" | "today" | "upcoming" | "completed">("today");
  const [title, setTitle] = useState("");
  const today = isoDate();
  const visible = state.tasks.filter((task) => {
    if (scope === "inbox") return !task.completed && !task.project;
    if (scope === "today") return !task.completed && task.dueDate === today;
    if (scope === "upcoming") return !task.completed && task.dueDate > today;
    return task.completed;
  });
  const addTask = (event: FormEvent) => { event.preventDefault(); if (!title.trim()) return; setState((current) => ({ ...current, tasks: [{ id: newId("task"), title: title.trim(), dueDate: today, project: "", priority: "normal", completed: false }, ...current.tasks] })); setTitle(""); };
  return <>
    <PageHeader title="Plan" detail="用列表与明确的下一步管理执行，而不是把任务拆成一墙卡片。" />
    <div className="lt-row-between" style={{ marginBottom: 16 }}><SegmentedControl label="任务范围" value={scope} onChange={setScope} options={[{ value: "inbox", label: "Inbox" }, { value: "today", label: "Today" }, { value: "upcoming", label: "Upcoming" }, { value: "completed", label: "Completed" }]} /></div>
    <form className="lt-row" onSubmit={addTask} style={{ marginBottom: 14 }}><Input value={title} onChange={(event) => setTitle(event.target.value)} placeholder="新增任务…" style={{ flex: 1 }} /><Button type="submit"><Plus size={17} />添加</Button></form>
    {visible.length ? <List>{visible.map((task) => <ListItem key={task.id}><Checkbox checked={task.completed} onChange={() => setState((current) => ({ ...current, tasks: current.tasks.map((item) => item.id === task.id ? { ...item, completed: !item.completed } : item) }))} /><div style={{ flex: 1 }}><strong style={{ fontWeight: 500 }}>{task.title}</strong><div className="lt-caption">{task.dueDate} · {task.project || "Inbox"}</div></div><Select aria-label="优先级" value={task.priority} onChange={(event) => setState((current) => ({ ...current, tasks: current.tasks.map((item) => item.id === task.id ? { ...item, priority: event.target.value as "low" | "normal" | "high" } : item) }))}><option value="low">Low</option><option value="normal">Normal</option><option value="high">High</option></Select></ListItem>)}</List> : <EmptyState title="这里已经清空" detail="没有待处理项目时，不添加额外视觉噪声。" />}
  </>;
}

function CalendarPage({ state }: { state: LifeTraceState }) {
  const now = new Date();
  const year = now.getFullYear();
  const month = now.getMonth();
  const days = new Date(year, month + 1, 0).getDate();
  const cells = Array.from({ length: days }, (_, index) => index + 1);
  return <>
    <PageHeader title="Calendar" detail="桌面显示月视图；窄窗口自动转为可扫描的日程列表。" />
    <div className="lt-calendar-grid">{cells.map((day) => { const date = `${year}-${String(month + 1).padStart(2, "0")}-${String(day).padStart(2, "0")}`; const items = state.tasks.filter((task) => task.dueDate === date); return <div className="lt-day-cell" key={day}><strong>{day}</strong>{items.slice(0, 3).map((task) => <small key={task.id}>{task.completed ? "✓ " : ""}{task.title}</small>)}</div>; })}</div>
  </>;
}

function HabitsPage({ state, setState }: { state: LifeTraceState; setState: SetState }) {
  const [name, setName] = useState("");
  const today = isoDate();
  const add = (event: FormEvent) => { event.preventDefault(); if (!name.trim()) return; setState((current) => ({ ...current, habits: [...current.habits, { id: newId("habit"), name: name.trim(), targetDays: 7, streak: 0, completedDates: [] }] })); setName(""); };
  return <>
    <PageHeader title="Habits" detail="当天打卡优先，趋势用于反馈，不让图表淹没行动。" />
    <form className="lt-row" onSubmit={add}><Input value={name} onChange={(event) => setName(event.target.value)} placeholder="添加习惯…" style={{ flex: 1 }} /><Button type="submit"><Plus size={17} />添加</Button></form>
    <Section title="Today check-in">{state.habits.length ? <List>{state.habits.map((habit) => { const done = habit.completedDates.includes(today); const recent = Array.from({ length: 7 }, (_, i) => { const date = new Date(); date.setDate(date.getDate() - i); return habit.completedDates.includes(isoDate(date)); }).filter(Boolean).length; return <ListItem key={habit.id}><Checkbox checked={done} onChange={() => setState((current) => ({ ...current, habits: current.habits.map((item) => item.id === habit.id ? { ...item, completedDates: done ? item.completedDates.filter((date) => date !== today) : [...item.completedDates, today], streak: done ? Math.max(0, item.streak - 1) : item.streak + 1 } : item) }))} /><div style={{ flex: 1 }}><div className="lt-row-between"><strong style={{ fontWeight: 500 }}>{habit.name}</strong><span className="lt-caption">7 日 {recent}/7</span></div><Progress label={`${habit.name} 七日完成率`} value={(recent / 7) * 100} /></div><Badge tone={habit.streak >= 3 ? "success" : "neutral"}>{habit.streak} day streak</Badge></ListItem>; })}</List> : <EmptyState title="没有习惯" detail="从一个可每天执行的小动作开始。" />}</Section>
  </>;
}

function FitnessPage({ state, setState }: { state: LifeTraceState; setState: SetState }) {
  const [title, setTitle] = useState("");
  const [minutes, setMinutes] = useState("45");
  const add = (event: FormEvent) => { event.preventDefault(); if (!title.trim()) return; setState((current) => ({ ...current, workouts: [{ id: newId("workout"), date: isoDate(), title: title.trim(), durationMinutes: Number(minutes) || 0, volume: 0 }, ...current.workouts] })); setTitle(""); };
  const week = state.workouts.slice(0, 7);
  return <>
    <PageHeader title="Fitness / Health" detail="用本周训练、训练量和身体趋势组成高密度但克制的分析工作区。" />
    <div className="lt-metrics"><StatCard label="本周训练" value={week.length} detail="sessions" /><StatCard label="训练时长" value={week.reduce((sum, item) => sum + item.durationMinutes, 0)} detail="minutes" /><StatCard label="训练量" value={week.reduce((sum, item) => sum + item.volume, 0).toLocaleString()} detail="volume" /></div>
    <Section title="Record workout"><form className="lt-form-grid two" onSubmit={add}><Input value={title} onChange={(event) => setTitle(event.target.value)} placeholder="训练名称" /><Input type="number" min="0" value={minutes} onChange={(event) => setMinutes(event.target.value)} placeholder="分钟" /><Button type="submit"><Plus size={17} />记录训练</Button></form></Section>
    <div className="lt-grid-2 lt-section"><ChartContainer title="Recent duration"><div className="lt-bar-chart">{(week.length ? [...week].reverse() : [{ durationMinutes: 0 }]).map((item, index) => <span key={index} style={{ height: `${Math.max(4, Math.min(100, item.durationMinutes))}%` }} />)}</div></ChartContainer><Card><h3 style={{ marginTop: 0 }}>Recent workouts</h3>{week.length ? week.map((item) => <div className="lt-row-between" key={item.id} style={{ padding: "10px 0", borderBottom: "1px solid var(--border-subtle)" }}><span>{item.title}</span><span className="lt-caption">{item.durationMinutes} min</span></div>) : <p className="lt-muted">记录一次训练后，这里会形成最近训练摘要。</p>}</Card></div>
  </>;
}

const financeTabs = [
  ["Overview", "/app/finance"], ["Transactions", "/app/finance/transactions"], ["Calendar", "/app/finance/calendar"], ["Ledgers", "/app/finance/ledgers"], ["Budgets", "/app/finance/budgets"], ["Accounts", "/app/finance/accounts"], ["Categories", "/app/finance/categories"], ["Tags", "/app/finance/tags"], ["Import", "/app/finance/import"]
] as const;

function FinancePage({ state, setState, path, navigate }: { state: LifeTraceState; setState: SetState; path: string; navigate: (path: string) => void }) {
  const summary = financeSummary(state.transactions);
  const [title, setTitle] = useState("");
  const [amount, setAmount] = useState("");
  const [direction, setDirection] = useState<"expense" | "income">("expense");
  const [csv, setCsv] = useState("");
  const add = (event: FormEvent) => { event.preventDefault(); const cents = Math.round(Number(amount) * 100); if (!title.trim() || !Number.isFinite(cents)) return; const item: FinanceTransaction = { id: newId("tx"), date: isoDate(), title: title.trim(), category: direction === "expense" ? "日常" : "收入", account: "默认账户", amountCents: Math.abs(cents), direction }; setState((current) => ({ ...current, transactions: [item, ...current.transactions] })); setTitle(""); setAmount(""); };
  const importCsv = () => {
    const parsed = csv.split(/\r?\n/).map((line) => line.trim()).filter(Boolean).map((line) => line.split(",")).filter((parts) => parts.length >= 3).map((parts) => ({ id: newId("tx"), date: parts[0] || isoDate(), title: parts[1] || "导入记录", category: parts[3] || "导入", account: parts[4] || "默认账户", amountCents: Math.round(Math.abs(Number(parts[2]) || 0) * 100), direction: Number(parts[2]) >= 0 ? "income" as const : "expense" as const }));
    if (parsed.length) setState((current) => ({ ...current, transactions: [...parsed, ...current.transactions] }));
    setCsv("");
  };
  return <>
    <PageHeader title="Finance" detail="财务保留成熟账本心智模型，同时统一到 LifeTrace V2 的导航、排版与语义 Token。" />
    <div style={{ overflowX: "auto", paddingBottom: 4 }}><div className="lt-segmented">{financeTabs.map(([label, tabPath]) => <button key={tabPath} className={path === tabPath || (tabPath === "/app/finance" && path === "/app/finance/") ? "is-active" : ""} onClick={() => navigate(tabPath)}>{label}</button>)}</div></div>
    <div className="lt-metrics lt-section"><StatCard label="Balance" value={money(summary.balance)} /><StatCard label="Income" value={money(summary.income)} /><StatCard label="Expense" value={money(summary.expense)} /></div>
    {path.endsWith("/import") ? <Section title="Import transactions"><Card><p className="lt-muted">CSV 格式：日期,标题,金额,分类,账户。正数按收入，负数按支出。</p><Textarea value={csv} onChange={(event) => setCsv(event.target.value)} placeholder="2026-08-21,午餐,-35.5,餐饮,支付宝" /><Button onClick={importCsv} disabled={!csv.trim()} style={{ marginTop: 12 }}>导入</Button></Card></Section> : <>
      <Section title="Quick record"><form className="lt-form-grid two" onSubmit={add}><Input value={title} onChange={(event) => setTitle(event.target.value)} placeholder="交易说明" /><Input value={amount} onChange={(event) => setAmount(event.target.value)} inputMode="decimal" placeholder="金额" /><Select value={direction} onChange={(event) => setDirection(event.target.value as "expense" | "income")}><option value="expense">Expense</option><option value="income">Income</option></Select><Button type="submit"><Plus size={17} />记录</Button></form></Section>
      <Section title="Transactions">{state.transactions.length ? <Table><thead><tr><th>日期</th><th>说明</th><th>分类</th><th>账户</th><th>金额</th></tr></thead><tbody>{state.transactions.map((item) => <tr key={item.id}><td>{item.date}</td><td>{item.title}</td><td>{item.category}</td><td>{item.account}</td><td style={{ color: item.direction === "income" ? "var(--success)" : "var(--text-primary)" }}>{item.direction === "income" ? "+" : "−"}{money(item.amountCents)}</td></tr>)}</tbody></Table> : <EmptyState title="还没有账目" detail="从一条真实记录开始，而不是先填满统计卡片。" />}</Section>
    </>}
  </>;
}

function ReadingPage({ state, setState }: { state: LifeTraceState; setState: SetState }) {
  const [selectedId, setSelectedId] = useState<string | null>(state.reading[0]?.id ?? null);
  const [title, setTitle] = useState("");
  const selected = state.reading.find((item) => item.id === selectedId) ?? state.reading[0];
  const add = (event: FormEvent) => { event.preventDefault(); if (!title.trim()) return; const item = { id: newId("reading"), title: title.trim(), source: "Manual", progress: 0, completed: false, highlights: [], note: "" }; setState((current) => ({ ...current, reading: [item, ...current.reading] })); setSelectedId(item.id); setTitle(""); };
  return <>
    <PageHeader title="Reading / English" detail="进入阅读状态后弱化应用框架，把正文、高亮、快捷笔记和完成反馈放在中心。" />
    <form className="lt-row" onSubmit={add}><Input value={title} onChange={(event) => setTitle(event.target.value)} placeholder="添加阅读材料标题…" style={{ flex: 1 }} /><Button type="submit"><Plus size={17} />添加</Button></form>
    <div className="lt-grid-2 lt-section" style={{ gridTemplateColumns: "minmax(220px, .7fr) minmax(0, 1.5fr)" }}>
      <List>{state.reading.length ? state.reading.map((item) => <ListItem key={item.id} className={item.id === selected?.id ? "is-selected" : ""}><button className="lt-button ghost" style={{ width: "100%", justifyContent: "flex-start", paddingInline: 0 }} onClick={() => setSelectedId(item.id)}><BookOpen size={17} /><span style={{ textAlign: "left", flex: 1 }}>{item.title}<span className="lt-caption" style={{ display: "block" }}>{item.completed ? "已读" : `${item.progress}%`}</span></span></button></ListItem>) : <EmptyState title="阅读列表为空" detail="添加一篇文章开始阅读闭环。" />}</List>
      {selected ? <Card><article className="lt-reader"><div className="lt-caption">{selected.source}</div><h2>{selected.title}</h2><p className="lt-muted">这里是内容优先的阅读工作区。V2 不再使用 Dashboard 卡片包裹正文；真实文章内容由 EnglishArticle 合同与同步层接入。</p><Progress value={selected.progress} label="阅读进度" /><input style={{ width: "100%", margin: "20px 0" }} aria-label="阅读进度" type="range" min="0" max="100" value={selected.progress} onChange={(event) => setState((current) => ({ ...current, reading: current.reading.map((item) => item.id === selected.id ? { ...item, progress: Number(event.target.value) } : item) }))} /><Textarea value={selected.note} onChange={(event) => setState((current) => ({ ...current, reading: current.reading.map((item) => item.id === selected.id ? { ...item, note: event.target.value } : item) }))} placeholder="快捷笔记（不重复展示原句）" /><Button style={{ marginTop: 12 }} onClick={() => setState((current) => ({ ...current, reading: current.reading.map((item) => item.id === selected.id ? { ...item, completed: true, progress: 100 } : item) }))}><Check size={17} />标记读完</Button></article></Card> : <EmptyState title="选择一篇内容" detail="正文区域将在选择后打开。" />}
    </div>
  </>;
}

function NotesPage({ state, setState }: { state: LifeTraceState; setState: SetState }) {
  const [selectedId, setSelectedId] = useState<string | null>(state.notes[0]?.id ?? null);
  const selected = state.notes.find((note) => note.id === selectedId) ?? state.notes[0];
  const create = () => { const note = { id: newId("note"), title: "未命名笔记", content: "", updatedAt: new Date().toISOString(), pinned: false }; setState((current) => ({ ...current, notes: [note, ...current.notes] })); setSelectedId(note.id); };
  const update = (patch: Partial<{ title: string; content: string; pinned: boolean }>) => selected && setState((current) => ({ ...current, notes: current.notes.map((note) => note.id === selected.id ? { ...note, ...patch, updatedAt: new Date().toISOString() } : note) }));
  return <>
    <PageHeader title="Notes" detail="桌面采用 List + Editor，避免把笔记退化为卡片瀑布流。" action={<Button onClick={create}><Plus size={17} />New note</Button>} />
    <div className="lt-notes-layout"><List>{state.notes.length ? state.notes.map((note) => <ListItem key={note.id}><button className="lt-button ghost" style={{ width: "100%", justifyContent: "flex-start", padding: 0 }} onClick={() => setSelectedId(note.id)}><FileText size={17} /><span style={{ textAlign: "left", flex: 1 }}>{note.title || "未命名笔记"}<span className="lt-caption" style={{ display: "block" }}>{new Date(note.updatedAt).toLocaleString("zh-CN")}</span></span></button></ListItem>) : <EmptyState title="没有笔记" detail="新建一条笔记开始记录。" />}</List>{selected ? <Card><Input value={selected.title} onChange={(event) => update({ title: event.target.value })} aria-label="笔记标题" style={{ width: "100%", background: "transparent", fontSize: 24, fontWeight: 600, paddingInline: 0 }} /><textarea className="lt-note-editor" aria-label="笔记内容" value={selected.content} onChange={(event) => update({ content: event.target.value })} placeholder="开始写作…" /></Card> : <EmptyState title="选择或新建笔记" detail="编辑器会在这里打开。" />}</div>
  </>;
}

function ReviewPage({ state, setState }: { state: LifeTraceState; setState: SetState }) {
  const metrics = reviewMetrics(state);
  const existing = state.reviews.find((item) => item.date === isoDate()) ?? { date: isoDate(), bestThing: "", problem: "", tomorrowPriority: "" };
  const save = (field: "bestThing" | "problem" | "tomorrowPriority", value: string) => setState((current) => ({ ...current, reviews: [...current.reviews.filter((item) => item.date !== isoDate()), { ...existing, [field]: value }] }));
  return <>
    <PageHeader title="Review" detail="把日 / 周 / 月回顾聚焦在已完成事项、关键趋势和下一步，而不是装饰型图表。" />
    <div className="lt-grid-2"><ChartContainer title="Task completion"><div style={{ padding: "18px 0" }}><strong style={{ fontSize: 42 }}>{Math.round(metrics.taskCompletion * 100)}%</strong><Progress label="任务完成率" value={metrics.taskCompletion * 100} /></div></ChartContainer><ChartContainer title="Habit completion"><div style={{ padding: "18px 0" }}><strong style={{ fontSize: 42 }}>{Math.round(metrics.habitCompletion * 100)}%</strong><Progress label="习惯完成率" value={metrics.habitCompletion * 100} /></div></ChartContainer></div>
    <Section title="Daily reflection"><Card><div className="lt-form-grid"><label>今天最好的一件事<Textarea value={existing.bestThing} onChange={(event) => save("bestThing", event.target.value)} /></label><label>阻碍 / 问题<Textarea value={existing.problem} onChange={(event) => save("problem", event.target.value)} /></label><label>明天最重要的事<Textarea value={existing.tomorrowPriority} onChange={(event) => save("tomorrowPriority", event.target.value)} /></label></div></Card></Section>
  </>;
}

function SearchPage({ state, navigate }: { state: LifeTraceState; navigate: (path: string) => void }) {
  const [query, setQuery] = useState("");
  const results = useMemo(() => searchState(state, query), [state, query]);
  return <><PageHeader title="Search" detail="跨任务、笔记、阅读和财务记录检索个人工作区。" /><SearchField autoFocus value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索 LifeTrace…" />{query && <Section title={`Results · ${results.length}`}>{results.length ? <List>{results.map((item) => <ListItem key={`${item.type}-${item.id}`}><Badge>{item.type}</Badge><button className="lt-button ghost" style={{ flex: 1, justifyContent: "flex-start" }} onClick={() => navigate(item.path)}>{item.title}</button></ListItem>)}</List> : <EmptyState title="没有匹配结果" detail="尝试更短或更具体的关键词。" />}</Section>}</>;
}

function SettingsPage({ state, setState, platform }: { state: LifeTraceState; setState: SetState; platform: PlatformAdapter }) {
  const [status, setStatus] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  const refresh = async () => { if (!platform.getNativeStatus) return; setBusy(true); try { setStatus(await platform.getNativeStatus()); } finally { setBusy(false); } };
  return <><PageHeader title="Settings" detail="外观、辅助功能、同步和桌面原生能力都在同一套设置层级中。" /><div className="lt-grid-2"><Card><h3 style={{ marginTop: 0 }}>Appearance</h3><div className="lt-row-between" style={{ marginBottom: 16 }}><span>Theme</span><Select value={state.settings.theme} onChange={(event) => setState((current) => ({ ...current, settings: { ...current.settings, theme: event.target.value as "system" | "light" | "dark" } }))}><option value="system">System</option><option value="light">Light</option><option value="dark">Dark</option></Select></div><div className="lt-row-between"><span>Reduced motion</span><Switch label="减少动态效果" checked={state.settings.reducedMotion} onChange={(checked) => setState((current) => ({ ...current, settings: { ...current.settings, reducedMotion: checked } }))} /></div></Card><Card><div className="lt-row-between"><div><h3 style={{ margin: 0 }}>Platform</h3><p className="lt-muted">{platform.label}</p></div><Badge tone={platform.kind === "desktop" ? "accent" : "neutral"}>{platform.kind}</Badge></div>{platform.kind === "desktop" ? <><Button className="secondary" onClick={refresh} disabled={busy}><RefreshCw size={17} />{busy ? "Checking…" : "Native status"}</Button>{platform.syncNow ? <Button style={{ marginLeft: 8 }} onClick={() => void platform.syncNow?.()}>Sync now</Button> : null}</> : <p className="lt-muted">Web 状态通过浏览器平台 adapter 管理；云端数据通过 API / sync adapter 接入。</p>}</Card></div>{status ? <Section title="Native diagnostics"><Card><pre style={{ whiteSpace: "pre-wrap", overflowWrap: "anywhere", margin: 0, fontSize: 12 }}>{JSON.stringify(status, null, 2)}</pre></Card></Section> : null}</>;
}

function DesignSystemPage() {
  return <><PageHeader title="UI System" detail="V2 共享 Design System 的运行时校验页。" /><div className="lt-grid-3"><Card><h3>Controls</h3><div className="lt-row"><Button>Primary</Button><Button className="secondary">Secondary</Button><IconButton aria-label="示例"><Plus /></IconButton></div></Card><Card><h3>Status</h3><div className="lt-row"><Badge tone="success">Success</Badge><Badge tone="warning">Warning</Badge><Badge tone="danger">Danger</Badge></div></Card><Card><h3>Progress</h3><Progress value={62} label="示例进度" /></Card></div></>;
}

function LoginPage({ navigate }: { navigate: (path: string) => void }) {
  return <main className="lt-content" style={{ maxWidth: 520, minHeight: "100vh", display: "grid", alignContent: "center" }}><div className="lt-brand" style={{ padding: 0, marginBottom: 24 }}><span className="lt-brand-mark">LT</span>LifeTrace</div><Card><h1 style={{ marginTop: 0, letterSpacing: "-.03em" }}>Your Personal OS</h1><p className="lt-muted">V2 登录界面保持安静，把认证协议交给平台 auth adapter；当前 clean-room shell 可直接进入本地工作区进行界面与功能回归。</p><div className="lt-form-grid"><Input type="email" placeholder="Email" aria-label="Email" /><Input type="password" placeholder="Password" aria-label="Password" /><Button onClick={() => navigate("/app/today")}>Continue</Button></div></Card></main>;
}

function QuickCapture({ open, onClose, setState }: { open: boolean; onClose: () => void; setState: SetState }) {
  const [kind, setKind] = useState<"task" | "note">("task");
  const [text, setText] = useState("");
  const save = () => { const value = text.trim(); if (!value) return; if (kind === "task") setState((current) => ({ ...current, tasks: [{ id: newId("task"), title: value, dueDate: isoDate(), project: "", priority: "normal", completed: false }, ...current.tasks] })); else setState((current) => ({ ...current, notes: [{ id: newId("note"), title: value.slice(0, 42), content: value, updatedAt: new Date().toISOString(), pinned: false }, ...current.notes] })); setText(""); onClose(); };
  return <Modal open={open} title="Quick Capture" onClose={onClose}><div className="lt-form-grid"><SegmentedControl label="记录类型" value={kind} onChange={setKind} options={[{ value: "task", label: "Task" }, { value: "note", label: "Note" }]} /><Textarea autoFocus value={text} onChange={(event) => setText(event.target.value)} placeholder={kind === "task" ? "下一步要做什么？" : "快速记录一个想法…"} /><Button onClick={save}>Save</Button></div></Modal>;
}

export function LifeTraceApp({ platform }: { platform: PlatformAdapter }) {
  const [state, setState] = useState<LifeTraceState>(initialState);
  const [hydrated, setHydrated] = useState(false);
  const [path, setPath] = useState(() => window.location.pathname === "/" ? "/app/today" : window.location.pathname);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [paletteQuery, setPaletteQuery] = useState("");
  const [quickOpen, setQuickOpen] = useState(false);

  useEffect(() => { let active = true; void platform.loadState().then((stored) => { if (!active) return; if (stored) setState({ ...initialState(), ...stored, settings: { ...initialState().settings, ...stored.settings } }); setHydrated(true); }); return () => { active = false; }; }, [platform]);
  useEffect(() => { if (hydrated) void platform.saveState(state); }, [state, hydrated, platform]);
  useEffect(() => { const systemDark = window.matchMedia?.("(prefers-color-scheme: dark)").matches; const resolved = state.settings.theme === "system" ? (systemDark ? "dark" : "light") : state.settings.theme; document.documentElement.dataset.theme = resolved; document.documentElement.dataset.reducedMotion = String(state.settings.reducedMotion); }, [state.settings]);
  useEffect(() => { const onPop = () => setPath(window.location.pathname); const onKey = (event: KeyboardEvent) => { const mod = event.metaKey || event.ctrlKey; if (mod && event.key.toLowerCase() === "k") { event.preventDefault(); setPaletteOpen((value) => !value); } if (mod && event.key.toLowerCase() === "n") { event.preventDefault(); setQuickOpen(true); } if (mod && event.key === ",") { event.preventDefault(); navigate("/app/settings"); } if (event.key === "Escape") { setPaletteOpen(false); setQuickOpen(false); } }; window.addEventListener("popstate", onPop); window.addEventListener("keydown", onKey); return () => { window.removeEventListener("popstate", onPop); window.removeEventListener("keydown", onKey); }; });

  const navigate = (next: string) => { if (next === path) return; window.history.pushState({}, "", next); setPath(next); window.scrollTo({ top: 0, behavior: state.settings.reducedMotion ? "auto" : "smooth" }); };
  const paletteItems = useMemo(() => {
    const nav = navigation.filter((item) => !paletteQuery || item.label.toLowerCase().includes(paletteQuery.toLowerCase())).map((item) => ({ id: item.path, type: "页面", title: item.label, path: item.path }));
    return [...nav, ...searchState(state, paletteQuery)];
  }, [state, paletteQuery]);

  if (path === "/login") return <LoginPage navigate={navigate} />;

  const renderPage = () => {
    if (path === "/app/today") return <TodayPage state={state} setState={setState} openQuickCapture={() => setQuickOpen(true)} />;
    if (path === "/app/execution" || path.startsWith("/app/execution/")) return <PlanPage state={state} setState={setState} />;
    if (path === "/app/calendar") return <CalendarPage state={state} />;
    if (path === "/app/habits") return <HabitsPage state={state} setState={setState} />;
    if (path === "/app/fitness" || path === "/app/health") return <FitnessPage state={state} setState={setState} />;
    if (path.startsWith("/app/finance")) return <FinancePage state={state} setState={setState} path={path} navigate={navigate} />;
    if (path.startsWith("/app/english")) return <ReadingPage state={state} setState={setState} />;
    if (path === "/app/notes") return <NotesPage state={state} setState={setState} />;
    if (path === "/app/review") return <ReviewPage state={state} setState={setState} />;
    if (path === "/app/search") return <SearchPage state={state} navigate={navigate} />;
    if (path === "/app/settings") return <SettingsPage state={state} setState={setState} platform={platform} />;
    if (path === "/app/system/ui") return <DesignSystemPage />;
    return <EmptyState title="页面未找到" detail={path} action={<Button onClick={() => navigate("/app/today")}>返回 Today</Button>} />;
  };

  const mobileItems = navigation.filter((item) => item.mobile).slice(0, 5);
  return <div className="lt-shell" data-platform={platform.kind}>
    <aside className="lt-sidebar" aria-label="主导航"><div className="lt-brand"><span className="lt-brand-mark">LT</span>LifeTrace</div><nav className="lt-nav">{navigation.slice(0, 9).map((item) => { const Icon = item.icon; return <button key={item.path} className={isActive(path, item) ? "is-active" : ""} onClick={() => navigate(item.path)}><Icon size={18} /><span>{item.label}</span></button>; })}</nav><div className="lt-nav-spacer" /><nav className="lt-nav">{navigation.slice(9).map((item) => { const Icon = item.icon; return <button key={item.path} className={isActive(path, item) ? "is-active" : ""} onClick={() => navigate(item.path)}><Icon size={18} /><span>{item.label}</span></button>; })}</nav><div className="lt-sidebar-footer">{platform.label} · Frontend V2</div></aside>
    <div className="lt-workspace"><header className="lt-toolbar"><IconButton aria-label="打开导航" onClick={() => setPaletteOpen(true)}><PanelLeft size={18} /></IconButton><div className="lt-toolbar-title"><strong>{routeTitle(path)}</strong><span>LifeTrace Personal OS</span></div><SearchField placeholder="Search" aria-label="全局搜索" onFocus={() => setPaletteOpen(true)} readOnly style={{ width: 220 }} /><div className="lt-toolbar-actions"><IconButton aria-label="切换主题" onClick={() => setState((current) => ({ ...current, settings: { ...current.settings, theme: current.settings.theme === "dark" ? "light" : "dark" } }))}>{state.settings.theme === "dark" ? <Sun size={18} /> : <Moon size={18} />}</IconButton><IconButton aria-label="命令菜单" onClick={() => setPaletteOpen(true)}><Command size={18} /></IconButton><IconButton aria-label="快速记录" onClick={() => setQuickOpen(true)}><Plus size={18} /></IconButton></div></header><main className="lt-content">{renderPage()}</main></div>
    <nav className="lt-mobile-nav" aria-label="移动端导航">{mobileItems.map((item) => { const Icon = item.icon; return <button key={item.path} className={isActive(path, item) ? "is-active" : ""} onClick={() => navigate(item.path)}><Icon /><span>{item.label}</span></button>; })}</nav>
    <CommandPalette open={paletteOpen} query={paletteQuery} onQuery={setPaletteQuery} onClose={() => setPaletteOpen(false)}>{paletteItems.length ? <List>{paletteItems.map((item) => <ListItem key={`${item.type}-${item.id}`}><Badge>{item.type}</Badge><button className="lt-button ghost" style={{ flex: 1, justifyContent: "flex-start" }} onClick={() => { navigate(item.path); setPaletteOpen(false); setPaletteQuery(""); }}>{item.title}</button></ListItem>)}</List> : <EmptyState title="没有结果" detail="输入页面、任务、笔记、阅读或财务关键词。" />}</CommandPalette>
    <QuickCapture open={quickOpen} onClose={() => setQuickOpen(false)} setState={setState} />
  </div>;
}
