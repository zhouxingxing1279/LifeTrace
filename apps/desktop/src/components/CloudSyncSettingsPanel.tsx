import { useEffect, useState } from "react";
import { Cloud, RefreshCw } from "lucide-react";
import { useCloudAuthStore } from "@/src/stores/useCloudAuthStore";
import type { SyncConflictView, SyncStatusView } from "@/src/services/cloudSync";

const phaseLabel: Record<string, string> = {
  disabled: "同步已关闭",
  local_only: "仅本地",
  auth_required: "需要重新登录",
  idle: "等待同步",
  initializing_snapshot: "正在初始化",
  pushing: "正在上传",
  pulling: "正在下载",
  up_to_date: "同步正常",
  offline: "当前离线",
  backoff: "等待重试",
  conflict: "存在冲突",
  error: "同步异常",
};

export default function CloudSyncSettingsPanel() {
  const auth = useCloudAuthStore();
  const [status, setStatus] = useState<SyncStatusView>();
  const [conflicts, setConflicts] = useState<SyncConflictView[]>([]);
  const [syncing, setSyncing] = useState(false);

  const refresh = async () => {
    if (!window.syncApi) return;
    const next = await window.syncApi.status();
    setStatus(next);
    setConflicts(next.conflictCount > 0 ? await window.syncApi.conflicts() : []);
  };

  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => void refresh(), 15_000);
    return () => window.clearInterval(timer);
  }, [auth.authenticated, auth.binding?.profileId]);

  const runSync = async (forceSnapshot = false) => {
    if (!window.syncApi || !auth.authenticated) return;
    setSyncing(true);
    auth.clearError();
    try {
      await window.syncApi.now(forceSnapshot);
    } catch (error) {
      useCloudAuthStore.setState({ error: error instanceof Error ? error.message : String(error) });
    } finally {
      setSyncing(false);
      await refresh();
    }
  };

  if (!auth.authenticated) {
    return <section id="settings-sync" className="hx-settings-page-section">
      <header><h2>云同步</h2><p>登录后，当前账号的数据会同步到该账号自己的云端空间。</p></header>
      <div className="hx-setting-rows">
        <div className="hx-setting-row"><div><strong>LifeTrace 账号</strong><small>{auth.phase === "offline" ? "当前无法连接云端，已保留本机登录凭据" : "当前未登录"}</small></div><button className="hx-btn primary" type="button" onClick={() => window.dispatchEvent(new Event("lifetrace:open-auth"))}>登录 / 注册</button></div>
      </div>
    </section>;
  }

  return <section id="settings-sync" className="hx-settings-page-section">
    <header><h2>云同步</h2><p>每个账号拥有独立数据空间；切换账号时会自动切换到对应的本地 Profile。</p></header>
    <div className="hx-setting-rows">
      <div className="hx-setting-row"><div><strong>同步状态</strong><small>{status?.lastErrorMessage || "LifeTrace 会自动在后台保持同步"}</small></div><span className={`hx-setting-status ${status?.phase === "error" || status?.phase === "conflict" ? "warning" : ""}`}><Cloud />{status ? phaseLabel[status.phase] || status.phase : "读取中"}</span></div>
      <div className="hx-setting-row"><div><strong>当前账号</strong><small>当前数据空间只属于该账号</small></div><span>{auth.user?.email}</span></div>
      <div className="hx-setting-row"><div><strong>最后同步</strong><small>{status?.pendingCount ? `还有 ${status.pendingCount} 项等待上传` : "没有待上传的数据"}</small></div><span>{status?.lastSuccessAt ? new Date(status.lastSuccessAt).toLocaleString("zh-CN") : "尚未完成"}</span></div>
      <div className="hx-setting-row"><div><strong>手动同步</strong><small>通常无需操作；排查同步问题时可以手动执行。</small></div><button className="hx-btn secondary" type="button" disabled={syncing || auth.loading} onClick={() => void runSync(false)}><RefreshCw className={syncing ? "spin" : ""} />{syncing ? "同步中…" : "立即同步"}</button></div>
    </div>

    {conflicts.length > 0 && <div className="hx-settings-conflicts">
      <h3>需要处理的冲突</h3>
      {conflicts.map((item) => <div key={item.conflictId}><span><strong>{item.entityType}</strong><small>{item.entityId}</small></span><div><button className="hx-btn secondary" onClick={async () => { await window.syncApi?.resolveConflict(item.conflictId, "accept_remote"); await refresh(); }}>接受云端</button><button className="hx-btn secondary" onClick={async () => { await window.syncApi?.resolveConflict(item.conflictId, "keep_local"); await refresh(); }}>保留本地</button></div></div>)}
    </div>}

    <details className="hx-settings-advanced"><summary>高级同步操作</summary><p>“从云端重新初始化”会重新获取当前账号的云端快照，仅建议在排查同步异常时使用。</p><button className="hx-btn secondary" type="button" disabled={syncing || auth.loading} onClick={() => void runSync(true)}>从云端重新初始化</button></details>
    {auth.error && <p className="hx-inline-message" role="alert">{auth.error}</p>}
  </section>;
}
