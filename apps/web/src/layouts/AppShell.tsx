import { useEffect, useMemo, useState } from "react";
import { NavLink, Outlet, useLocation, useNavigate } from "react-router-dom";
import {
  Activity, Bot, CalendarDays, CheckSquare2, ChevronLeft, ChevronRight, Command,
  Dumbbell, FileText, GraduationCap, HeartPulse, Home, Leaf, Menu, Moon, NotebookPen,
  RefreshCw, Search, Settings, Sun, WalletCards, X,
} from "lucide-react";
import { useApp } from "../app/AppContext";
import { Badge, Button, Input, cn } from "../components/ui";

const nav = [
  { group: "工作台", items: [
    ["/app/today", "今日", Home], ["/app/execution", "计划与待办", CheckSquare2], ["/app/calendar", "日历", CalendarDays], ["/app/assistant", "AI 助手", Bot],
  ] },
  { group: "成长健康", items: [
    ["/app/habits", "坚持", Activity], ["/app/fitness", "健身", Dumbbell], ["/app/health", "健康", HeartPulse], ["/app/review", "复盘", FileText],
  ] },
  { group: "知识与资产", items: [
    ["/app/notes", "笔记", NotebookPen], ["/app/english", "英语学习", GraduationCap], ["/app/finance", "财务", WalletCards],
  ] },
] as const;

const mobile = [
  ["/app/today", "今日", Home], ["/app/execution", "计划", CheckSquare2], ["/app/finance", "财务", WalletCards], ["/app/notes", "笔记", NotebookPen],
] as const;

const commands = [
  ["打开今日", "/app/today"], ["新建任务", "/app/execution?new=task"], ["记录支出", "/app/finance/transactions?new=expense"], ["开始训练", "/app/fitness?new=workout"], ["新建笔记", "/app/notes?new=note"], ["打开设置", "/app/settings"],
] as const;

function routeActive(current: string, target: string) {
  return current === target || (target !== "/app/today" && current.startsWith(`${target}/`));
}

export function AppShell() {
  const { session, loading, online, privacy, setPrivacy, refresh, logout, theme, setTheme, error, clearError } = useApp();
  const location = useLocation();
  const navigate = useNavigate();
  const [collapsed, setCollapsed] = useState(false);
  const [commandOpen, setCommandOpen] = useState(false);
  const [moreOpen, setMoreOpen] = useState(false);
  const [query, setQuery] = useState("");

  useEffect(() => {
    const listener = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setCommandOpen((value) => !value);
      }
    };
    window.addEventListener("keydown", listener);
    return () => window.removeEventListener("keydown", listener);
  }, []);

  useEffect(() => { setMoreOpen(false); }, [location.pathname]);
  const filtered = useMemo(() => commands.filter(([label]) => label.includes(query.trim())), [query]);

  return <div className="min-h-screen bg-background lg:grid lg:grid-cols-[auto_1fr]">
    <aside className={cn("sticky top-0 hidden h-screen border-r bg-card lg:flex lg:flex-col", collapsed ? "w-[76px]" : "w-[236px]") }>
      <div className="flex h-16 items-center gap-3 border-b px-4">
        <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-primary text-primary-foreground"><Leaf size={18} /></div>
        {!collapsed ? <div className="min-w-0"><div className="font-semibold tracking-[-0.02em]">LifeTrace</div><div className="text-[11px] text-muted-foreground">Personal OS</div></div> : null}
      </div>
      <nav className="scrollbar-thin flex-1 overflow-y-auto px-3 py-4" aria-label="主导航">
        {nav.map((group) => <div key={group.group} className="mb-5">
          {!collapsed ? <div className="mb-1.5 px-2 text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">{group.group}</div> : null}
          <div className="space-y-1">{group.items.map(([path, label, Icon]) => <NavLink key={path} to={path} title={collapsed ? label : undefined} className={({ isActive }) => cn("flex h-9 items-center gap-3 rounded-md px-2.5 text-sm transition-colors", isActive || routeActive(location.pathname, path) ? "bg-accent font-medium text-accent-foreground" : "text-muted-foreground hover:bg-muted hover:text-foreground", collapsed && "justify-center px-0")}><Icon size={17} />{!collapsed ? <span>{label}</span> : null}</NavLink>)}</div>
        </div>)}
      </nav>
      <div className="border-t p-3">
        <NavLink to="/app/settings" className={({ isActive }) => cn("flex h-9 items-center gap-3 rounded-md px-2.5 text-sm text-muted-foreground hover:bg-muted hover:text-foreground", isActive && "bg-accent text-accent-foreground", collapsed && "justify-center px-0")}><Settings size={17} />{!collapsed ? "设置" : null}</NavLink>
        <Button className="mt-2 w-full" size={collapsed ? "icon" : "sm"} variant="ghost" onClick={() => setCollapsed((value) => !value)} aria-label={collapsed ? "展开侧边栏" : "折叠侧边栏"}>{collapsed ? <ChevronRight size={16} /> : <><ChevronLeft size={16} />收起侧栏</>}</Button>
      </div>
    </aside>

    <div className="min-w-0">
      <header className="sticky top-0 z-30 flex h-14 items-center justify-between border-b bg-background/95 px-4 backdrop-blur supports-[backdrop-filter]:bg-background/85 sm:px-6 lg:h-16 lg:px-8">
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-2 lg:hidden"><Leaf size={18} className="text-primary" /><span className="font-semibold">LifeTrace</span></div>
          {!online ? <Badge className="border-warning/30 bg-warning/10 text-warning">离线</Badge> : null}
          {loading ? <Badge>同步中</Badge> : null}
        </div>
        <div className="flex items-center gap-1.5">
          <Button variant="outline" className="hidden min-w-56 justify-between text-muted-foreground md:flex" onClick={() => setCommandOpen(true)}><span className="flex items-center gap-2"><Search size={15} />搜索或执行命令</span><kbd className="rounded border bg-muted px-1.5 py-0.5 text-[10px]">⌘K</kbd></Button>
          <Button size="icon" variant="ghost" onClick={() => void refresh()} aria-label="刷新云端数据"><RefreshCw size={17} /></Button>
          <Button size="icon" variant="ghost" onClick={() => void setTheme(theme === "dark" ? "light" : "dark")} aria-label="切换主题">{theme === "dark" ? <Sun size={17} /> : <Moon size={17} />}</Button>
          <Button className="md:hidden" size="icon" variant="ghost" onClick={() => setCommandOpen(true)} aria-label="搜索"><Command size={18} /></Button>
          <button onClick={() => setMoreOpen(true)} className="ml-1 flex h-9 w-9 items-center justify-center rounded-full border bg-card text-xs font-semibold" aria-label="账户菜单">{(session?.user.displayName || session?.user.email || "LT").slice(0, 2).toUpperCase()}</button>
        </div>
      </header>

      {error ? <div className="mx-4 mt-3 flex items-start justify-between gap-3 rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive sm:mx-6 lg:mx-8"><span>{error}</span><button onClick={clearError} aria-label="关闭错误"><X size={16} /></button></div> : null}
      <Outlet />
    </div>

    <nav className="fixed inset-x-0 bottom-0 z-40 grid h-[68px] grid-cols-5 border-t bg-background/95 pb-[env(safe-area-inset-bottom)] backdrop-blur lg:hidden" aria-label="移动端导航">
      {mobile.map(([path, label, Icon]) => <NavLink key={path} to={path} className={({ isActive }) => cn("flex min-h-11 flex-col items-center justify-center gap-1 text-[10px] text-muted-foreground", isActive || routeActive(location.pathname, path) ? "text-primary" : "")}><Icon size={19} /><span>{label}</span></NavLink>)}
      <button className="flex min-h-11 flex-col items-center justify-center gap-1 text-[10px] text-muted-foreground" onClick={() => setMoreOpen(true)}><Menu size={19} /><span>更多</span></button>
    </nav>

    {commandOpen ? <div className="fixed inset-0 z-50 flex items-start justify-center bg-black/35 px-4 pt-[10vh]" role="dialog" aria-modal="true" aria-label="全局命令">
      <div className="w-full max-w-xl overflow-hidden rounded-xl border bg-popover shadow-2xl">
        <div className="flex items-center gap-2 border-b px-3"><Search size={17} className="text-muted-foreground" /><Input autoFocus className="h-12 border-0 bg-transparent px-0 focus:ring-0" placeholder="搜索页面或执行命令…" value={query} onChange={(event) => setQuery(event.target.value)} onKeyDown={(event) => { if (event.key === "Escape") setCommandOpen(false); }} /><button onClick={() => setCommandOpen(false)} aria-label="关闭"><X size={17} /></button></div>
        <div className="max-h-[360px] overflow-y-auto p-2">{filtered.map(([label, path]) => <button key={label} className="flex w-full items-center justify-between rounded-md px-3 py-2.5 text-left text-sm hover:bg-muted" onClick={() => { setCommandOpen(false); setQuery(""); navigate(path); }}><span>{label}</span><span className="text-xs text-muted-foreground">↵</span></button>)}</div>
      </div>
    </div> : null}

    {moreOpen ? <div className="fixed inset-0 z-50 flex items-end justify-center bg-black/35 lg:items-center" role="dialog" aria-modal="true" aria-label="账户与更多菜单" onMouseDown={(event) => { if (event.currentTarget === event.target) setMoreOpen(false); }}>
      <div className="w-full rounded-t-xl border bg-popover p-4 shadow-2xl sm:max-w-md sm:rounded-xl">
        <div className="flex items-center justify-between"><div><div className="font-semibold">{session?.user.displayName || "LifeTrace 用户"}</div><div className="text-xs text-muted-foreground">{session?.user.email}</div></div><Button size="icon" variant="ghost" onClick={() => setMoreOpen(false)}><X size={17} /></Button></div>
        <div className="mt-4 grid gap-2">
          <Button variant="outline" className="justify-start" onClick={() => { setMoreOpen(false); navigate("/app/search"); }}><Search size={16} />全局搜索</Button>
          <Button variant="outline" className="justify-start" onClick={() => { setMoreOpen(false); navigate("/app/settings"); }}><Settings size={16} />设置</Button>
          <label className="flex items-center justify-between rounded-md border px-3 py-2 text-sm"><span>隐私金额</span><input type="checkbox" checked={privacy} onChange={(event) => setPrivacy(event.target.checked)} /></label>
          <Button variant="destructive" onClick={() => void logout()}>退出登录</Button>
        </div>
      </div>
    </div> : null}
  </div>;
}
