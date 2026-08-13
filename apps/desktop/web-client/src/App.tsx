import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import {
  BarChart3, BookOpen, Bot, CalendarDays, Check, ChevronRight, CircleDollarSign,
  Cloud, Dumbbell, FileUp, Home, Languages, Menu, NotebookPen, RefreshCw, Search,
  Settings, ShieldCheck, Smartphone, WalletCards,
} from "lucide-react";
import { AuthApi, CloudDataStore, EMPTY_CLOUD_STATE, formatMoney, searchEntities, type CloudState, type JsonEntity, type WebSession } from "./core";
import { AuthScreen } from "./AuthScreen";
import { AccountsPage, BudgetsPage, CategoriesPage, FinanceOverview, ImportPage, TransactionsPage } from "./pages/FinancePages";
import { BeeCountFinancePage } from "./pages/BeeCountFinancePage";
import { NotesPage } from "./pages/NotesPage";
import { ArticlesPage, EnglishStatsPage, VocabularyPage } from "./pages/EnglishPages";
import { DevicesPage } from "./pages/DevicesPage";
import { AssistantPage, CalendarPage, FitnessPage, HabitsPage, ReviewPage, SettingsPage } from "./pages/GrowthPages";
import { NAV_GROUPS, PAGE_COPY, SECONDARY_NAV, currentRoute, navigate, routeIsActive, type NavItem, type Route } from "./navigation";
import { Empty, Metric, Notice, Panel, entities, number, text } from "./ui";

const ICONS: Record<NavItem["icon"], typeof Home> = {
  home: Home, bot: Bot, check: Check, languages: Languages, dumbbell: Dumbbell,
  note: NotebookPen, calendar: CalendarDays, review: BookOpen, chart: BarChart3,
  money: CircleDollarSign, wallet: WalletCards, upload: FileUp, devices: Smartphone,
  cloud: Cloud, settings: Settings, search: Search,
};

export default function App() {
  const auth = useMemo(() => new AuthApi(), []);
  const [session, setSession] = useState<WebSession | null>(null);
  const [route, setRoute] = useState<Route>(currentRoute());
  const [state, setState] = useState<CloudState>(EMPTY_CLOUD_STATE);
  const [authLoading, setAuthLoading] = useState(true);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [online, setOnline] = useState(navigator.onLine);
  const [privacy, setPrivacy] = useState(false);
  const [menu, setMenu] = useState(false);
  const storeRef = useRef<CloudDataStore | null>(null);

  useEffect(() => {
    const routeChanged = () => { setRoute(currentRoute()); setMenu(false); window.scrollTo({ top: 0, behavior: "smooth" }); };
    const wentOnline = () => setOnline(true);
    const wentOffline = () => setOnline(false);
    window.addEventListener("popstate", routeChanged);
    window.addEventListener("online", wentOnline);
    window.addEventListener("offline", wentOffline);
    return () => {
      window.removeEventListener("popstate", routeChanged);
      window.removeEventListener("online", wentOnline);
      window.removeEventListener("offline", wentOffline);
    };
  }, []);

  useEffect(() => {
    let active = true;
    if (!navigator.onLine) { setAuthLoading(false); return; }
    auth.session()
      .then((value) => { if (active) setSession(value); })
      .catch((cause: unknown) => {
        if (active && cause instanceof Error && !/401|unauth|authentication|session/i.test(cause.message)) setError(cause.message);
      })
      .finally(() => { if (active) setAuthLoading(false); });
    return () => { active = false; };
  }, [auth]);

  useEffect(() => {
    if (!session) {
      storeRef.current = null;
      setState(EMPTY_CLOUD_STATE);
      return;
    }
    const store = new CloudDataStore(session.user.id, session.session.deviceId, session.csrfToken);
    storeRef.current = store;
    setLoading(true); setError("");
    store.load().then(setState).catch((cause: unknown) => setError(cause instanceof Error ? cause.message : "无法加载云端数据")).finally(() => setLoading(false));
  }, [session?.user.id, session?.session.deviceId, session?.csrfToken]);

  useEffect(() => {
    const preference = entities(state, "user.preference").find((item) => text(item, "preferenceKey") === "appearance.theme");
    document.documentElement.dataset.theme = preference?.value === "dark" ? "dark" : "light";
  }, [state]);

  const refresh = useCallback(async () => {
    if (!storeRef.current || !navigator.onLine) return;
    setLoading(true); setError("");
    try { setState(await storeRef.current.refresh()); }
    catch (cause) { setError(cause instanceof Error ? cause.message : "刷新失败"); }
    finally { setLoading(false); }
  }, []);

  const run = useCallback(async (action: (store: CloudDataStore) => Promise<CloudState>) => {
    const store = storeRef.current;
    if (!store) throw new Error("云端数据服务尚未就绪");
    if (!navigator.onLine) throw new Error("当前无网络，数据未保存");
    setLoading(true); setError("");
    try {
      const next = await action(store);
      setState(next);
      return next;
    } catch (cause) {
      setState(store.snapshot());
      setError(cause instanceof Error ? cause.message : "云端操作失败");
      throw cause;
    } finally { setLoading(false); }
  }, []);

  async function logout() {
    try { if (navigator.onLine) await auth.logout(session?.csrfToken); }
    finally {
      storeRef.current?.reset(); storeRef.current = null; setSession(null); setState(EMPTY_CLOUD_STATE);
      navigate("/");
    }
  }

  if (authLoading) return <Centered>正在验证云端会话…</Centered>;
  if (!online && !session) return <OfflineScreen />;
  if (!session) return <AuthScreen auth={auth} error={error} onAuthenticated={setSession} />;

  const common = { session, state, privacy, online, run };
  const [title, subtitle] = PAGE_COPY[route];

  return <main className={`hx-shell browser-shell ${privacy ? "privacy-on" : ""}`}>
    <aside className={menu ? "open" : ""} aria-label="主导航">
      <button className="hx-brand" onClick={() => navigate("/")}><span>LT</span><div><strong>Life trace</strong><small>个人管理系统</small></div></button>
      <nav>{NAV_GROUPS.map((group) => <div key={group.label}><label>{group.label}</label>{group.items.map((item) => <NavButton current={route} item={item} key={item.route} />)}</div>)}</nav>
      <div className="hx-sidebar-foot">
        <div><i className={online ? "online" : "offline"} /><strong>{online ? "云端在线" : "网络离线"}</strong><p>浏览器数据直接写入 LifeTrace Cloud。</p></div>
        {SECONDARY_NAV.map((item) => <NavButton current={route} item={item} key={item.route} />)}
        <section><span>{(session.user.displayName || session.user.email)[0]?.toUpperCase()}</span><div><strong>{session.user.displayName || "个人空间"}</strong><small>{session.user.email}</small></div></section>
        <button className="hx-logout" onClick={() => void logout()}>退出登录</button>
      </div>
    </aside>
    {menu && <button className="hx-nav-scrim" aria-label="关闭导航" onClick={() => setMenu(false)} />}
    <div className="hx-main">
      <header className="hx-topbar">
        <button className="hx-menu" aria-label={menu ? "关闭导航" : "打开导航"} aria-expanded={menu} onClick={() => setMenu(!menu)}><Menu /></button>
        <div><span className="hx-kicker">{new Intl.DateTimeFormat("zh-CN", { month: "long", day: "numeric", weekday: "long" }).format(new Date())}</span><h1>{title}</h1><p>{subtitle}</p></div>
        <div className="hx-top-actions"><span className={`hx-status ${online ? "success" : "warning"}`}>{online ? "云端在线" : "离线"}</span><button className="hx-icon-btn" aria-label="全局搜索" onClick={() => navigate("/search")}><Search /></button><button className={`hx-btn secondary ${privacy ? "active" : ""}`} onClick={() => setPrivacy((value) => !value)}>{privacy ? "退出隐私模式" : "隐私模式"}</button><button className="hx-btn secondary" disabled={!online || loading} onClick={() => void refresh()}><RefreshCw className={loading ? "spin" : ""} />{loading ? "同步中" : "刷新"}</button></div>
      </header>
      <div className="browser-notices">
        {!online && <Notice kind="warning">网页端需要联网使用；离线期间所有写操作均已禁用。</Notice>}
        {error && <Notice kind="error">{error}</Notice>}
        {state.conflicts.length > 0 && <Notice kind="warning">检测到 {state.conflicts.length} 个跨设备版本冲突，页面已显示服务器最新版本。</Notice>}
      </div>
      <section className="hx-page-content">
        {route === "/" && <Dashboard state={state} privacy={privacy} />}
        {route === "/assistant" && <AssistantPage {...common} />}
        {route === "/habits" && <HabitsPage {...common} />}
        {route === "/fitness" && <FitnessPage {...common} />}
        {route === "/calendar" && <CalendarPage {...common} />}
        {route === "/review" && <ReviewPage {...common} />}
        {route === "/search" && <SearchPage state={state} />}
        {route === "/devices" && <DevicesPage session={session} auth={auth} online={online} />}
        {route === "/settings" && <SettingsPage {...common} />}
        {route === "/finance" && <FinanceOverview {...common} />}
        {route === "/finance/transactions" && <TransactionsPage {...common} />}
        {route === "/finance/accounts" && <AccountsPage {...common} />}
        {route === "/finance/categories" && <CategoriesPage {...common} />}
        {route === "/finance/budgets" && <BudgetsPage {...common} />}
        {route === "/finance/import" && <ImportPage {...common} />}
        {route === "/finance/beecount" && <BeeCountFinancePage privacy={privacy} online={online} />}
        {route === "/notes" && <NotesPage {...common} />}
        {route === "/english/articles" && <ArticlesPage {...common} />}
        {route === "/english/vocabulary" && <VocabularyPage {...common} />}
        {route === "/english/stats" && <EnglishStatsPage state={state} />}
      </section>
      <MobileNav route={route} />
    </div>
  </main>;
}

function NavButton({ current, item }: { current: Route; item: NavItem }) {
  const Icon = ICONS[item.icon]; const active = routeIsActive(current, item.route);
  return <button className={active ? "active" : ""} aria-current={active ? "page" : undefined} onClick={() => navigate(item.route)}><span><Icon />{item.label}</span><ChevronRight /></button>;
}

function Dashboard({ state, privacy }: { state: CloudState; privacy: boolean }) {
  const activities = entities(state, "habit.activity").filter((item) => item.isArchived !== true);
  const logs = entities(state, "habit.log");
  const transactions = entities(state, "finance.transaction");
  const accounts = entities(state, "finance.account").filter((item) => item.isArchived !== true);
  const workouts = entities(state, "workout.workout");
  const notes = entities(state, "note.note").filter((item) => item.isArchived !== true);
  const records = entities(state, "english.learning_record");
  const reviews = entities(state, "review.daily");
  const today = new Date();
  const todayKey = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, "0")}-${String(today.getDate()).padStart(2, "0")}`;
  const month = todayKey.slice(0, 7);
  const todayLogs = logs.filter((item) => text(item, "logDate") === todayKey && item.status !== "skipped");
  const completed = new Set(todayLogs.map((item) => text(item, "activityId"))).size;
  const monthExpense = transactions.filter((item) => text(item, "localDate").startsWith(month) && ["expense", "fee"].includes(text(item, "transactionType")) && text(item, "status") !== "ignored").reduce((sum, item) => sum + number(item, "amountCents"), 0);
  const assets = accounts.reduce((sum, item) => sum + number(item, "openingBalanceCents"), 0);
  const weekWorkouts = workouts.filter((item) => Date.now() - new Date(text(item, "occurredAt")).getTime() < 7 * 86400000);
  const timeline = buildTimeline(state).slice(0, 12);

  return <div className="hx-view">
    <div className="hx-hero-grid"><article className="hx-hero-dark"><span className="hx-pill">今日</span><h2>今天，从最重要的一小步开始。</h2><p>你的坚持、训练、英语、消费和复盘会在这里形成统一反馈。</p><div className="hx-hero-progress"><div><strong>{completed} / {activities.length}</strong><small>今日坚持</small></div><i><b style={{ width: `${activities.length ? completed / activities.length * 100 : 0}%` }} /></i></div></article><article className="hx-quote"><span>“</span><h3>不要打断两次。</h3><p>允许偶尔错过，但下一次按计划回来。</p></article></div>
    <div className="hx-metrics"><Metric label="今日完成" value={`${completed} 项`} detail={`还有 ${Math.max(activities.length - completed, 0)} 项等待完成`} /><Metric label="本周训练" value={`${weekWorkouts.length} 次`} detail="训练记录已云端同步" positive /><Metric label="本月支出" value={formatMoney(monthExpense, "CNY", privacy)} detail={`${transactions.filter((item) => text(item, "localDate").startsWith(month)).length} 笔收支记录`} /><Metric label="账户基准资产" value={formatMoney(assets, "CNY", privacy)} detail={`${accounts.length} 个账户`} positive /></div>
    <div className="hx-dashboard-grid">
      <Panel eyebrow="TODAY" title="今天的坚持"><div className="hx-list">{activities.slice(0, 6).map((activity) => { const done = todayLogs.some((item) => text(item, "activityId") === activity.meta.id); return <button className="hx-row" key={activity.meta.id} onClick={() => navigate("/habits")}><span className="hx-row-icon">{text(activity, "icon") || text(activity, "name").slice(0, 1)}</span><div><strong>{text(activity, "name")}</strong><small>{done ? "今天已记录" : `目标 ${number(activity, "normalTarget") || 1} ${text(activity, "unit")}`}</small></div><b className={done ? "positive" : ""}>{done ? "完成" : "待完成"}</b></button>; })}{!activities.length && <Empty title="暂无坚持项目" description="创建第一个长期项目后会显示在这里。" />}</div></Panel>
      <Panel eyebrow="RECENT" title="最近动态"><div className="hx-list">{timeline.map((item) => <article className="hx-row" key={`${item.type}-${item.id}`}><span className="hx-row-icon">{item.type.slice(0, 1)}</span><div><strong>{item.title}</strong><small>{item.type} · {new Date(item.updatedAt).toLocaleString("zh-CN")}</small></div></article>)}{!timeline.length && <Empty title="暂无动态" description="新增记录后会形成跨模块时间线。" />}</div></Panel>
      <Panel eyebrow="KNOWLEDGE" title="笔记与英语"><div className="hx-metrics compact"><Metric label="笔记" value={String(notes.length)} detail={`${notes.filter((item) => item.isPinned === true).length} 条置顶`} /><Metric label="阅读" value={String(records.length)} detail="已提交总结" /><Metric label="复盘" value={String(reviews.length)} detail="生活记录" /></div><div className="hx-inline-actions"><button className="hx-btn secondary" onClick={() => navigate("/notes")}>打开笔记</button><button className="hx-btn secondary" onClick={() => navigate("/english/articles")}>每日英语</button><button className="hx-btn secondary" onClick={() => navigate("/review")}>今日复盘</button></div></Panel>
      <Panel eyebrow="CLOUD" title="云端状态"><div className="hx-setting-row"><div><strong>{state.lastLoadedAt ? "数据已加载" : "正在等待数据"}</strong><small>{state.lastLoadedAt ? `最后加载：${new Date(state.lastLoadedAt).toLocaleString("zh-CN")}` : "登录后自动加载完整快照"}</small></div><ShieldCheck /></div><p className="hx-muted">浏览器不保存业务数据库。相册、私密相册和局域网照片同步仅保留在桌面应用。</p></Panel>
    </div>
  </div>;
}

function buildTimeline(state: CloudState): Array<{ id: string; type: string; title: string; updatedAt: string }> {
  const values: Array<{ id: string; type: string; title: string; updatedAt: string }> = [];
  const add = (items: JsonEntity[], type: string, title: (item: JsonEntity) => string) => items.forEach((item) => values.push({ id: item.meta.id, type, title: title(item), updatedAt: item.meta.updatedAt }));
  add(entities(state, "habit.log"), "坚持", () => "完成坚持记录");
  add(entities(state, "workout.workout"), "训练", (item) => text(item, "name") || "训练记录");
  add(entities(state, "finance.transaction"), "财务", (item) => text(item, "merchant") || text(item, "counterparty") || "财务流水");
  add(entities(state, "note.note"), "笔记", (item) => text(item, "title") || "无标题笔记");
  add(entities(state, "english.learning_record"), "英语", () => "完成英语阅读");
  add(entities(state, "review.daily"), "复盘", (item) => `${text(item, "reviewDate")} 每日复盘`);
  return values.sort((left, right) => right.updatedAt.localeCompare(left.updatedAt));
}

function SearchPage({ state }: { state: CloudState }) {
  const [query, setQuery] = useState("");
  const hits = searchEntities(state, query);
  return <div className="hx-view"><Panel eyebrow="SEARCH" title="全局搜索"><div className="hx-search-box"><Search /><input autoFocus value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索坚持、训练、账单、笔记、文章、生词和复盘" /></div></Panel><div className="hx-search-results">{hits.map((hit) => <button key={`${hit.entityType}-${hit.id}`} onClick={() => navigate(hit.route as Route)}><span>{hit.entityType}</span><div><strong>{hit.title}</strong><p>{hit.subtitle.slice(0, 180)}</p><small>{new Date(hit.updatedAt).toLocaleString("zh-CN")}</small></div></button>)}{query && !hits.length && <Empty title="没有搜索结果" description="尝试其他关键词。" />}</div></div>;
}

function MobileNav({ route }: { route: Route }) {
  const items: NavItem[] = [NAV_GROUPS[0]!.items[0]!, NAV_GROUPS[1]!.items[0]!, NAV_GROUPS[1]!.items[2]!, NAV_GROUPS[2]!.items[0]!, SECONDARY_NAV[1]!];
  return <nav className="browser-mobile-nav">{items.map((item) => { const Icon = ICONS[item.icon]; return <button className={routeIsActive(route, item.route) ? "active" : ""} key={item.route} onClick={() => navigate(item.route)}><Icon /><span>{item.label}</span></button>; })}</nav>;
}

function OfflineScreen() {
  return <div className="hx-loading"><span>×</span><h1>需要网络连接</h1><p>LifeTrace 浏览器端不在本地保存业务数据库，请联网后重新加载。</p><button className="hx-btn primary" onClick={() => window.location.reload()}>重新加载</button></div>;
}

function Centered({ children }: { children: ReactNode }) {
  return <div className="hx-loading"><span>LT</span><p>{children}</p></div>;
}
