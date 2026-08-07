import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import {
  ChevronsLeft,
  Menu,
  Search,
  Settings,
  X,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useLifeStore } from "@/src/stores/useLifeStore";
import { IconButton, Kbd, Tooltip } from "@/src/components/ui";
import CommandPalette from "./CommandPalette";
import type { CommandItem } from "./CommandPalette";

export interface NavItem {
  id: string;
  label: string;
  icon: LucideIcon;
}

export interface NavGroup {
  label: string;
  items: NavItem[];
}

interface AppShellProps {
  view: string;
  navGroups: NavGroup[];
  title: string;
  subtitle?: string;
  onNavigate: (view: string) => void;
  commandItems: CommandItem[];
  children: ReactNode;
  pageActions?: ReactNode;
}

const SIDEBAR_STORAGE_KEY = "lifetrace:sidebar-collapsed";

export default function AppShell({
  view,
  navGroups,
  title,
  subtitle,
  onNavigate,
  commandItems,
  children,
  pageActions,
}: AppShellProps) {
  const dark = useLifeStore((state) => state.dark);
  const [menuOpen, setMenuOpen] = useState(false);
  const [collapsed, setCollapsed] = useState(
    () => window.localStorage.getItem(SIDEBAR_STORAGE_KEY) === "1",
  );
  const [commandOpen, setCommandOpen] = useState(false);

  useEffect(() => {
    const forced = new URLSearchParams(window.location.search).get("theme");
    document.documentElement.dataset.theme =
      forced === "dark" || forced === "light"
        ? forced
        : dark
          ? "dark"
          : "light";
  }, [dark]);

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      const shortcut = event.ctrlKey || event.metaKey;
      if (!shortcut) return;
      const key = event.key.toLowerCase();
      if (key === "k" || (event.shiftKey && key === "p")) {
        event.preventDefault();
        setCommandOpen((open) => !open);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  const navigate = (next: string) => {
    onNavigate(next);
    setMenuOpen(false);
  };

  const toggleCollapsed = () => {
    setCollapsed((current) => {
      const next = !current;
      window.localStorage.setItem(SIDEBAR_STORAGE_KEY, next ? "1" : "0");
      return next;
    });
  };

  return (
    <>
      <main className="hx-shell">
        <aside className={collapsed && !menuOpen ? "collapsed" : ""} aria-label="主导航">
          <div className="hx-brand">
            <span aria-hidden="true">LT</span>
            <div>
              <strong>LifeTrace</strong>
              <small>个人管理系统</small>
            </div>
            <button
              type="button"
              className="hx-sidebar-collapse"
              aria-label={collapsed ? "展开侧边栏" : "折叠侧边栏"}
              onClick={toggleCollapsed}
            >
              <ChevronsLeft
                aria-hidden="true"
                style={collapsed ? { transform: "rotate(180deg)" } : undefined}
              />
            </button>
          </div>
          <nav>
            {navGroups.map((group) => (
              <div key={group.label}>
                <label>{group.label}</label>
                {group.items.map(({ id, label, icon: Icon }) => (
                  <button
                    key={id}
                    type="button"
                    className={view === id ? "active" : ""}
                    aria-current={view === id ? "page" : undefined}
                    onClick={() => navigate(id)}
                    title={collapsed ? label : undefined}
                  >
                    <span>
                      <Icon aria-hidden="true" />
                      <span className="lt-sidebar-text">{label}</span>
                    </span>
                  </button>
                ))}
              </div>
            ))}
          </nav>
          <div className="hx-sidebar-foot">
            <button
              type="button"
              className={view === "settings" ? "active" : ""}
              aria-current={view === "settings" ? "page" : undefined}
              onClick={() => navigate("settings")}
              title={collapsed ? "设置" : undefined}
            >
              <span>
                <Settings aria-hidden="true" />
                <span className="lt-sidebar-text">设置</span>
              </span>
            </button>
          </div>
        </aside>

        {menuOpen ? (
          <button
            type="button"
            className="hx-nav-scrim"
            aria-label="关闭导航"
            onClick={() => setMenuOpen(false)}
          />
        ) : null}

        <div className="hx-main">
          <header className="hx-topbar">
            <button
              type="button"
              className="hx-menu"
              aria-label={menuOpen ? "关闭导航" : "打开导航"}
              aria-expanded={menuOpen}
              onClick={() => setMenuOpen((open) => !open)}
            >
              {menuOpen ? <X aria-hidden="true" /> : <Menu aria-hidden="true" />}
            </button>
            <div className="hx-page-heading">
              <h1>{title}</h1>
              {subtitle ? <p>{subtitle}</p> : null}
            </div>
            <div className="hx-page-actions">
              {pageActions}
              <Tooltip label="命令面板（Ctrl+K）">
                <IconButton
                  label="打开命令面板"
                  onClick={() => setCommandOpen(true)}
                >
                  <Search aria-hidden="true" />
                </IconButton>
              </Tooltip>
              <span className="lt-topbar-kbd">
                <Kbd>Ctrl K</Kbd>
              </span>
            </div>
          </header>
          {children}
        </div>
      </main>
      <CommandPalette
        open={commandOpen}
        onClose={() => setCommandOpen(false)}
        items={commandItems}
      />
    </>
  );
}
