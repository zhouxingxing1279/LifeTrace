import { useEffect, useMemo, useState, type ReactNode } from "react";
import { Activity, BookOpen, CalendarDays, CircleDollarSign, Command, Dumbbell, Home, ListTodo, Moon, NotebookPen, PanelLeft, Plus, Search, Settings, Sun } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { Badge, Button, CommandPalette, IconButton } from "../design-system/ui";
import { searchState, type LifeTraceState } from "../model";

interface NavigationItem { label: string; path: string; icon: LucideIcon; mobile?: boolean }

export const navigation: NavigationItem[] = [
  { label: "Today", path: "/app/today", icon: Home, mobile: true },
  { label: "Plan", path: "/app/execution", icon: ListTodo, mobile: true },
  { label: "Calendar", path: "/app/calendar", icon: CalendarDays, mobile: true },
  { label: "Habits", path: "/app/habits", icon: Activity },
  { label: "Fitness", path: "/app/fitness", icon: Dumbbell, mobile: true },
  { label: "Finance", path: "/app/finance", icon: CircleDollarSign, mobile: true },
  { label: "Reading", path: "/app/english/articles", icon: BookOpen },
  { label: "Notes", path: "/app/notes", icon: NotebookPen },
  { label: "Review", path: "/app/review", icon: Activity },
  { label: "Search", path: "/app/search", icon: Search },
  { label: "Settings", path: "/app/settings", icon: Settings }
];

export function routeTitle(path: string) {
  if (path.startsWith("/app/finance")) return "Finance";
  if (path.startsWith("/app/english")) return "Reading";
  if (path === "/app/health") return "Fitness / Health";
  return navigation.find((item) => path === item.path)?.label ?? "LifeTrace";
}

function active(path: string, item: NavigationItem) {
  if (item.path === "/app/finance") return path.startsWith("/app/finance");
  if (item.path === "/app/english/articles") return path.startsWith("/app/english");
  return path === item.path;
}

export function AppShell({ path, navigate, state, openQuickCapture, children }: { path: string; navigate: (path: string) => void; state: LifeTraceState; openQuickCapture: () => void; children: ReactNode }) {
  const [collapsed, setCollapsed] = useState(false);
  const [commandsOpen, setCommandsOpen] = useState(false);
  const [commandQuery, setCommandQuery] = useState("");
  const commandHits = useMemo(() => searchState(state, commandQuery), [state, commandQuery]);

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      const modifier = event.metaKey || event.ctrlKey;
      if (modifier && event.key.toLowerCase() === "k") { event.preventDefault(); setCommandsOpen(true); }
      if (modifier && event.key.toLowerCase() === "n") { event.preventDefault(); openQuickCapture(); }
      if (modifier && event.key === ",") { event.preventDefault(); navigate("/app/settings"); }
      if (event.key === "Escape") setCommandsOpen(false);
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [navigate, openQuickCapture]);

  const commandItems = navigation.filter((item) => !commandQuery || item.label.toLowerCase().includes(commandQuery.toLowerCase()));
  const go = (target: string) => { navigate(target); setCommandsOpen(false); setCommandQuery(""); };
  return <div className={`lt-app-shell ${collapsed ? "is-collapsed" : ""}`}>
    <aside className="lt-sidebar">
      <div className="lt-sidebar-brand"><strong>LifeTrace</strong><IconButton aria-label="切换侧边栏" onClick={() => setCollapsed((value) => !value)}><PanelLeft size={18} /></IconButton></div>
      <nav aria-label="主导航">{navigation.map((item) => { const Icon = item.icon; return <button key={item.path} className={`lt-nav-item ${active(path, item) ? "is-active" : ""}`} onClick={() => navigate(item.path)} aria-current={active(path, item) ? "page" : undefined}><Icon size={18} /><span>{item.label}</span></button>; })}</nav>
      <div className="lt-sidebar-footer"><Badge tone="accent">V2</Badge><span className="lt-caption">Cloud + Native adapters</span></div>
    </aside>
    <div className="lt-workspace">
      <header className="lt-toolbar"><div className="lt-row"><strong>{routeTitle(path)}</strong></div><div className="lt-row"><IconButton aria-label="切换主题" onClick={() => { const html = document.documentElement; html.dataset.theme = html.dataset.theme === "dark" ? "light" : "dark"; }}>{document.documentElement.dataset.theme === "dark" ? <Sun size={18} /> : <Moon size={18} />}</IconButton><Button className="secondary" onClick={() => setCommandsOpen(true)}><Command size={16} /> <span>Command</span></Button><Button onClick={openQuickCapture}><Plus size={16} /> Capture</Button></div></header>
      <main className="lt-main">{children}</main>
    </div>
    <nav className="lt-mobile-nav" aria-label="移动导航">{navigation.filter((item) => item.mobile).map((item) => { const Icon = item.icon; return <button key={item.path} className={active(path, item) ? "is-active" : ""} onClick={() => navigate(item.path)} aria-label={item.label}><Icon size={20} /><span>{item.label}</span></button>; })}</nav>
    <CommandPalette open={commandsOpen} query={commandQuery} onQuery={setCommandQuery} onClose={() => setCommandsOpen(false)}>{commandItems.map((item) => { const Icon = item.icon; return <button className="lt-command-item" key={item.path} onClick={() => go(item.path)}><Icon size={17} /><span>{item.label}</span></button>; })}{commandHits.map((item) => <button className="lt-command-item" key={`${item.type}-${item.id}`} onClick={() => go(item.path)}><Search size={17} /><span>{item.title}</span><Badge>{item.type}</Badge></button>)}</CommandPalette>
  </div>;
}
