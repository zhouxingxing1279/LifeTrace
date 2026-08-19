import { useEffect, useMemo, useState, type ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import {
  BarChart3,
  Bot,
  CalendarDays,
  Camera,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  Cloud,
  Dumbbell,
  Eye,
  EyeOff,
  FileText,
  HardDrive,
  Home,
  Languages,
  ListChecks,
  LoaderCircle,
  LogOut,
  MonitorSmartphone,
  NotebookPen,
  PanelRightClose,
  PanelRightOpen,
  Plus,
  ReceiptText,
  RefreshCw,
  Search,
  Settings,
  WalletCards,
  WifiOff,
} from "lucide-react";
import CommandPalette, { type CommandItem } from "@/src/components/layout/CommandPalette";
import {
  NAV_GROUPS,
  PAGE_COPY,
  SECONDARY_NAV,
  routeIsActive,
  type NavIcon,
  type Route,
} from "@/web-client/src/navigation";

const SIDEBAR_COMPACT_KEY = "lifetrace:desktop:sidebar-compact";
const INSPECTOR_OPEN_KEY = "lifetrace:desktop:inspector-open";

const NAV_ICONS: Record<NavIcon, LucideIcon> = {
  home: Home,
  bot: Bot,
  check: ListChecks,
  camera: Camera,
  languages: Languages,
  dumbbell: Dumbbell,
  note: NotebookPen,
  calendar: CalendarDays,
  review: CheckCircle2,
  chart: BarChart3,
  money: ReceiptText,
  wallet: WalletCards,
  upload: Plus,
  cloud: Cloud,
  devices: MonitorSmartphone,
  settings: Settings,
  search: Search,
};

function storedBoolean(key: string, fallback: boolean): boolean {
  if (typeof window === "undefined") return fallback;
  const value = window.localStorage.getItem(key);
  if (value === "true") return true;
  if (value === "false") return false;
  return fallback;
}

type DesktopWorkbenchShellProps = {
  route: Route;
  titleOverride?: string;
  descriptionOverride?: string;
  userLabel: string;
  online: boolean;
  loading: boolean;
  privacy: boolean;
  error: string;
  conflictCount: number;
  onNavigate: (route: Route) => void;
  onRefresh: () => void;
  onTogglePrivacy: () => void;
  onLogout: () => void;
  onOpenLocalTools: () => void;
  children: ReactNode;
};

function NavButton({
  route,
  current,
  label,
  icon,
  onNavigate,
}: {
  route: Route;
  current: Route;
  label: string;
  icon: NavIcon;
  onNavigate: (route: Route) => void;
}) {
  const Icon = NAV_ICONS[icon];
  const active = routeIsActive(current, route);
  return (
    <button
      type="button"
      className={`lt-desk-nav-item${active ? " active" : ""}`}
      aria-current={active ? "page" : undefined}
      onClick={() => onNavigate(route)}
    >
      <Icon aria-hidden="true" />
      <span>{label}</span>
    </button>
  );
}

export default function DesktopWorkbenchShell({
  route,
  titleOverride,
  descriptionOverride,
  userLabel,
  online,
  loading,
  privacy,
  error,
  conflictCount,
  onNavigate,
  onRefresh,
  onTogglePrivacy,
  onLogout,
  onOpenLocalTools,
  children,
}: DesktopWorkbenchShellProps) {
  const [inspectorOpen, setInspectorOpen] = useState(() => storedBoolean(INSPECTOR_OPEN_KEY, true));
  const [sidebarCompact, setSidebarCompact] = useState(() => storedBoolean(SIDEBAR_COMPACT_KEY, false));
  const [commandOpen, setCommandOpen] = useState(false);
  const [routeTitle, routeDescription] = PAGE_COPY[route];
  const title = titleOverride ?? routeTitle;
  const description = descriptionOverride ?? routeDescription;

  const setInspector = (next: boolean) => {
    setInspectorOpen(next);
    window.localStorage.setItem(INSPECTOR_OPEN_KEY, String(next));
  };

  const setSidebar = (next: boolean) => {
    setSidebarCompact(next);
    window.localStorage.setItem(SIDEBAR_COMPACT_KEY, String(next));
  };

  useEffect(() => {
    const handleShortcut = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey)) return;
      const key = event.key.toLowerCase();
      if (key === "k" || (event.shiftKey && key === "p")) {
        event.preventDefault();
        setCommandOpen((value) => !value);
      }
    };
    window.addEventListener("keydown", handleShortcut);
    return () => window.removeEventListener("keydown", handleShortcut);
  }, []);

  const commandItems = useMemo<CommandItem[]>(() => {
    const navigation: CommandItem[] = NAV_GROUPS.flatMap((group) =>
      group.items.map((item) => ({
        id: `desktop-nav:${item.route}`,
        label: `打开${item.label}`,
        hint: group.label,
        icon: NAV_ICONS[item.icon],
        group: "导航",
        keywords: `${group.label} ${item.label} ${item.route}`,
        execute: () => onNavigate(item.route),
      })),
    );
    const secondary: CommandItem[] = SECONDARY_NAV.map((item) => ({
      id: `desktop-secondary:${item.route}`,
      label: `打开${item.label}`,
      hint: "桌面",
      icon: NAV_ICONS[item.icon],
      group: "导航",
      keywords: `${item.label} ${item.route}`,
      execute: () => onNavigate(item.route),
    }));
    return [
      ...navigation,
      ...secondary,
      {
        id: "desktop-search",
        label: "打开全局搜索",
        hint: "搜索所有云端内容",
        icon: Search,
        group: "导航",
        keywords: "搜索 search",
        execute: () => onNavigate("/search"),
      },
      {
        id: "desktop-refresh",
        label: "立即同步",
        hint: online ? "刷新云端并更新本机副本" : "当前离线",
        icon: RefreshCw,
        group: "操作",
        keywords: "同步 刷新 refresh sync",
        execute: () => { if (online && !loading) onRefresh(); },
      },
      {
        id: "desktop-privacy",
        label: privacy ? "关闭隐私模式" : "开启隐私模式",
        hint: "快速隐藏敏感内容",
        icon: privacy ? EyeOff : Eye,
        group: "操作",
        keywords: "隐私 privacy",
        execute: onTogglePrivacy,
      },
      {
        id: "desktop-local-tools",
        label: "打开本机工具",
        hint: "SQLite、照片、文件与离线能力",
        icon: HardDrive,
        group: "桌面能力",
        keywords: "本机 本地 sqlite 照片 文件 离线 local",
        execute: onOpenLocalTools,
      },
      {
        id: "desktop-toggle-inspector",
        label: inspectorOpen ? "隐藏右侧信息面板" : "显示右侧信息面板",
        hint: "调整桌面工作区布局",
        icon: inspectorOpen ? PanelRightClose : PanelRightOpen,
        group: "桌面能力",
        keywords: "侧栏 面板 inspector",
        execute: () => setInspector(!inspectorOpen),
      },
      {
        id: "desktop-toggle-sidebar",
        label: sidebarCompact ? "展开左侧导航" : "收起左侧导航",
        hint: "调整桌面工作区布局",
        icon: sidebarCompact ? ChevronRight : ChevronLeft,
        group: "桌面能力",
        keywords: "导航 侧栏 sidebar compact",
        execute: () => setSidebar(!sidebarCompact),
      },
    ];
  }, [inspectorOpen, loading, onNavigate, onOpenLocalTools, onRefresh, onTogglePrivacy, online, privacy, sidebarCompact]);

  return (
    <>
      <div
        className={`lt-desktop-native-shell${inspectorOpen ? " inspector-open" : ""}${sidebarCompact ? " sidebar-compact" : ""}`}
      >
        <aside className="lt-desk-sidebar" aria-label="LifeTrace 桌面导航">
          <div className="lt-desk-brand-row">
            <button
              type="button"
              className="lt-desk-brand"
              title="LifeTrace"
              onClick={() => onNavigate("/")}
            >
              <span>LT</span>
              <strong>LifeTrace</strong>
            </button>
            <button
              type="button"
              className="lt-desk-sidebar-toggle"
              title={sidebarCompact ? "展开侧栏" : "收起侧栏"}
              onClick={() => setSidebar(!sidebarCompact)}
            >
              {sidebarCompact ? <ChevronRight /> : <ChevronLeft />}
            </button>
          </div>

          <button type="button" className="lt-desk-search" onClick={() => setCommandOpen(true)}>
            <Search aria-hidden="true" />
            <span>搜索或执行命令</span>
            <kbd>Ctrl K</kbd>
          </button>

          <nav className="lt-desk-nav-scroll">
            {NAV_GROUPS.map((group) => (
              <section className="lt-desk-nav-group" key={group.label}>
                <p>{group.label}</p>
                {group.items.map((item) => (
                  <NavButton
                    key={item.route}
                    route={item.route}
                    current={route}
                    label={item.label}
                    icon={item.icon}
                    onNavigate={onNavigate}
                  />
                ))}
              </section>
            ))}
          </nav>

          <div className="lt-desk-sidebar-bottom">
            <button type="button" className="lt-desk-nav-item lt-desk-local-entry" onClick={onOpenLocalTools}>
              <HardDrive aria-hidden="true" />
              <span>本机工具</span>
            </button>
            {SECONDARY_NAV.map((item) => (
              <NavButton
                key={item.route}
                route={item.route}
                current={route}
                label={item.label}
                icon={item.icon}
                onNavigate={onNavigate}
              />
            ))}
            <div className="lt-desk-account">
              <div className="lt-desk-avatar">{userLabel.trim().slice(0, 1).toUpperCase() || "L"}</div>
              <span title={userLabel}>{userLabel}</span>
              <button type="button" title="退出登录" onClick={onLogout}><LogOut /></button>
            </div>
          </div>
        </aside>

        <section className="lt-desk-stage">
          <header className="lt-desktop-commandbar">
            <div className="lt-desk-history-actions">
              <button type="button" title="后退" onClick={() => window.history.back()}><ChevronLeft /></button>
              <button type="button" title="前进" onClick={() => window.history.forward()}><ChevronRight /></button>
            </div>

            <div className="lt-desk-page-heading">
              <strong>{title}</strong>
              <span>{description}</span>
            </div>

            <div className="lt-desk-command-actions">
              <button type="button" title="命令面板（Ctrl+K）" onClick={() => setCommandOpen(true)}><Search /></button>
              <button
                type="button"
                className={`lt-desk-sync${!online ? " offline" : ""}`}
                title={online ? "立即同步" : "当前离线"}
                disabled={!online || loading}
                onClick={onRefresh}
              >
                {!online ? <WifiOff /> : loading ? <LoaderCircle className="spin" /> : <RefreshCw />}
                <span>{online ? (loading ? "同步中" : "已连接") : "离线"}</span>
              </button>
              <button type="button" title={privacy ? "关闭隐私模式" : "开启隐私模式"} onClick={onTogglePrivacy}>
                {privacy ? <EyeOff /> : <Eye />}
              </button>
              <button
                type="button"
                title={inspectorOpen ? "隐藏侧边信息" : "显示侧边信息"}
                onClick={() => setInspector(!inspectorOpen)}
              >
                {inspectorOpen ? <PanelRightClose /> : <PanelRightOpen />}
              </button>
            </div>
          </header>

          <main className="lt-desk-content">
            {error ? <div className="lt-desk-error" role="alert">{error}</div> : null}
            <div className="lt-desk-route-content">{children}</div>
          </main>
        </section>

        {inspectorOpen ? (
          <aside className="lt-desk-inspector" aria-label="桌面辅助面板">
            <section className="lt-desk-inspector-section">
              <div className="lt-desk-inspector-title"><span>同步状态</span>{online ? <Cloud /> : <WifiOff />}</div>
              <strong>{online ? "云端已连接" : "正在使用本机能力"}</strong>
              <p>{online ? "修改会保存到云端，并同步一份到本机。" : "云端暂不可用，本机数据和离线能力仍可使用。"}</p>
              {conflictCount > 0 ? <button type="button" onClick={() => onNavigate("/settings")}>{conflictCount} 个同步冲突待处理</button> : null}
            </section>

            <section className="lt-desk-inspector-section">
              <div className="lt-desk-inspector-title"><span>快捷开始</span><Plus /></div>
              <div className="lt-desk-quick-grid">
                <button type="button" onClick={() => onNavigate("/execution")}><ListChecks /><span>新建任务</span></button>
                <button type="button" onClick={() => onNavigate("/notes")}><FileText /><span>记一条笔记</span></button>
                <button type="button" onClick={() => onNavigate("/finance/transactions")}><ReceiptText /><span>手动记账</span></button>
                <button type="button" onClick={onOpenLocalTools}><HardDrive /><span>本机工具</span></button>
              </div>
            </section>

            <section className="lt-desk-inspector-section lt-desk-native-card">
              <div className="lt-desk-inspector-title"><span>桌面能力</span><MonitorSmartphone /></div>
              <ul>
                <li>SQLite 本地副本</li>
                <li>本地照片与文件导入</li>
                <li>离线访问与后台同步</li>
                <li>Ctrl+K 命令面板</li>
              </ul>
              <button type="button" onClick={onOpenLocalTools}>打开本机工具</button>
            </section>
          </aside>
        ) : null}
      </div>
      <CommandPalette open={commandOpen} onClose={() => setCommandOpen(false)} items={commandItems} />
    </>
  );
}
