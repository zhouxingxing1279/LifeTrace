import { useState, type ReactNode } from "react";
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
import {
  NAV_GROUPS,
  PAGE_COPY,
  SECONDARY_NAV,
  routeIsActive,
  type NavIcon,
  type Route,
} from "@/web-client/src/navigation";

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

type DesktopWorkbenchShellProps = {
  route: Route;
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
  const [inspectorOpen, setInspectorOpen] = useState(true);
  const [sidebarCompact, setSidebarCompact] = useState(false);
  const [title, description] = PAGE_COPY[route];

  return (
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
            onClick={() => setSidebarCompact((value) => !value)}
          >
            {sidebarCompact ? <ChevronRight /> : <ChevronLeft />}
          </button>
        </div>

        <button type="button" className="lt-desk-search" onClick={() => onNavigate("/search")}>
          <Search aria-hidden="true" />
          <span>搜索全部内容</span>
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
              onClick={() => setInspectorOpen((value) => !value)}
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
              <li>系统级快捷操作</li>
            </ul>
            <button type="button" onClick={onOpenLocalTools}>打开本机工具</button>
          </section>
        </aside>
      ) : null}
    </div>
  );
}
