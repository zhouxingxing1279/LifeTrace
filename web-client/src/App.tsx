import { FormEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  AuthApi,
  AuthSession,
  AuthUser,
  PersistedState,
  SyncEntity,
  WebSession,
  WebSyncStore,
  createFinanceAccount,
  createNote,
  createTransaction,
  createVocabulary,
  formatMoney,
  getOrCreateDeviceId,
} from "./core";

type View = "overview" | "finance" | "notes" | "english";

const SESSION_CACHE_KEY = "lifetrace:web:session-cache";

interface CachedSession {
  user: AuthUser;
  session: AuthSession;
  cachedAt: string;
}

function readCachedSession(): WebSession | null {
  try {
    const raw = localStorage.getItem(SESSION_CACHE_KEY);
    if (!raw) return null;
    const cached = JSON.parse(raw) as CachedSession;
    if (cached.session.publicDevice) {
      localStorage.removeItem(SESSION_CACHE_KEY);
      return null;
    }
    return { user: cached.user, session: cached.session, csrfToken: "" };
  } catch {
    return null;
  }
}

function cacheSession(value: WebSession): void {
  if (value.session.publicDevice) {
    localStorage.removeItem(SESSION_CACHE_KEY);
    return;
  }
  localStorage.setItem(
    SESSION_CACHE_KEY,
    JSON.stringify({ user: value.user, session: value.session, cachedAt: new Date().toISOString() }),
  );
}

function entityText(entity: SyncEntity, key: string): string {
  const value = entity[key];
  return typeof value === "string" ? value : "";
}

function entityNumber(entity: SyncEntity, key: string): number {
  const value = entity[key];
  return typeof value === "number" ? value : 0;
}

export default function App() {
  const auth = useMemo(() => new AuthApi(), []);
  const [session, setSession] = useState<WebSession | null>(null);
  const [authLoading, setAuthLoading] = useState(true);
  const [authError, setAuthError] = useState("");
  const [view, setView] = useState<View>("overview");
  const [online, setOnline] = useState(navigator.onLine);
  const [syncing, setSyncing] = useState(false);
  const [syncError, setSyncError] = useState("");
  const [state, setState] = useState<PersistedState>({
    cursor: null,
    entities: {},
    outbox: [],
    conflicts: [],
    lastSyncedAt: null,
  });
  const storeRef = useRef<WebSyncStore | null>(null);

  useEffect(() => {
    let active = true;
    auth
      .session()
      .then((value) => {
        if (!active) return;
        cacheSession(value);
        setSession(value);
      })
      .catch((error: unknown) => {
        if (!active) return;
        if (!navigator.onLine) {
          const cached = readCachedSession();
          if (cached) setSession(cached);
        } else if (error instanceof Error && !/401|unauth|session/i.test(error.message)) {
          setAuthError(error.message);
        }
      })
      .finally(() => active && setAuthLoading(false));
    return () => {
      active = false;
    };
  }, [auth]);

  useEffect(() => {
    const handleOnline = () => setOnline(true);
    const handleOffline = () => setOnline(false);
    window.addEventListener("online", handleOnline);
    window.addEventListener("offline", handleOffline);
    return () => {
      window.removeEventListener("online", handleOnline);
      window.removeEventListener("offline", handleOffline);
    };
  }, []);

  useEffect(() => {
    if (!session) {
      storeRef.current = null;
      return;
    }
    const storage = session.session.publicDevice ? sessionStorage : localStorage;
    const deviceId = getOrCreateDeviceId(storage);
    const store = new WebSyncStore(session.user.id, deviceId, storage);
    store.setCsrfToken(session.csrfToken);
    storeRef.current = store;
    setState(store.snapshot());
  }, [session]);

  const refresh = useCallback(() => {
    if (storeRef.current) setState(storeRef.current.snapshot());
  }, []);

  const synchronize = useCallback(async () => {
    const store = storeRef.current;
    if (!store || !navigator.onLine || syncing) return;
    setSyncing(true);
    setSyncError("");
    try {
      if (!session?.csrfToken) {
        const current = await auth.session();
        cacheSession(current);
        store.setCsrfToken(current.csrfToken);
        setSession(current);
      }
      const next = await store.sync();
      setState(next);
    } catch (error) {
      setSyncError(error instanceof Error ? error.message : "同步失败");
    } finally {
      setSyncing(false);
    }
  }, [auth, session?.csrfToken, syncing]);

  useEffect(() => {
    if (session && online) void synchronize();
  }, [session?.user.id, online]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if ("serviceWorker" in navigator) {
      void navigator.serviceWorker.register("/sw.js");
    }
  }, []);

  async function handleLogin(email: string, password: string, publicDevice: boolean) {
    setAuthError("");
    const value = await auth.login(email, password, publicDevice);
    cacheSession(value);
    setSession(value);
  }

  async function handleLogout() {
    setAuthError("");
    try {
      if (navigator.onLine) await auth.logout(session?.csrfToken);
    } finally {
      localStorage.removeItem(SESSION_CACHE_KEY);
      storeRef.current?.clear();
      setSession(null);
      setState({ cursor: null, entities: {}, outbox: [], conflicts: [], lastSyncedAt: null });
    }
  }

  if (authLoading) return <LoadingScreen />;
  if (!session) return <LoginScreen error={authError} onLogin={handleLogin} />;

  const activeStorage = session.session.publicDevice ? sessionStorage : localStorage;
  const accounts = Object.values(state.entities["finance.account"] ?? {});
  const transactions = Object.values(state.entities["finance.transaction"] ?? {});
  const notes = Object.values(state.entities["note.note"] ?? {});
  const articles = Object.values(state.entities["english.article"] ?? {});
  const vocabulary = Object.values(state.entities["english.vocabulary"] ?? {});

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand"><span className="brand-mark">L</span><div><strong>LifeTrace</strong><small>个人管理平台</small></div></div>
        <nav aria-label="主导航">
          <NavButton active={view === "overview"} icon="◫" label="概览" onClick={() => setView("overview")} />
          <NavButton active={view === "finance"} icon="¥" label="财务" onClick={() => setView("finance")} />
          <NavButton active={view === "notes"} icon="✎" label="笔记" onClick={() => setView("notes")} />
          <NavButton active={view === "english"} icon="A" label="英语" onClick={() => setView("english")} />
        </nav>
        <div className="sidebar-footer">
          <div className="user-chip"><span>{(session.user.displayName || session.user.email).slice(0, 1).toUpperCase()}</span><div><strong>{session.user.displayName || "LifeTrace 用户"}</strong><small>{session.user.email}</small></div></div>
          <button className="text-button" onClick={() => void handleLogout()}>退出登录</button>
        </div>
      </aside>

      <main className="main-panel">
        <header className="topbar">
          <div>
            <p className="eyebrow">WEB / PWA</p>
            <h1>{viewTitle(view)}</h1>
          </div>
          <div className="sync-cluster">
            <span className={`status-dot ${online ? "online" : "offline"}`}>{online ? "在线" : "离线"}</span>
            {state.outbox.length > 0 && <span className="pending-badge">待同步 {state.outbox.length}</span>}
            <button className="secondary-button" disabled={!online || syncing} onClick={() => void synchronize()}>
              {syncing ? "同步中…" : "立即同步"}
            </button>
          </div>
        </header>

        {!online && <div className="notice warning">当前处于离线模式。浏览与编辑会保存在本机，恢复联网后自动同步。</div>}
        {syncError && <div className="notice error">{syncError}</div>}
        {state.conflicts.length > 0 && <div className="notice neutral">最近有 {state.conflicts.length} 个版本冲突，已采用服务器版本并保留记录。</div>}

        <section className="content-area">
          {view === "overview" && <Overview transactions={transactions} notes={notes} vocabulary={vocabulary} state={state} />}
          {view === "finance" && (
            <FinanceView
              userId={session.user.id}
              deviceId={getOrCreateDeviceId(activeStorage)}
              accounts={accounts}
              transactions={transactions}
              onChanged={refresh}
              store={storeRef.current}
            />
          )}
          {view === "notes" && (
            <NotesView
              userId={session.user.id}
              deviceId={getOrCreateDeviceId(activeStorage)}
              notes={notes}
              onChanged={refresh}
              store={storeRef.current}
            />
          )}
          {view === "english" && (
            <EnglishView
              userId={session.user.id}
              deviceId={getOrCreateDeviceId(activeStorage)}
              articles={articles}
              vocabulary={vocabulary}
              onChanged={refresh}
              store={storeRef.current}
            />
          )}
        </section>
      </main>

      <nav className="mobile-nav" aria-label="移动端导航">
        <NavButton active={view === "overview"} icon="◫" label="概览" onClick={() => setView("overview")} />
        <NavButton active={view === "finance"} icon="¥" label="财务" onClick={() => setView("finance")} />
        <NavButton active={view === "notes"} icon="✎" label="笔记" onClick={() => setView("notes")} />
        <NavButton active={view === "english"} icon="A" label="英语" onClick={() => setView("english")} />
      </nav>
    </div>
  );
}

function LoadingScreen() {
  return <div className="center-screen"><div className="loader" /><p>正在恢复 LifeTrace 会话…</p></div>;
}

function LoginScreen({ error, onLogin }: { error: string; onLogin: (email: string, password: string, publicDevice: boolean) => Promise<void> }) {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [publicDevice, setPublicDevice] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [localError, setLocalError] = useState("");

  async function submit(event: FormEvent) {
    event.preventDefault();
    setSubmitting(true);
    setLocalError("");
    try {
      await onLogin(email.trim(), password, publicDevice);
    } catch (cause) {
      setLocalError(cause instanceof Error ? cause.message : "登录失败");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="login-page">
      <section className="login-intro">
        <div className="brand large"><span className="brand-mark">L</span><div><strong>LifeTrace</strong><small>把每一天积累成长期变化</small></div></div>
        <div className="intro-copy"><p className="eyebrow">EPIC 13 · WEB CLIENT</p><h1>一个入口，查看你的财务、笔记与英语成长。</h1><p>数据通过 LifeTrace 云端协议与桌面端、移动端同步。断网时仍可使用核心功能。</p></div>
        <div className="feature-row"><span>离线可用</span><span>跨端同步</span><span>会话安全</span></div>
      </section>
      <section className="login-card-wrap">
        <form className="login-card" onSubmit={(event) => void submit(event)}>
          <p className="eyebrow">欢迎回来</p><h2>登录 LifeTrace</h2><p className="muted">使用已有云端账户继续。</p>
          <label>邮箱<input type="email" autoComplete="email" required value={email} onChange={(event) => setEmail(event.target.value)} placeholder="you@example.com" /></label>
          <label>密码<input type="password" autoComplete="current-password" required value={password} onChange={(event) => setPassword(event.target.value)} placeholder="输入密码" /></label>
          <label className="checkbox"><input type="checkbox" checked={publicDevice} onChange={(event) => setPublicDevice(event.target.checked)} /><span>这是公共设备（使用更短会话）</span></label>
          {(localError || error) && <div className="notice error">{localError || error}</div>}
          <button className="primary-button full" disabled={submitting}>{submitting ? "登录中…" : "登录"}</button>
          <small className="security-note">认证 Cookie 为 HttpOnly；退出后本机业务缓存会被清除。</small>
        </form>
      </section>
    </div>
  );
}

function NavButton({ active, icon, label, onClick }: { active: boolean; icon: string; label: string; onClick: () => void }) {
  return <button className={`nav-button ${active ? "active" : ""}`} onClick={onClick}><span>{icon}</span><b>{label}</b></button>;
}

function viewTitle(view: View): string {
  return { overview: "今日概览", finance: "财务记录", notes: "笔记空间", english: "英语学习" }[view];
}

function Overview({ transactions, notes, vocabulary, state }: { transactions: SyncEntity[]; notes: SyncEntity[]; vocabulary: SyncEntity[]; state: PersistedState }) {
  const month = new Date().toISOString().slice(0, 7);
  const expense = transactions.filter((item) => entityText(item, "transactionType") === "expense" && entityText(item, "localDate").startsWith(month)).reduce((sum, item) => sum + entityNumber(item, "amountCents"), 0);
  const mastered = vocabulary.filter((item) => entityText(item, "status") === "MASTERED").length;
  return <div className="page-stack">
    <div className="hero-card"><div><p className="eyebrow">你的数据正在持续积累</p><h2>今天也留下一点可回看的痕迹。</h2><p>Web 端采用同一同步协议，所有修改都会进入离线队列并最终汇入 LifeTrace。</p></div><div className="hero-number"><strong>{state.outbox.length}</strong><span>待同步修改</span></div></div>
    <div className="metric-grid">
      <Metric label="本月支出" value={formatMoney(expense)} detail={`${transactions.length} 条财务记录`} />
      <Metric label="笔记总数" value={String(notes.length)} detail={`${notes.filter((item) => item.isPinned === true).length} 条已置顶`} />
      <Metric label="词汇积累" value={String(vocabulary.length)} detail={`${mastered} 个已掌握`} />
      <Metric label="最近同步" value={state.lastSyncedAt ? new Date(state.lastSyncedAt).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" }) : "尚未"} detail={state.cursor ? `游标 ${state.cursor}` : "等待首次同步"} />
    </div>
    <div className="panel"><div className="panel-heading"><div><p className="eyebrow">最近动态</p><h3>跨模块时间线</h3></div></div><div className="timeline">
      {[...transactions.map((entity) => ({ entity, type: "财务", text: entityText(entity, "note") || entityText(entity, "transactionType") })), ...notes.map((entity) => ({ entity, type: "笔记", text: entityText(entity, "title") || entityText(entity, "summary") })), ...vocabulary.map((entity) => ({ entity, type: "英语", text: entityText(entity, "displayWord") }))]
        .sort((a, b) => b.entity.meta.updatedAt.localeCompare(a.entity.meta.updatedAt)).slice(0, 8).map((item) => <div className="timeline-item" key={`${item.type}-${item.entity.meta.id}`}><span>{item.type}</span><div><strong>{item.text || "未命名记录"}</strong><small>{new Date(item.entity.meta.updatedAt).toLocaleString("zh-CN")}</small></div></div>)}
      {transactions.length + notes.length + vocabulary.length === 0 && <EmptyState title="暂无数据" text="完成首次同步或创建一条记录后，这里会显示你的动态。" />}
    </div></div>
  </div>;
}

function Metric({ label, value, detail }: { label: string; value: string; detail: string }) {
  return <article className="metric-card"><span>{label}</span><strong>{value}</strong><small>{detail}</small></article>;
}

function FinanceView({ userId, deviceId, accounts, transactions, store, onChanged }: { userId: string; deviceId: string; accounts: SyncEntity[]; transactions: SyncEntity[]; store: WebSyncStore | null; onChanged: () => void }) {
  const [amount, setAmount] = useState("");
  const [note, setNote] = useState("");
  const [type, setType] = useState<"expense" | "income">("expense");
  const [accountId, setAccountId] = useState("");
  const [accountName, setAccountName] = useState("");
  const [error, setError] = useState("");

  function addAccount(event: FormEvent) {
    event.preventDefault();
    if (!store) return;
    store.queueUpsert("finance.account", createFinanceAccount(userId, deviceId, accountName));
    setAccountName(""); onChanged();
  }
  function addTransaction(event: FormEvent) {
    event.preventDefault(); if (!store) return; setError("");
    try {
      store.queueUpsert("finance.transaction", createTransaction(userId, deviceId, { accountId: accountId || null, amount, type, note }));
      setAmount(""); setNote(""); onChanged();
    } catch (cause) { setError(cause instanceof Error ? cause.message : "保存失败"); }
  }
  const sorted = [...transactions].sort((a, b) => entityText(b, "occurredAt").localeCompare(entityText(a, "occurredAt")));
  return <div className="two-column">
    <div className="page-stack">
      <div className="panel"><div className="panel-heading"><div><p className="eyebrow">新增流水</p><h3>快速记一笔</h3></div></div><form className="form-grid" onSubmit={addTransaction}>
        <div className="segmented"><button type="button" className={type === "expense" ? "active" : ""} onClick={() => setType("expense")}>支出</button><button type="button" className={type === "income" ? "active" : ""} onClick={() => setType("income")}>收入</button></div>
        <label>金额（元）<input inputMode="decimal" required value={amount} onChange={(event) => setAmount(event.target.value)} placeholder="0.00" /></label>
        <label>账户<select value={accountId} onChange={(event) => setAccountId(event.target.value)}><option value="">未指定账户</option>{accounts.map((account) => <option key={account.meta.id} value={account.meta.id}>{entityText(account, "name")}</option>)}</select></label>
        <label className="span-2">备注<input value={note} onChange={(event) => setNote(event.target.value)} placeholder="午餐、工资、交通…" /></label>
        {error && <div className="notice error span-2">{error}</div>}<button className="primary-button span-2">保存到离线队列</button>
      </form></div>
      <div className="panel"><div className="panel-heading"><div><p className="eyebrow">流水</p><h3>最近记录</h3></div><strong>{transactions.length}</strong></div><div className="record-list">
        {sorted.map((item) => <div className="record-row" key={item.meta.id}><div className={`record-icon ${entityText(item, "transactionType")}`}>{entityText(item, "transactionType") === "income" ? "+" : "−"}</div><div className="record-main"><strong>{entityText(item, "note") || (entityText(item, "transactionType") === "income" ? "收入" : "支出")}</strong><small>{entityText(item, "localDate")} · {accounts.find((a) => a.meta.id === item.accountId)?.name as string || "未指定账户"}</small></div><b className={entityText(item, "transactionType")}>{entityText(item, "transactionType") === "income" ? "+" : "−"}{formatMoney(entityNumber(item, "amountCents"), entityText(item, "currency") || "CNY")}</b><button className="icon-button danger" aria-label="删除流水" onClick={() => { store?.queueDelete("finance.transaction", item.meta.id); onChanged(); }}>×</button></div>)}
        {!sorted.length && <EmptyState title="还没有流水" text="新增记录后，即使离线也会立即显示。" />}
      </div></div>
    </div>
    <aside className="page-stack"><div className="panel"><div className="panel-heading"><div><p className="eyebrow">账户</p><h3>资金账户</h3></div></div><form className="inline-form" onSubmit={addAccount}><input value={accountName} onChange={(event) => setAccountName(event.target.value)} placeholder="例如：微信零钱" required /><button className="secondary-button">添加</button></form><div className="account-list">{accounts.map((account) => <div key={account.meta.id}><span className="account-dot" /><strong>{entityText(account, "name")}</strong><small>{entityText(account, "currency")}</small></div>)}{!accounts.length && <p className="muted compact">可先创建账户，也可以记录未指定账户的流水。</p>}</div></div></aside>
  </div>;
}

function NotesView({ userId, deviceId, notes, store, onChanged }: { userId: string; deviceId: string; notes: SyncEntity[]; store: WebSyncStore | null; onChanged: () => void }) {
  const [title, setTitle] = useState(""); const [content, setContent] = useState(""); const [editingId, setEditingId] = useState<string | null>(null); const [query, setQuery] = useState("");
  function save(event: FormEvent) { event.preventDefault(); if (!store) return; const existing = notes.find((item) => item.meta.id === editingId); const entity = existing ? { ...existing, title: title.trim() || null, contentText: content, contentMarkdown: content, contentHtml: `<p>${content.replace(/[&<>]/g, "")}</p>`, contentJson: { type: "doc", content }, summary: content.slice(0, 120), meta: { ...existing.meta } } : createNote(userId, deviceId, title, content); store.queueUpsert("note.note", entity); setTitle(""); setContent(""); setEditingId(null); onChanged(); }
  function edit(note: SyncEntity) { setEditingId(note.meta.id); setTitle(entityText(note, "title")); setContent(entityText(note, "contentMarkdown") || entityText(note, "contentText")); window.scrollTo({ top: 0, behavior: "smooth" }); }
  const filtered = notes.filter((note) => `${entityText(note, "title")} ${entityText(note, "contentText")}`.toLowerCase().includes(query.toLowerCase())).sort((a, b) => Number(b.isPinned === true) - Number(a.isPinned === true) || b.meta.updatedAt.localeCompare(a.meta.updatedAt));
  return <div className="two-column notes-layout"><div className="page-stack"><div className="panel sticky-editor"><div className="panel-heading"><div><p className="eyebrow">{editingId ? "编辑笔记" : "新建笔记"}</p><h3>{editingId ? "继续完善内容" : "捕捉当前想法"}</h3></div>{editingId && <button className="text-button" onClick={() => { setEditingId(null); setTitle(""); setContent(""); }}>取消编辑</button>}</div><form className="note-editor" onSubmit={save}><input value={title} onChange={(event) => setTitle(event.target.value)} placeholder="标题（可选）" /><textarea value={content} onChange={(event) => setContent(event.target.value)} placeholder="写下内容…" rows={10} required /><button className="primary-button">{editingId ? "保存修改" : "创建笔记"}</button></form></div></div><div className="page-stack"><div className="search-box"><span>⌕</span><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索标题或正文" /></div><div className="note-grid">{filtered.map((note) => <article className="note-card" key={note.meta.id}><div className="note-card-top"><span>{entityText(note, "noteType") || "quick"}</span>{note.isPinned === true && <b>置顶</b>}</div><h3>{entityText(note, "title") || "无标题笔记"}</h3><p>{entityText(note, "summary") || entityText(note, "contentText") || "暂无内容"}</p><small>{new Date(note.meta.updatedAt).toLocaleString("zh-CN")}</small><div className="card-actions"><button onClick={() => edit(note)}>编辑</button><button onClick={() => { store?.queueUpsert("note.note", { ...note, isPinned: note.isPinned !== true, meta: { ...note.meta } }); onChanged(); }}>{note.isPinned === true ? "取消置顶" : "置顶"}</button><button className="danger" onClick={() => { store?.queueDelete("note.note", note.meta.id); onChanged(); }}>删除</button></div></article>)}{!filtered.length && <EmptyState title="没有匹配的笔记" text="新建一条笔记，或调整搜索条件。" />}</div></div></div>;
}

function EnglishView({ userId, deviceId, articles, vocabulary, store, onChanged }: { userId: string; deviceId: string; articles: SyncEntity[]; vocabulary: SyncEntity[]; store: WebSyncStore | null; onChanged: () => void }) {
  const [word, setWord] = useState(""); const [definition, setDefinition] = useState(""); const [selectedArticle, setSelectedArticle] = useState<SyncEntity | null>(null); const [error, setError] = useState("");
  function addWord(event: FormEvent) { event.preventDefault(); if (!store) return; setError(""); try { store.queueUpsert("english.vocabulary", createVocabulary(userId, deviceId, word, definition)); setWord(""); setDefinition(""); onChanged(); } catch (cause) { setError(cause instanceof Error ? cause.message : "保存失败"); } }
  const sortedVocabulary = [...vocabulary].sort((a, b) => b.meta.updatedAt.localeCompare(a.meta.updatedAt));
  return <div className="page-stack"><div className="english-hero"><div><p className="eyebrow">DAILY ENGLISH</p><h2>阅读、摘词、复习，在同一个轻量流程里完成。</h2></div><div className="hero-number"><strong>{vocabulary.length}</strong><span>累计词汇</span></div></div><div className="two-column english-layout"><div className="page-stack"><div className="panel"><div className="panel-heading"><div><p className="eyebrow">文章目录</p><h3>可阅读内容</h3></div><strong>{articles.length}</strong></div><div className="article-list">{articles.map((article) => <button key={article.meta.id} onClick={() => setSelectedArticle(article)}><div><span>{entityText(article, "difficulty") || entityText(article, "level") || "English"}</span><h4>{entityText(article, "title") || "Untitled article"}</h4><p>{entityText(article, "summary") || entityText(article, "description") || entityText(article, "contentText").slice(0, 120)}</p></div><b>阅读 →</b></button>)}{!articles.length && <EmptyState title="暂无英语文章" text="文章目录由云端只读下发，完成同步后会显示。" />}</div></div>{selectedArticle && <div className="panel article-reader"><div className="panel-heading"><div><p className="eyebrow">阅读模式</p><h3>{entityText(selectedArticle, "title") || "English article"}</h3></div><button className="text-button" onClick={() => setSelectedArticle(null)}>关闭</button></div><div className="article-content">{entityText(selectedArticle, "contentMarkdown") || entityText(selectedArticle, "contentText") || entityText(selectedArticle, "content") || "文章正文暂不可用。"}</div></div>}</div><aside className="page-stack"><div className="panel"><div className="panel-heading"><div><p className="eyebrow">生词本</p><h3>添加词汇</h3></div></div><form className="form-stack" onSubmit={addWord}><label>单词<input value={word} onChange={(event) => setWord(event.target.value)} placeholder="resilient" required /></label><label>释义<input value={definition} onChange={(event) => setDefinition(event.target.value)} placeholder="有韧性的" /></label>{error && <div className="notice error">{error}</div>}<button className="primary-button">加入生词本</button></form></div><div className="panel"><div className="panel-heading"><div><p className="eyebrow">复习队列</p><h3>词汇状态</h3></div></div><div className="vocab-list">{sortedVocabulary.map((item) => <div key={item.meta.id}><div><strong>{entityText(item, "displayWord")}</strong><small>{entityText(item, "definition") || "暂未填写释义"}</small></div><button className={`status-pill ${entityText(item, "status").toLowerCase()}`} onClick={() => { const next = entityText(item, "status") === "MASTERED" ? "LEARNING" : "MASTERED"; store?.queueUpsert("english.vocabulary", { ...item, status: next, masteryLevel: next === "MASTERED" ? 5 : 1, meta: { ...item.meta } }); onChanged(); }}>{entityText(item, "status") || "LEARNING"}</button></div>)}{!sortedVocabulary.length && <p className="muted compact">从文章中摘词，或手动添加第一个单词。</p>}</div></div></aside></div></div>;
}

function EmptyState({ title, text }: { title: string; text: string }) { return <div className="empty-state"><span>·</span><h4>{title}</h4><p>{text}</p></div>; }
