import { useEffect, useMemo, useState, type ReactNode } from "react";
import {
  BarChart3, BookOpen, Bot, CalendarDays, Camera, Check, ChevronRight, Cloud, Dumbbell,
  Eye, EyeOff, FileUp, Home, Languages, LogOut, Menu, NotebookPen, PanelLeftClose,
  PanelLeftOpen, RefreshCw, Search, Settings, Smartphone, WalletCards, CircleDollarSign,
} from "lucide-react";
import type { WebSession } from "../core";
import {
  MOBILE_NAV, NAV_GROUPS, PAGE_COPY, SECONDARY_NAV, navigate, routeIsActive,
  type NavItem, type Route,
} from "../navigation";
import { Notice } from "../ui";

const ICONS: Record<NavItem["icon"], typeof Home> = {
  home: Home,
  bot: Bot,
  check: Check,
  camera: Camera,
  languages: Languages,
  dumbbell: Dumbbell,
  note: NotebookPen,
  calendar: CalendarDays,
  review: BookOpen,
  chart: BarChart3,
  money: CircleDollarSign,
  wallet: WalletCards,
  upload: FileUp,
  devices: Smartphone,
  cloud: Cloud,
  settings: Settings,
  search: Search,
};

interface AppShellProps {
  route: Route;
  session: WebSession;
  online: boolean;
  loading: boolean;
  privacy: boolean;
  error: string;
  conflictCount: number;
  onRefresh: () => void;
  onTogglePrivacy: () => void;
  onLogout: () => void;
  children: ReactNode;
}

export function AppShell({
  route,
  session,
  online,
  loading,
  privacy,
  error,
  conflictCount,
  onRefresh,
  onTogglePrivacy,
  onLogout,
  children,
}: AppShellProps) {
  const [menuOpen, setMenuOpen] = useState(false);
  const [collapsed, setCollapsed] = useState(false);
  const [title, subtitle] = PAGE_COPY[route];
  const date = useMemo(
    () => new Intl.DateTimeFormat("zh-CN", { month: "long", day: "numeric", weekday: "long" }).format(new Date()),
    [route],
  );

  useEffect(() => {
    setMenuOpen(false);
  }, [route]);

  return <main className={`hx-shell browser-shell ${privacy ? "privacy-on" : ""}`} data-route={route}>
    <aside className={`${menuOpen ? "open" : ""} ${collapsed ? "collapsed" : ""}`} aria-label="主导航">
      <div className="lt-sidebar-brand-row">
        <button className="hx-brand" onClick={() => navigate("/")} aria-label="返回今日总览">
          <span>LT</span>
          <div><strong>LifeTrace</strong><small>个人管理空间</small></div>
        </button>
        <button
          className="lt-sidebar-toggle"
          aria-label={collapsed ? "展开侧栏" : "收起侧栏"}
          onClick={() => setCollapsed((value) => !value)}
        >{collapsed ? <PanelLeftOpen /> : <PanelLeftClose />}</button>
      </div>

      <nav aria-label="功能导航">
        {NAV_GROUPS.map((group) => <div key={group.label}>
          <label>{group.label}</label>
          {group.items.map((item) => <NavButton current={route} item={item} key={item.route} />)}
        </div>)}
      </nav>

      <div className="hx-sidebar-foot">
        <div className="lt-cloud-state">
          <i className={online ? "online" : "offline"} />
          <strong>{online ? "云端在线" : "网络离线"}</strong>
          <p>{online ? "数据直接同步到 LifeTrace Cloud" : "写入操作已暂停"}</p>
        </div>
        {SECONDARY_NAV.map((item) => <NavButton current={route} item={item} key={item.route} />)}
        <section className="lt-user-chip">
          <span>{(session.user.displayName || session.user.email)[0]?.toUpperCase()}</span>
          <div><strong>{session.user.displayName || "个人空间"}</strong><small>{session.user.email}</small></div>
        </section>
        <button className="hx-logout" onClick={onLogout}><LogOut /><span>退出登录</span></button>
      </div>
    </aside>

    {menuOpen && <button className="hx-nav-scrim" aria-label="关闭导航" onClick={() => setMenuOpen(false)} />}

    <div className="hx-main">
      <header className="hx-topbar">
        <button
          className="hx-menu"
          aria-label={menuOpen ? "关闭导航" : "打开导航"}
          aria-expanded={menuOpen}
          onClick={() => setMenuOpen((value) => !value)}
        ><Menu /></button>
        <div className="lt-page-heading">
          <span className="hx-kicker">{date}</span>
          <h1>{title}</h1>
          <p>{subtitle}</p>
        </div>
        <div className="hx-top-actions">
          <span className={`hx-status ${online ? "success" : "warning"}`}><i />{online ? "已同步" : "离线"}</span>
          <button className="hx-icon-btn" aria-label="全局搜索" onClick={() => navigate("/search")}><Search /></button>
          <button className={`hx-btn secondary ${privacy ? "active" : ""}`} onClick={onTogglePrivacy}>
            {privacy ? <Eye /> : <EyeOff />}{privacy ? "显示金额" : "隐藏金额"}
          </button>
          <button className="hx-btn secondary" disabled={!online || loading} onClick={onRefresh}>
            <RefreshCw className={loading ? "spin" : ""} />{loading ? "同步中" : "同步"}
          </button>
        </div>
      </header>

      <div className="browser-notices" aria-live="polite">
        {!online && <Notice kind="warning">网页端需要联网使用；离线期间所有写操作均已禁用。</Notice>}
        {error && <Notice kind="error">{error}</Notice>}
        {conflictCount > 0 && <Notice kind="warning">检测到 {conflictCount} 个跨设备版本冲突，当前显示服务器最新版本。</Notice>}
      </div>

      <section className="hx-page-content" aria-label={title}>{children}</section>
      <MobileNav route={route} />
    </div>
  </main>;
}

function NavButton({ current, item }: { current: Route; item: NavItem }) {
  const Icon = ICONS[item.icon];
  const active = routeIsActive(current, item.route);
  return <button
    className={active ? "active" : ""}
    aria-current={active ? "page" : undefined}
    title={item.label}
    onClick={() => navigate(item.route)}
  >
    <span><Icon /><b>{item.label}</b></span><ChevronRight />
  </button>;
}

function MobileNav({ route }: { route: Route }) {
  return <nav className="browser-mobile-nav" aria-label="移动端导航">
    {MOBILE_NAV.map((item) => {
      const Icon = ICONS[item.icon];
      const active = routeIsActive(route, item.route);
      return <button
        className={active ? "active" : ""}
        aria-current={active ? "page" : undefined}
        key={item.route}
        onClick={() => navigate(item.route)}
      ><Icon /><span>{item.label}</span></button>;
    })}
  </nav>;
}
