import { startTransition, useEffect, useState } from "react";
import type { CSSProperties, PointerEvent as ReactPointerEvent, ReactNode } from "react";
import {
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

const SIDEBAR_WIDTH_STORAGE_KEY = "lifetrace:sidebar-width";
const DEFAULT_SIDEBAR_WIDTH = 252;
const MIN_SIDEBAR_WIDTH = 220;
const MAX_SIDEBAR_WIDTH = 360;

function storedSidebarWidth() {
  const value = Number(window.localStorage.getItem(SIDEBAR_WIDTH_STORAGE_KEY));
  return Number.isFinite(value) && value > 0
    ? Math.min(MAX_SIDEBAR_WIDTH, Math.max(MIN_SIDEBAR_WIDTH, value))
    : DEFAULT_SIDEBAR_WIDTH;
}

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
  const [sidebarWidth, setSidebarWidth] = useState(storedSidebarWidth);
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
    setMenuOpen(false);
    startTransition(() => onNavigate(next));
  };

  const startSidebarResize = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    event.preventDefault();
    const startX = event.clientX;
    const startWidth = sidebarWidth;
    let latestWidth = startWidth;
    const handle = event.currentTarget;
    handle.setPointerCapture(event.pointerId);
    document.documentElement.classList.add("is-resizing-sidebar");

    const move = (moveEvent: PointerEvent) => {
      latestWidth = Math.min(MAX_SIDEBAR_WIDTH, Math.max(MIN_SIDEBAR_WIDTH, startWidth + moveEvent.clientX - startX));
      setSidebarWidth(latestWidth);
    };
    const finish = () => {
      if (handle.hasPointerCapture(event.pointerId)) handle.releasePointerCapture(event.pointerId);
      document.documentElement.classList.remove("is-resizing-sidebar");
      window.localStorage.setItem(SIDEBAR_WIDTH_STORAGE_KEY, String(Math.round(latestWidth)));
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", finish);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", finish, { once: true });
  };

  const resizeSidebarBy = (delta: number) => {
    setSidebarWidth((current) => {
      const next = Math.min(MAX_SIDEBAR_WIDTH, Math.max(MIN_SIDEBAR_WIDTH, current + delta));
      window.localStorage.setItem(SIDEBAR_WIDTH_STORAGE_KEY, String(Math.round(next)));
      return next;
    });
  };

  return (
    <>
      <main className="hx-shell" style={{ "--ui-sidebar-width": `${sidebarWidth}px` } as CSSProperties}>
        <aside aria-label="主导航">
          <div className="hx-brand">
            <span aria-hidden="true">LT</span>
            <div>
              <strong>LifeTrace</strong>
              <small>个人管理系统</small>
            </div>
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
            >
              <span>
                <Settings aria-hidden="true" />
                <span className="lt-sidebar-text">设置</span>
              </span>
            </button>
          </div>
          <div
            className="hx-sidebar-resizer"
            role="separator"
            aria-label="调整导航栏宽度"
            aria-orientation="vertical"
            aria-valuemin={MIN_SIDEBAR_WIDTH}
            aria-valuemax={MAX_SIDEBAR_WIDTH}
            aria-valuenow={Math.round(sidebarWidth)}
            tabIndex={0}
            title="拖动调整导航栏宽度；双击恢复默认"
            onPointerDown={startSidebarResize}
            onDoubleClick={() => {
              setSidebarWidth(DEFAULT_SIDEBAR_WIDTH);
              window.localStorage.setItem(SIDEBAR_WIDTH_STORAGE_KEY, String(DEFAULT_SIDEBAR_WIDTH));
            }}
            onKeyDown={(event) => {
              if (event.key === "ArrowLeft") resizeSidebarBy(-10);
              if (event.key === "ArrowRight") resizeSidebarBy(10);
            }}
          />
        </aside>

        {menuOpen ? (
          <button
            type="button"
            className="hx-nav-scrim"
            aria-label="关闭导航"
            onClick={() => setMenuOpen(false)}
          />
        ) : null}

        <div className="hx-main" data-view={view}>
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
