import { useEffect, useState } from "react";
import { Cloud, CloudCog, Database, LogIn, LogOut, RefreshCw, ShieldCheck, TriangleAlert } from "lucide-react";
import { useCloudAuthStore } from "@/src/stores/useCloudAuthStore";
import type { SyncConflictView, SyncStatusView } from "@/src/services/cloudSync";

const phaseLabel: Record<string,string> = {
  disabled:"同步已关闭", local_only:"本地模式", auth_required:"需要重新登录", idle:"等待同步",
  initializing_snapshot:"正在初始化云端副本", pushing:"正在上传", pulling:"正在下载",
  up_to_date:"同步完成", offline:"离线", backoff:"等待重试", conflict:"存在冲突", error:"同步错误",
};

export default function CloudAccountPanel() {
  const auth = useCloudAuthStore();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [syncStatus, setSyncStatus] = useState<SyncStatusView>();
  const [conflicts, setConflicts] = useState<SyncConflictView[]>([]);
  const [syncing, setSyncing] = useState(false);

  const refreshStatus = async () => {
    if (!window.syncApi) return;
    const status = await window.syncApi.status();
    setSyncStatus(status);
    if (status.conflictCount > 0) setConflicts(await window.syncApi.conflicts());
    else setConflicts([]);
  };

  useEffect(() => {
    void refreshStatus();
    const timer = window.setInterval(() => void refreshStatus(), 15_000);
    return () => window.clearInterval(timer);
  }, [auth.authenticated, auth.binding?.profileId]);

  const runSync = async (forceSnapshot = false) => {
    if (!window.syncApi) return;
    setSyncing(true);
    auth.clearError();
    try { await window.syncApi.now(forceSnapshot); }
    catch (error) { useCloudAuthStore.setState({ error: error instanceof Error ? error.message : String(error) }); }
    finally { setSyncing(false); await refreshStatus(); }
  };

  const ownershipLabel = auth.binding?.bindingRequired
    ? "等待你选择归属"
    : auth.authenticated
      ? "已绑定当前云账号"
      : "仅属于本机资料";
  const profileId = auth.binding?.profileId;

  return <section className="hx-settings-section hx-cloud-account">
    <header className="hx-settings-section-head">
      <div><span>账号、归属与同步</span><h2>LifeTrace Cloud</h2></div>
      <i className={auth.authenticated ? "configured" : ""}><Cloud /></i>
    </header>
    <div className="hx-settings-section-body hx-settings-form">
      <div className="hx-local-ownership">
        <div className="hx-local-ownership-title">
          <div><Database /><span><strong>未登录数据归属</strong><small>{profileId ? `本地资料 ${profileId.slice(0, 8)}…` : "当前设备的本地资料"}</small></span></div>
          <b>{ownershipLabel}</b>
        </div>
        <p>未登录时，新增记录会立即归属于当前本地资料，而不是“匿名用户”或临时待认领数据。前端传入的用户标识不会决定归属，后端会统一写入当前本地资料 ID。</p>
        <div className="hx-ownership-rules">
          <span><strong>离线记录</strong><small>始终写入本地 Profile，重启后仍保持归属。</small></span>
          <span><strong>首次登录</strong><small>只建立会话，不会自动上传或改写历史数据。</small></span>
          <span><strong>显式选择</strong><small>绑定现有资料，或新建空白云端资料，二选一。</small></span>
        </div>
      </div>

      <p className="hx-cloud-local-note"><ShieldCheck /> 无论是否登录，SQLite 本地数据和全部本地功能都可继续使用。</p>
      <label>云服务地址<input value={auth.origin} placeholder="https://cloud.example.com" onChange={event=>auth.setOrigin(event.target.value)} /></label>
      {!auth.authenticated ? <>
        <label>邮箱<input type="email" autoComplete="username" value={email} onChange={event=>setEmail(event.target.value)} /></label>
        <label>密码<input type="password" autoComplete="current-password" value={password} onChange={event=>setPassword(event.target.value)} /></label>
        <div className="hx-settings-section-actions"><button className="hx-btn primary" disabled={auth.loading||!auth.origin||!email||!password} onClick={async()=>{await auth.login(email,password);setPassword("")}}><LogIn /> {auth.loading?"正在登录…":"登录云账号"}</button></div>
      </> : <>
        <div className="hx-cloud-user"><strong>{auth.user?.displayName||auth.user?.email}</strong><small>{auth.user?.email}</small><span>当前会话：{auth.session?.status} · 权限 {auth.scopes.length} 项</span></div>
        {auth.binding?.bindingRequired && <div className="hx-cloud-binding"><strong><TriangleAlert/>选择当前本地数据的归属</strong><p>登录不会自动上传。绑定当前资料会保留现有本地数据；创建空白资料会保留旧资料在本机，并切换到新的云端资料。</p><div className="hx-settings-actions"><button className="hx-btn primary" disabled={auth.loading} onClick={()=>auth.bindCurrentProfile()}>绑定当前本地资料</button><button className="hx-btn secondary" disabled={auth.loading} onClick={()=>auth.createCloudProfile()}>创建空白云端资料</button></div></div>}
        {!auth.binding?.bindingRequired && syncStatus && <div className="hx-sync-status"><div><CloudCog/><strong>{phaseLabel[syncStatus.phase]||syncStatus.phase}</strong></div><span>待上传 {syncStatus.pendingCount} · 冲突 {syncStatus.conflictCount}</span>{syncStatus.lastSuccessAt&&<small>最后成功：{new Date(syncStatus.lastSuccessAt).toLocaleString()}</small>}{syncStatus.lastErrorMessage&&<small className="negative">{syncStatus.lastErrorMessage}</small>}<div className="hx-settings-actions"><button className="hx-btn primary" disabled={syncing||auth.loading} onClick={()=>runSync(false)}><RefreshCw/>{syncing?"同步中…":"立即同步"}</button><button className="hx-btn secondary" disabled={syncing||auth.loading} onClick={()=>runSync(true)}>从云端重新初始化</button></div></div>}
        {conflicts.length>0&&<div className="hx-sync-conflicts"><strong>需要处理的冲突</strong>{conflicts.map(item=><div key={item.conflictId}><span>{item.entityType} · {item.entityId}</span><div className="hx-settings-actions"><button className="hx-btn secondary" onClick={async()=>{await window.syncApi?.resolveConflict(item.conflictId,"accept_remote");await refreshStatus()}}>接受云端</button><button className="hx-btn secondary" onClick={async()=>{await window.syncApi?.resolveConflict(item.conflictId,"keep_local");await refreshStatus()}}>保留本地</button></div></div>)}</div>}
        <div className="hx-settings-actions"><button className="hx-btn secondary" disabled={auth.loading} onClick={()=>auth.logout(false)}><LogOut />退出当前设备</button><button className="hx-btn secondary" disabled={auth.loading} onClick={()=>auth.logout(true)}>退出全部设备</button></div>
      </>}
      {auth.error&&<p className="hx-inline-message" role="alert">{auth.error}</p>}
      <small>Access Token 仅保存在 Rust/前端进程内存；Refresh Token 仅保存在 Windows Credential Manager，不写入 SQLite、JSON 或 localStorage。</small>
    </div>
  </section>;
}
