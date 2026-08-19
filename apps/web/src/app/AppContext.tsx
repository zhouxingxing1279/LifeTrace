import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type PropsWithChildren } from "react";
import {
  AuthApi,
  CloudDataStore,
  EMPTY_CLOUD_STATE,
  createPreference,
  type CloudState,
  type EntityType,
  type JsonEntity,
  type WebSession,
} from "../services/core";

export type ThemeMode = "system" | "light" | "dark";

type StoreAction = (store: CloudDataStore) => Promise<CloudState>;

interface AppContextValue {
  session: WebSession | null;
  state: CloudState;
  authLoading: boolean;
  loading: boolean;
  online: boolean;
  error: string;
  privacy: boolean;
  theme: ThemeMode;
  login(email: string, password: string, publicDevice: boolean): Promise<void>;
  logout(): Promise<void>;
  refresh(): Promise<void>;
  run(action: StoreAction): Promise<CloudState>;
  upsert(entityType: EntityType, entity: JsonEntity): Promise<CloudState>;
  remove(entityType: EntityType, entityId: string): Promise<CloudState>;
  setPrivacy(value: boolean): void;
  setTheme(mode: ThemeMode): Promise<void>;
  clearError(): void;
}

const AppContext = createContext<AppContextValue | null>(null);
const THEME_COOKIE = "lifetrace_theme";

function resolvedTheme(mode: ThemeMode): "light" | "dark" {
  if (mode === "system" && typeof window !== "undefined") {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  return mode === "dark" ? "dark" : "light";
}

function applyTheme(mode: ThemeMode): void {
  const value = resolvedTheme(mode);
  document.documentElement.dataset.theme = value;
  document.documentElement.style.colorScheme = value;
  document.querySelector('meta[name="theme-color"]')?.setAttribute("content", value === "dark" ? "#121713" : "#f7f8f6");
  document.cookie = `${THEME_COOKIE}=${mode}; Path=/; Max-Age=31536000; SameSite=Lax`;
}

function themeFromCookie(): ThemeMode {
  if (typeof document === "undefined") return "system";
  const value = document.cookie.match(/(?:^|; )lifetrace_theme=([^;]+)/)?.[1];
  return value === "light" || value === "dark" || value === "system" ? value : "system";
}

export function AppProvider({ children }: PropsWithChildren) {
  const auth = useMemo(() => new AuthApi(), []);
  const storeRef = useRef<CloudDataStore | null>(null);
  const [session, setSession] = useState<WebSession | null>(null);
  const [state, setState] = useState<CloudState>(EMPTY_CLOUD_STATE);
  const [authLoading, setAuthLoading] = useState(true);
  const [loading, setLoading] = useState(false);
  const [online, setOnline] = useState(() => typeof navigator === "undefined" ? true : navigator.onLine);
  const [error, setError] = useState("");
  const [privacy, setPrivacy] = useState(false);
  const [theme, setThemeState] = useState<ThemeMode>(themeFromCookie);

  useEffect(() => {
    applyTheme(theme);
    if (theme !== "system") return;
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const listener = () => applyTheme("system");
    media.addEventListener("change", listener);
    return () => media.removeEventListener("change", listener);
  }, [theme]);

  useEffect(() => {
    const connected = () => setOnline(true);
    const disconnected = () => setOnline(false);
    window.addEventListener("online", connected);
    window.addEventListener("offline", disconnected);
    return () => {
      window.removeEventListener("online", connected);
      window.removeEventListener("offline", disconnected);
    };
  }, []);

  useEffect(() => {
    let active = true;
    if (!online) {
      setAuthLoading(false);
      return;
    }
    auth.session()
      .then((value) => { if (active) setSession(value); })
      .catch((cause: unknown) => {
        const message = cause instanceof Error ? cause.message : String(cause);
        if (active && !/401|unauth|authentication|session/i.test(message)) setError(message);
      })
      .finally(() => { if (active) setAuthLoading(false); });
    return () => { active = false; };
  }, [auth, online]);

  useEffect(() => {
    if (!session) {
      storeRef.current = null;
      setState(EMPTY_CLOUD_STATE);
      return;
    }
    const store = new CloudDataStore(session.user.id, session.session.deviceId, session.csrfToken);
    storeRef.current = store;
    let active = true;
    setLoading(true);
    setError("");
    store.load()
      .then((next) => { if (active) setState(next); })
      .catch((cause: unknown) => { if (active) setError(cause instanceof Error ? cause.message : "无法加载云端数据"); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [session?.user.id, session?.session.deviceId, session?.csrfToken]);

  const login = useCallback(async (email: string, password: string, publicDevice: boolean) => {
    setLoading(true);
    setError("");
    try {
      const next = await auth.login(email, password, publicDevice);
      setSession(next);
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : "登录失败";
      setError(message);
      throw cause;
    } finally {
      setLoading(false);
    }
  }, [auth]);

  const logout = useCallback(async () => {
    setLoading(true);
    try {
      if (online) await auth.logout(session?.csrfToken);
    } finally {
      storeRef.current?.reset();
      storeRef.current = null;
      setSession(null);
      setState(EMPTY_CLOUD_STATE);
      setLoading(false);
    }
  }, [auth, online, session?.csrfToken]);

  const refresh = useCallback(async () => {
    const store = storeRef.current;
    if (!store || !online) return;
    setLoading(true);
    setError("");
    try { setState(await store.refresh()); }
    catch (cause) { setError(cause instanceof Error ? cause.message : "刷新失败"); }
    finally { setLoading(false); }
  }, [online]);

  const run = useCallback(async (action: StoreAction) => {
    const store = storeRef.current;
    if (!store) throw new Error("云端数据服务尚未就绪");
    if (!online) throw new Error("当前离线，数据未保存");
    setLoading(true);
    setError("");
    try {
      const next = await action(store);
      setState(next);
      return next;
    } catch (cause) {
      setState(store.snapshot());
      setError(cause instanceof Error ? cause.message : "云端操作失败");
      throw cause;
    } finally {
      setLoading(false);
    }
  }, [online]);

  const upsert = useCallback((entityType: EntityType, entity: JsonEntity) => run((store) => store.upsert(entityType, entity)), [run]);
  const remove = useCallback((entityType: EntityType, entityId: string) => run((store) => store.delete(entityType, entityId)), [run]);

  const setTheme = useCallback(async (mode: ThemeMode) => {
    setThemeState(mode);
    applyTheme(mode);
    if (!session || !storeRef.current || !online) return;
    const existing = Object.values(state.entities["user.preference"] ?? {}).find((item) => item.preferenceKey === "appearance.theme");
    const preference = existing
      ? { ...existing, value: mode }
      : createPreference(session.user.id, session.session.deviceId, "appearance.theme", mode);
    try { await upsert("user.preference", preference); }
    catch { /* Cookie theme remains usable even if sync is unavailable. */ }
  }, [online, session, state.entities, upsert]);

  const value = useMemo<AppContextValue>(() => ({
    session, state, authLoading, loading, online, error, privacy, theme,
    login, logout, refresh, run, upsert, remove,
    setPrivacy, setTheme, clearError: () => setError(""),
  }), [session, state, authLoading, loading, online, error, privacy, theme, login, logout, refresh, run, upsert, remove, setTheme]);

  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
}

export function useApp(): AppContextValue {
  const value = useContext(AppContext);
  if (!value) throw new Error("useApp must be used inside AppProvider");
  return value;
}
