import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { AuthApi, CloudDataStore, EMPTY_CLOUD_STATE, type CloudState, type WebSession } from "./core";
import { AuthScreen } from "./AuthScreen";
import { AppShell } from "./components/AppShell";
import { AppLoading, OfflineGate } from "./components/AppStates";
import { RouteView } from "./components/RouteView";
import { currentRoute, navigate, type Route } from "./navigation";
import { entities, text } from "./ui";

export default function App() {
  const auth = useMemo(() => new AuthApi(), []);
  const [session, setSession] = useState<WebSession | null>(null);
  const [route, setRoute] = useState<Route>(currentRoute());
  const [state, setState] = useState<CloudState>(EMPTY_CLOUD_STATE);
  const [authLoading, setAuthLoading] = useState(true);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [online, setOnline] = useState(navigator.onLine);
  const [privacy, setPrivacy] = useState(false);
  const storeRef = useRef<CloudDataStore | null>(null);

  useEffect(() => {
    const routeChanged = () => {
      setRoute(currentRoute());
      const reduceMotion = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
      window.scrollTo({ top: 0, behavior: reduceMotion ? "auto" : "smooth" });
    };
    const wentOnline = () => setOnline(true);
    const wentOffline = () => setOnline(false);
    window.addEventListener("popstate", routeChanged);
    window.addEventListener("online", wentOnline);
    window.addEventListener("offline", wentOffline);
    return () => {
      window.removeEventListener("popstate", routeChanged);
      window.removeEventListener("online", wentOnline);
      window.removeEventListener("offline", wentOffline);
    };
  }, []);

  useEffect(() => {
    let active = true;
    if (!navigator.onLine) {
      setAuthLoading(false);
      return;
    }
    auth.session()
      .then((value) => { if (active) setSession(value); })
      .catch((cause: unknown) => {
        if (active && cause instanceof Error && !/401|unauth|authentication|session/i.test(cause.message)) setError(cause.message);
      })
      .finally(() => { if (active) setAuthLoading(false); });
    return () => { active = false; };
  }, [auth]);

  useEffect(() => {
    if (!session) {
      storeRef.current = null;
      setState(EMPTY_CLOUD_STATE);
      return;
    }
    const store = new CloudDataStore(session.user.id, session.session.deviceId, session.csrfToken);
    storeRef.current = store;
    setLoading(true);
    setError("");
    store.load()
      .then(setState)
      .catch((cause: unknown) => setError(cause instanceof Error ? cause.message : "无法加载云端数据"))
      .finally(() => setLoading(false));
  }, [session?.user.id, session?.session.deviceId, session?.csrfToken]);

  useEffect(() => {
    const preference = entities(state, "user.preference").find((item) => text(item, "preferenceKey") === "appearance.theme");
    document.documentElement.dataset.theme = preference?.value === "dark" ? "dark" : "light";
  }, [state]);

  const refresh = useCallback(async () => {
    if (!storeRef.current || !navigator.onLine) return;
    setLoading(true);
    setError("");
    try {
      setState(await storeRef.current.refresh());
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "刷新失败");
    } finally {
      setLoading(false);
    }
  }, []);

  const run = useCallback(async (action: (store: CloudDataStore) => Promise<CloudState>) => {
    const store = storeRef.current;
    if (!store) throw new Error("云端数据服务尚未就绪");
    if (!navigator.onLine) throw new Error("当前无网络，数据未保存");
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
  }, []);

  async function logout() {
    try {
      if (navigator.onLine) await auth.logout(session?.csrfToken);
    } finally {
      storeRef.current?.reset();
      storeRef.current = null;
      setSession(null);
      setState(EMPTY_CLOUD_STATE);
      navigate("/");
    }
  }

  if (authLoading) return <AppLoading>正在验证云端会话…</AppLoading>;
  if (!online && !session) return <OfflineGate />;
  if (!session) return <AuthScreen auth={auth} error={error} onAuthenticated={setSession} />;

  return <AppShell
    route={route}
    session={session}
    online={online}
    loading={loading}
    privacy={privacy}
    error={error}
    conflictCount={state.conflicts.length}
    onRefresh={() => void refresh()}
    onTogglePrivacy={() => setPrivacy((value) => !value)}
    onLogout={() => void logout()}
  >
    <RouteView
      route={route}
      auth={auth}
      session={session}
      state={state}
      privacy={privacy}
      online={online}
      run={run}
    />
  </AppShell>;
}
