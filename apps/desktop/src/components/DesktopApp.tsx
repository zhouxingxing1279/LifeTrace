import { useEffect } from "react";
import { LogIn, LoaderCircle, ShieldCheck } from "lucide-react";
import DesktopProviders from "@/src/app/DesktopProviders";
import HengXuShell from "@/src/components/HengXuShell";
import AppUpdaterHost from "@/src/components/AppUpdaterHost";
import { AccountEntry, AccountEntryHost } from "@/src/components/account/AccountEntry";
import { clientLogger } from "@/src/services/clientObservability";
import { useCloudAuthStore } from "@/src/stores/useCloudAuthStore";

const OFFLINE_RECONNECT_INTERVAL_MS = 10_000;

function SignedOutShell({ restoring }: { restoring: boolean }) {
  const openLogin = () => window.dispatchEvent(new Event("lifetrace:open-auth"));

  return <>
    <main className="hx-shell hx-auth-shell">
      <aside aria-label="LifeTrace">
        <div className="hx-brand"><span>LT</span><div><strong>Life trace</strong><small>个人管理系统</small></div></div>
        <div className="hx-auth-sidebar-note"><ShieldCheck/><span>账号数据相互隔离</span></div>
        <div className="hx-sidebar-foot"><AccountEntry autoOpen={!restoring}/></div>
      </aside>
      <div className="hx-main hx-signed-out-main">
        <section className="hx-signed-out-card" aria-live="polite">
          <span className="hx-signed-out-mark">LT</span>
          <h1>{restoring ? "正在恢复登录状态" : "登录 LifeTrace"}</h1>
          <p>{restoring ? "正在安全验证本机保存的登录凭据，请稍候。" : "登录后才能查看你的坚持、账单、笔记和其他个人数据。退出登录后，这些数据不会继续显示。"}</p>
          <button className="hx-btn primary" type="button" disabled={restoring} onClick={openLogin}>
            {restoring ? <><LoaderCircle className="spin"/>正在恢复…</> : <><LogIn/>登录 / 注册</>}
          </button>
        </section>
      </div>
    </main>
    <AppUpdaterHost />
  </>;
}

export default function DesktopApp() {
  const user = useCloudAuthStore((state) => state.user);
  const authenticated = useCloudAuthStore((state) => state.authenticated);
  const phase = useCloudAuthStore((state) => state.phase);
  const initialize = useCloudAuthStore((state) => state.initialize);
  const reconnect = useCloudAuthStore((state) => state.reconnect);

  useEffect(() => {
    clientLogger.info("cloud.auth.auto_restore_started");
    void initialize().then(() => {
      const state = useCloudAuthStore.getState();
      clientLogger.info("cloud.auth.auto_restore_finished", {
        authenticated: state.authenticated,
        phase: state.phase,
        profileId: state.binding?.profileId,
      });
    }).catch((error) => {
      clientLogger.warn("cloud.auth.auto_restore_failed", undefined, error);
    });
  }, [initialize]);

  useEffect(() => {
    if (phase !== "offline" || !user) return;

    const retry = () => {
      void reconnect().then((restored) => {
        if (restored) clientLogger.info("cloud.auth.offline_reconnect_succeeded");
      }).catch((error) => {
        clientLogger.warn("cloud.auth.offline_reconnect_failed", undefined, error);
      });
    };

    retry();
    const timer = window.setInterval(retry, OFFLINE_RECONNECT_INTERVAL_MS);
    window.addEventListener("online", retry);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener("online", retry);
    };
  }, [phase, reconnect, user]);

  const hasIdentity = Boolean(user && (authenticated || phase === "offline"));
  const restoring = phase === "bootstrapping" || phase === "refreshing";

  if (!hasIdentity) {
    return <SignedOutShell restoring={restoring}/>;
  }

  // Desktop is always local-first once an identity is known. Cloud connectivity
  // only controls background synchronization; it no longer swaps the application
  // into the Web feature runtime.
  return (
    <DesktopProviders>
      <HengXuShell />
      <AccountEntryHost />
    </DesktopProviders>
  );
}
