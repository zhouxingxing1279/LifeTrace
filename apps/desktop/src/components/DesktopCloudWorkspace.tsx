import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { AppShell as CloudAppShell } from "@/web-client/src/components/AppShell";
import { RouteView } from "@/web-client/src/components/RouteView";
import {
  AuthApi,
  CloudDataStore,
  EMPTY_CLOUD_STATE,
  setCloudFetchOverride,
  type CloudState,
  type FetchLike,
  type WebSession,
} from "@/web-client/src/core";
import { currentRoute, navigate, type Route } from "@/web-client/src/navigation";
import AppUpdaterHost from "@/src/components/AppUpdaterHost";
import { cloudAuthClient } from "@/src/services/cloudAuth";
import { useCloudAuthStore } from "@/src/stores/useCloudAuthStore";

function headersRecord(...sources: Array<HeadersInit | undefined>): Record<string, string> {
  const merged = new Headers();
  for (const source of sources) {
    if (!source) continue;
    new Headers(source).forEach((value, key) => merged.set(key, value));
  }
  return Object.fromEntries(merged.entries());
}

async function desktopCloudFetch(input: RequestInfo | URL, init: RequestInit = {}): Promise<Response> {
  const request = input instanceof Request ? input : undefined;
  const rawUrl = request?.url ?? (input instanceof URL ? input.toString() : String(input));
  const url = new URL(rawUrl, window.location.href);
  const path = `${url.pathname}${url.search}`;
  if (!path.startsWith("/api/") && !path.startsWith("/health")) {
    throw new Error(`桌面云工作台拒绝非 LifeTrace API 请求：${path}`);
  }

  const method = (init.method ?? request?.method ?? "GET").toUpperCase();
  let body = init.body;
  if (body === undefined && request && method !== "GET" && method !== "HEAD") {
    body = await request.clone().text();
  }
  if (body != null && typeof body !== "string") {
    if (body instanceof URLSearchParams) body = body.toString();
    else throw new Error("桌面云工作台当前只支持 JSON/文本请求体");
  }

  return cloudAuthClient.request(path, {
    ...init,
    method,
    headers: headersRecord(request?.headers, init.headers),
    body,
  });
}

function syncLocalReplica(): void {
  const api = window.syncApi;
  if (!api) return;
  void api.now(false).catch(() => undefined);
}

export default function DesktopCloudWorkspace() {
  // Install before child effects run. Every browser cloud API keeps using the
  // same exported browserFetch function, which now delegates to native auth.
  setCloudFetchOverride(desktopCloudFetch);

  const user = useCloudAuthStore((value) => value.user);
  const desktopSession = useCloudAuthStore((value) => value.session);
  const scopes = useCloudAuthStore((value) => value.scopes);
  const logout = useCloudAuthStore((value) => value.logout);
  const [route, setRoute] = useState<Route>(() => currentRoute());
  const [state, setState] = useState<CloudState>(EMPTY_CLOUD_STATE);
  const [networkOnline, setNetworkOnline] = useState(() => navigator.onLine);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [privacy, setPrivacy] = useState(false);
  const storeRef = useRef<CloudDataStore | null>(null);
  const auth = useMemo(() => new AuthApi(desktopCloudFetch), []);

  const session = useMemo<WebSession | null>(() => {
    if (!user || !desktopSession) return null;
    return {
      user: {
        id: user.id,
        email: user.email,
        displayName: user.displayName,
      },
      session: {
        id: desktopSession.id,
        appId: desktopSession.appId,
        deviceId: desktopSession.deviceId,
        scopes: [...scopes],
        idleExpiresAt: desktopSession.absoluteExpiresAt,
        absoluteExpiresAt: desktopSession.absoluteExpiresAt,
        publicDevice: false,
      },
      csrfToken: "",
    };
  }, [desktopSession, scopes, user]);

  useEffect(() => () => setCloudFetchOverride(undefined), []);

  useEffect(() => {
    const routeChanged = () => {
      setRoute(currentRoute());
      const reduceMotion = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
      window.scrollTo({ top: 0, behavior: reduceMotion ? "auto" : "smooth" });
    };
    const wentOnline = () => setNetworkOnline(true);
    const wentOffline = () => setNetworkOnline(false);
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
    if (!session) {
      storeRef.current = null;
      setState(EMPTY_CLOUD_STATE);
      return () => { active = false; };
    }

    const store = new CloudDataStore(session.user.id, session.session.deviceId, "", desktopCloudFetch);
    storeRef.current = store;
    setLoading(true);
    setError("");
    void store.load()
      .then((next) => { if (active) setState(next); })
      .catch((cause: unknown) => {
        if (active) setError(cause instanceof Error ? cause.message : "无法加载云端数据");
      })
      .finally(() => { if (active) setLoading(false); });

    return () => { active = false; };
  }, [session?.session.deviceId, session?.user.id]);

  const refresh = useCallback(async () => {
    const store = storeRef.current;
    if (!store || !navigator.onLine) return;
    setLoading(true);
    setError("");
    try {
      setState(await store.refresh());
      syncLocalReplica();
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
      syncLocalReplica();
      return next;
    } catch (cause) {
      setState(store.snapshot());
      setError(cause instanceof Error ? cause.message : "云端操作失败");
      throw cause;
    } finally {
      setLoading(false);
    }
  }, []);

  if (!session) {
    return <div className="hx-loading"><span>LT</span><p>正在恢复桌面云会话…</p></div>;
  }

  return <>
    <CloudAppShell
      route={route}
      session={session}
      online={networkOnline}
      loading={loading}
      privacy={privacy}
      error={error}
      conflictCount={state.conflicts.length}
      onRefresh={() => void refresh()}
      onTogglePrivacy={() => setPrivacy((value) => !value)}
      onLogout={() => void logout().finally(() => navigate("/"))}
    >
      <RouteView
        route={route}
        auth={auth}
        session={session}
        state={state}
        privacy={privacy}
        online={networkOnline}
        run={run}
      />
    </CloudAppShell>
    <AppUpdaterHost />
  </>;
}
