import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { AuthApi, CloudDataStore, EMPTY_CLOUD_STATE, formatMoney, searchEntities, type CloudState, type WebSession } from "./core";
import { AuthScreen } from "./AuthScreen";
import { AccountsPage, BudgetsPage, CategoriesPage, FinanceOverview, ImportPage, TransactionsPage } from "./pages/FinancePages";
import { NotesPage } from "./pages/NotesPage";
import { ArticlesPage, EnglishStatsPage, VocabularyPage } from "./pages/EnglishPages";
import { DevicesPage } from "./pages/DevicesPage";
import { Empty, Metric, Notice, PageStack, Panel, currentRoute, entities, navigate, number, text, type Route } from "./ui";

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
  const storeRef = useRef<CloudDataStore | null>(null);

  useEffect(() => {
    const routeChanged = () => setRoute(currentRoute());
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
  }, [session?.user.id, session?.session.deviceId]);

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
      storeRef.current?.reset();
      storeRef.current = null;
      setSession(null);
      setState(EMPTY_CLOUD_STATE);
    }
  }

  if (authLoading) return <Centered>正在验证云端会话…</Centered>;
  if (!online && !session) return <OfflineScreen />;
  if (!session) return <AuthScreen auth={auth} error={error} onAuthenticated={setSession} />;

  const common = { session, state, privacy, online, run };
  return <div className={`app-shell ${privacy ? "privacy-on" : ""}`}>
    <Sidebar route={route} session={session} onLogout={() => void logout()} />
    <main className="main-panel">
      <Header route={route} online={online} loading={loading} privacy={privacy} onPrivacy={() => setPrivacy((value) => !value)} onRefresh={() => void refresh()} />
      {!online && <Notice kind="error">网页端需要联网使用。当前页面数据仅存在于内存中，所有写操作均已禁用。</Notice>}
      {error && <Notice kind="error">{error}</Notice>}
      {state.conflicts.length > 0 && <Notice kind="warning">检测到跨设备版本冲突，已经显示服务器最新版本，请检查后重新提交。</Notice>}
      <section className="content-area">
        {route === "/" && <Dashboard state={state} privacy={privacy} />}
        {route === "/search" && <SearchPage state={state} />}
        {route === "/devices" && <DevicesPage session={session} auth={auth} online={online} />}
        {route === "/finance" && <FinanceOverview {...common} />}
        {route === "/finance/transactions" && <TransactionsPage {...common} />}
        {route === "/finance/accounts" && <AccountsPage {...common} />}
        {route === "/finance/categories" && <CategoriesPage {...common} />}
        {route === "/finance/budgets" && <BudgetsPage {...common} />}
        {route === "/finance/import" && <ImportPage {...common} />}
        {route === "/notes" && <NotesPage {...common} />}
        {route === "/english/articles" && <ArticlesPage {...common} />}
        {route === "/english/vocabulary" && <VocabularyPage {...common} />}
        {route === "/english/stats" && <EnglishStatsPage state={state} />}
      </section>
    </main>
    <MobileNav route={route} />
  </div>;
}

function Dashboard({ state, privacy }: { state: CloudState; privacy: boolean }) {
  const transactions = entities(state, "finance.transaction");
  const notes = entities(state, "note.note");
  const vocabulary = entities(state, "english.vocabulary");
  const records = entities(state, "english.learning_record");
  const month = new Date().toISOString().slice(0, 7);
  const expense = transactions.filter((item) => text(item, "localDate").startsWith(month) && ["expense", "fee"].includes(text(item, "transactionType")) && text(item, "status") !== "ignored").reduce((sum, item) => sum + number(item, "amountCents"), 0);
  const timeline = [
    ...transactions.map((item) => ({ item, type: "财务", title: text(item, "merchant") || text(item, "note") || "财务流水" })),
    ...notes.map((item) => ({ item, type: "笔记", title: text(item, "title") || "无标题笔记" })),
    ...vocabulary.map((item) => ({ item, type: "英语", title: text(item, "displayWord") })),
  ].sort((a, b) => b.item.meta.updatedAt.localeCompare(a.item.meta.updatedAt)).slice(0, 10);
  return <PageStack><section className="hero-card"><div><p className="eyebrow">LIFETRACE CLOUD</p><h2>所有 Web 修改都由云端确认。</h2><p>浏览器不保存业务数据库，也不保留离线待同步队列。</p></div><div className="hero-number"><strong>{state.lastLoadedAt ? "✓" : "…"}</strong><span>{state.lastLoadedAt ? "云端已加载" : "正在加载"}</span></div></section><div className="metric-grid"><Metric label="本月支出" value={formatMoney(expense, "CNY", privacy)} detail={`${transactions.length} 条账单`} /><Metric label="笔记" value={String(notes.length)} detail={`${notes.filter((item) => item.isPinned === true).length} 条置顶`} /><Metric label="生词" value={String(vocabulary.length)} detail={`${vocabulary.filter((item) => text(item, "status") === "MASTERED").length} 个掌握`} /><Metric label="阅读" value={String(records.length)} detail="已提交总结" /></div><Panel title="最近动态" eyebrow="TIMELINE"><div className="timeline">{timeline.map(({ item, type, title }) => <div className="timeline-item" key={`${type}-${item.meta.id}`}><span>{type}</span><div><strong>{title}</strong><small>{new Date(item.meta.updatedAt).toLocaleString("zh-CN")}</small></div></div>)}{!timeline.length && <Empty title="暂无数据" description="创建第一条记录后会显示跨模块动态。" />}</div></Panel></PageStack>;
}

function SearchPage({ state }: { state: CloudState }) {
  const [query, setQuery] = useState("");
  const hits = searchEntities(state, query);
  return <PageStack><Panel title="全局搜索" eyebrow="SEARCH"><div className="search-box"><span>⌕</span><input autoFocus value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索笔记、账单、文章和生词" /></div></Panel><div className="search-results">{hits.map((hit) => <button key={`${hit.entityType}-${hit.id}`} onClick={() => navigate(hit.route as Route)}><span>{hit.entityType}</span><div><strong>{hit.title}</strong><p>{hit.subtitle.slice(0, 180)}</p><small>{new Date(hit.updatedAt).toLocaleString("zh-CN")}</small></div></button>)}{query && !hits.length && <Empty title="没有搜索结果" description="尝试其他关键词。" />}</div></PageStack>;
}

function Header({ route, online, loading, privacy, onPrivacy, onRefresh }: { route: Route; online: boolean; loading: boolean; privacy: boolean; onPrivacy: () => void; onRefresh: () => void }) {
  const titles: Record<Route, string> = { "/": "今日概览", "/search": "全局搜索", "/devices": "设备与会话", "/finance": "财务概览", "/finance/transactions": "账单列表", "/finance/accounts": "资金账户", "/finance/categories": "收支分类", "/finance/budgets": "预算管理", "/finance/import": "账单导入与对账", "/notes": "笔记空间", "/english/articles": "英语阅读", "/english/vocabulary": "生词本", "/english/stats": "学习统计" };
  return <header className="topbar"><div><p className="eyebrow">WEB / CLOUD</p><h1>{titles[route]}</h1></div><div className="sync-cluster"><span className={`status-dot ${online ? "online" : "offline"}`}>{online ? "在线" : "离线"}</span><button className={`secondary-button ${privacy ? "active" : ""}`} onClick={onPrivacy}>{privacy ? "退出隐私模式" : "隐私模式"}</button><button className="secondary-button" onClick={() => navigate("/search")}>搜索</button><button className="secondary-button" disabled={!online || loading} onClick={onRefresh}>{loading ? "处理中…" : "刷新云端"}</button></div></header>;
}

function Sidebar({ route, session, onLogout }: { route: Route; session: WebSession; onLogout: () => void }) {
  return <aside className="sidebar"><button className="brand" onClick={() => navigate("/")}><span className="brand-mark">L</span><span><strong>LifeTrace</strong><small>Cloud Web</small></span></button><nav><Nav active={route === "/"} icon="◫" label="概览" route="/" /><Nav active={route.startsWith("/finance")} icon="¥" label="财务" route="/finance" /><Nav active={route === "/notes"} icon="✎" label="笔记" route="/notes" /><Nav active={route.startsWith("/english")} icon="A" label="英语" route="/english/articles" /><Nav active={route === "/devices"} icon="◇" label="设备" route="/devices" /></nav><div className="sidebar-footer"><div className="user-chip"><span>{(session.user.displayName || session.user.email)[0]?.toUpperCase()}</span><div><strong>{session.user.displayName || "LifeTrace 用户"}</strong><small>{session.user.email}</small></div></div><button className="link-button" onClick={onLogout}>退出登录</button></div></aside>;
}

function MobileNav({ route }: { route: Route }) {
  return <nav className="mobile-nav"><Nav active={route === "/"} icon="◫" label="概览" route="/" /><Nav active={route.startsWith("/finance")} icon="¥" label="财务" route="/finance" /><Nav active={route === "/notes"} icon="✎" label="笔记" route="/notes" /><Nav active={route.startsWith("/english")} icon="A" label="英语" route="/english/articles" /></nav>;
}

function Nav({ active, icon, label, route }: { active: boolean; icon: string; label: string; route: Route }) {
  return <button className={`nav-button ${active ? "active" : ""}`} onClick={() => navigate(route)}><span>{icon}</span><b>{label}</b></button>;
}

function OfflineScreen() {
  return <div className="center-screen"><span className="offline-symbol">×</span><h1>需要网络连接</h1><p>LifeTrace Web 不在浏览器保存业务数据库，请联网后重新加载。</p><button className="primary-button" onClick={() => window.location.reload()}>重新加载</button></div>;
}

function Centered({ children }: { children: ReactNode }) {
  return <div className="center-screen"><div className="loader" /><p>{children}</p></div>;
}
