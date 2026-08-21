import { useCallback, useEffect, useMemo, useState, type FormEvent } from "react";
import { Plus } from "lucide-react";
import type { CloudSession } from "./api/cloud";
import { Button, Card, EmptyState, Input, Modal, Select, Skeleton, Toast } from "./design-system/ui";
import { initialState, isoDate, newId, type LifeTraceState } from "./model";
import type { PlatformAdapter } from "./platform";
import { AppShell } from "./shell/AppShell";
import { CalendarPage, FitnessPage, HabitsPage, PlanPage, TodayPage } from "./features/core-pages";
import { NotesPage, ReadingPage, ReviewPage, SearchPage, SettingsPage } from "./features/content-pages";
import { FinancePage, financeRoutes } from "./features/finance/FinancePage";
import { LoginPage } from "./features/auth/LoginPage";
import { PageHeader, Section } from "./features/shared";

function replacePath(path: string) {
  window.history.replaceState({}, "", path);
}

function LoadingWorkspace() {
  return <main className="lt-auth-page" aria-busy="true"><Card className="lt-auth-card"><div className="lt-caption">LIFETRACE V2</div><h1>Loading workspace</h1><Skeleton /><div style={{ height: 12 }} /><Skeleton width="72%" /></Card></main>;
}

function UiSystemPage() {
  return <><PageHeader title="Design System" detail="共享 tokens、primitives、反馈状态与平台响应式行为的生产验证页。" /><Section title="Production primitives"><Card><div className="lt-row"><Button>Primary action</Button><Button className="secondary">Secondary</Button><Button disabled>Disabled</Button></div><p className="lt-muted">所有业务页面只组合共享设计系统，Liquid Glass 仅用于导航、工具栏与瞬态控制层。</p></Card></Section></>;
}

function UnknownPage({ path, navigate }: { path: string; navigate: (path: string) => void }) {
  return <><PageHeader title="Not found" detail={path} /><EmptyState title="这个工作区不存在" detail="返回 Today 继续。" action={<Button onClick={() => navigate("/app/today")}>Back to Today</Button>} /></>;
}

export function LifeTraceApp({ platform }: { platform: PlatformAdapter }) {
  const [path, setPath] = useState(() => window.location.pathname || "/app/today");
  const [state, setState] = useState<LifeTraceState>(() => initialState());
  const [hydrated, setHydrated] = useState(false);
  const [booting, setBooting] = useState(true);
  const [session, setSession] = useState<CloudSession | null>(null);
  const [syncError, setSyncError] = useState("");
  const [quickOpen, setQuickOpen] = useState(false);
  const [captureType, setCaptureType] = useState<"task" | "note">("task");
  const [captureTitle, setCaptureTitle] = useState("");

  const navigate = useCallback((next: string) => {
    if (window.location.pathname !== next) window.history.pushState({}, "", next);
    setPath(next);
    window.scrollTo({ top: 0, behavior: state.settings.reducedMotion ? "auto" : "smooth" });
  }, [state.settings.reducedMotion]);

  const loadCloudState = useCallback(async () => {
    const stored = await platform.loadState();
    setState(stored ?? initialState());
    setHydrated(true);
    setSyncError("");
  }, [platform]);

  const bootstrap = useCallback(async () => {
    setBooting(true);
    try {
      if (platform.requiresAuthentication) {
        const current = await platform.getSession?.();
        setSession(current ?? { authenticated: false });
        if (!current?.authenticated) {
          setHydrated(false);
          if (window.location.pathname !== "/login") replacePath("/login");
          setPath("/login");
          if (current?.error) setSyncError(current.error);
          return;
        }
      }
      await loadCloudState();
      if (window.location.pathname === "/login" || window.location.pathname === "/") {
        replacePath("/app/today");
        setPath("/app/today");
      }
    } catch (error) {
      setSyncError(error instanceof Error ? error.message : "Workspace initialization failed");
      if (platform.requiresAuthentication) {
        replacePath("/login");
        setPath("/login");
      }
    } finally {
      setBooting(false);
    }
  }, [loadCloudState, platform]);

  useEffect(() => { void bootstrap(); }, [bootstrap]);
  useEffect(() => {
    const onPop = () => setPath(window.location.pathname);
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, []);

  useEffect(() => {
    const html = document.documentElement;
    html.dataset.theme = state.settings.theme;
    html.dataset.reducedMotion = state.settings.reducedMotion ? "true" : "false";
  }, [state.settings]);

  useEffect(() => {
    if (!hydrated) return;
    void platform.saveState(state).then(() => setSyncError("")).catch((error) => setSyncError(error instanceof Error ? error.message : "Cloud sync failed"));
  }, [hydrated, platform, state]);

  const onAuthenticated = useCallback(async (nextSession: CloudSession) => {
    setSession(nextSession);
    setBooting(true);
    try {
      await loadCloudState();
      replacePath("/app/today");
      setPath("/app/today");
    } finally {
      setBooting(false);
    }
  }, [loadCloudState]);

  const logout = useCallback(async () => {
    await platform.logout?.();
    setSession({ authenticated: false });
    setState(initialState());
    setHydrated(false);
    replacePath("/login");
    setPath("/login");
  }, [platform]);

  const reload = useCallback(async () => {
    setBooting(true);
    try { await loadCloudState(); }
    finally { setBooting(false); }
  }, [loadCloudState]);

  const saveCapture = (event: FormEvent) => {
    event.preventDefault();
    if (!captureTitle.trim()) return;
    if (captureType === "task") setState((current) => ({ ...current, tasks: [{ id: newId("task"), title: captureTitle.trim(), dueDate: isoDate(), project: "Inbox", priority: "normal", completed: false }, ...current.tasks] }));
    else setState((current) => ({ ...current, notes: [{ id: newId("note"), title: captureTitle.trim(), content: "", updatedAt: new Date().toISOString(), pinned: false }, ...current.notes] }));
    setCaptureTitle(""); setQuickOpen(false);
  };

  const page = useMemo(() => {
    if (path === "/app/today") return <TodayPage state={state} setState={setState} openQuickCapture={() => setQuickOpen(true)} />;
    if (path === "/app/execution") return <PlanPage state={state} setState={setState} />;
    if (path === "/app/calendar") return <CalendarPage state={state} />;
    if (path === "/app/habits") return <HabitsPage state={state} setState={setState} />;
    if (path === "/app/fitness" || path === "/app/health") return <FitnessPage state={state} setState={setState} />;
    if ((financeRoutes as readonly string[]).includes(path)) return <FinancePage state={state} setState={setState} path={path} navigate={navigate} />;
    if (path.startsWith("/app/english")) return <ReadingPage state={state} setState={setState} />;
    if (path === "/app/notes") return <NotesPage state={state} setState={setState} />;
    if (path === "/app/review") return <ReviewPage state={state} setState={setState} />;
    if (path === "/app/search") return <SearchPage state={state} navigate={navigate} />;
    if (path === "/app/settings") return <SettingsPage state={state} setState={setState} platform={platform} onLogout={logout} onReload={reload} />;
    if (path === "/app/system/ui") return <UiSystemPage />;
    return <UnknownPage path={path} navigate={navigate} />;
  }, [logout, navigate, path, platform, reload, state]);

  if (booting) return <LoadingWorkspace />;
  if (platform.requiresAuthentication && (!session?.authenticated || path === "/login")) return <><LoginPage platform={platform} onAuthenticated={onAuthenticated} />{syncError ? <Toast>{syncError}</Toast> : null}</>;
  if (!hydrated) return <LoadingWorkspace />;

  return <><AppShell path={path} navigate={navigate} state={state} openQuickCapture={() => setQuickOpen(true)}>{page}</AppShell>{syncError ? <Toast>{syncError}</Toast> : null}<Modal open={quickOpen} title="Quick Capture" onClose={() => setQuickOpen(false)}><form className="lt-form-grid" onSubmit={saveCapture}><Select value={captureType} onChange={(event) => setCaptureType(event.target.value as "task" | "note")} aria-label="Capture 类型"><option value="task">Task</option><option value="note">Note</option></Select><Input autoFocus value={captureTitle} onChange={(event) => setCaptureTitle(event.target.value)} placeholder="下一步要做什么？" /><Button type="submit"><Plus size={17} />Save</Button></form></Modal></>;
}
