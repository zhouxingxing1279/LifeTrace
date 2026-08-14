import type { ReactNode } from "react";

export function AppLoading({ children }: { children: ReactNode }) {
  return <main className="lt-app-state" aria-live="polite" aria-busy="true">
    <span className="lt-app-state-mark">LT</span>
    <div><strong>LifeTrace</strong><p>{children}</p></div>
  </main>;
}

export function OfflineGate() {
  return <main className="lt-app-state" role="status">
    <span className="lt-app-state-mark">×</span>
    <div>
      <strong>需要网络连接</strong>
      <p>LifeTrace Web 不在浏览器中保存业务数据库。恢复网络后即可继续使用。</p>
      <button className="hx-btn primary" onClick={() => window.location.reload()}>重新加载</button>
    </div>
  </main>;
}
