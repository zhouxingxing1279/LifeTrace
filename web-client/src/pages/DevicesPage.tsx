import { useCallback, useEffect, useMemo, useState } from "react";
import { WebManagementApi, type AuthApi, type DeviceInstallation, type ManagedSession, type WebSession } from "../core";
import { Empty, Notice, PageStack, Panel } from "../ui";

export function DevicesPage({ session, auth, online }: { session: WebSession; auth: AuthApi; online: boolean }) {
  const management = useMemo(() => new WebManagementApi(), [auth]);
  const [devices, setDevices] = useState<DeviceInstallation[]>([]);
  const [sessions, setSessions] = useState<ManagedSession[]>([]);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    if (!online) return;
    setLoading(true); setError("");
    try {
      const [nextDevices, nextSessions] = await Promise.all([management.devices(), management.sessions()]);
      setDevices(nextDevices); setSessions(nextSessions);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "设备信息加载失败");
    } finally { setLoading(false); }
  }, [management, online]);

  useEffect(() => { void load(); }, [load]);

  async function rename(device: DeviceInstallation) {
    const name = window.prompt("设备名称", device.deviceName);
    if (!name?.trim()) return;
    try { await management.renameDevice(device.id, name, session.csrfToken); await load(); }
    catch (cause) { setError(cause instanceof Error ? cause.message : "重命名失败"); }
  }

  async function revokeDevice(device: DeviceInstallation) {
    if (!window.confirm(`撤销设备“${device.deviceName}”？该设备需要重新登录。`)) return;
    try { await management.revokeDevice(device.id, session.csrfToken); await load(); }
    catch (cause) { setError(cause instanceof Error ? cause.message : "撤销失败"); }
  }

  async function revokeSession(item: ManagedSession) {
    if (!window.confirm("退出该登录会话？")) return;
    try { await management.revokeSession(item.id, session.csrfToken); await load(); }
    catch (cause) { setError(cause instanceof Error ? cause.message : "退出会话失败"); }
  }

  return <PageStack>{error && <Notice kind="error">{error}</Notice>}<Panel title="已登录设备" eyebrow="DEVICES"><div className="device-list">{devices.map((item) => <div key={item.id}><span className={`device-icon ${item.current ? "current" : ""}`}>{item.platform.slice(0, 1).toUpperCase()}</span><div><strong>{item.deviceName}</strong><small>{item.appId} · 最近活动 {new Date(item.lastSeenAt).toLocaleString("zh-CN")}</small></div>{item.current && <span className="status-pill online">当前设备</span>}<button className="small-button" onClick={() => void rename(item)}>重命名</button>{!item.current && <button className="small-button danger" onClick={() => void revokeDevice(item)}>撤销</button>}</div>)}{!devices.length && !loading && <Empty title="没有设备记录" description="完成一次云端登录后会显示。" />}</div></Panel><Panel title="活动会话" eyebrow="SESSIONS"><div className="device-list">{sessions.map((item) => <div key={item.id}><span className="device-icon">S</span><div><strong>{item.appId}</strong><small>{item.sessionType} · 到期 {new Date(item.absoluteExpiresAt).toLocaleString("zh-CN")}</small></div>{item.current && <span className="status-pill online">当前</span>}{!item.current && <button className="small-button danger" onClick={() => void revokeSession(item)}>退出该会话</button>}</div>)}{!sessions.length && !loading && <Empty title="没有活动会话" description="当前没有可管理的会话。" />}</div></Panel></PageStack>;
}
