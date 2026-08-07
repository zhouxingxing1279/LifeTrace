import { useEffect } from "react";
import { LogIn, LoaderCircle, ShieldCheck } from "lucide-react";
import HengXuShell from "@/src/components/HengXuShell";
import { AccountEntry, AccountEntryHost } from "@/src/components/account/AccountEntry";
import { clientLogger } from "@/src/services/clientObservability";
import { useCloudAuthStore } from "@/src/stores/useCloudAuthStore";

function SignedOutShell({ restoring }: { restoring: boolean }) {
  const openLogin = () => window.dispatchEvent(new Event("lifetrace:open-auth"));

  return <main className="hx-shell hx-auth-shell">
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
  </main>;
}

export default function DesktopApp() {
  const user = useCloudAuthStore((state) => state.user);
  const authenticated = useCloudAuthStore((state) => state.authenticated);
  const phase = useCloudAuthStore((state) => state.phase);
  const initialize = useCloudAuthStore((state) => state.initialize);

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

  const hasIdentity = Boolean(user && (authenticated || phase === "offline"));
  const restoring = phase === "bootstrapping" || phase === "refreshing";

  if (!hasIdentity) {
    return <SignedOutShell restoring={restoring}/>;
  }

  return <><HengXuShell/><AccountEntryHost/></>;
}
