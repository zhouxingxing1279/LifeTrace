import { useEffect, useMemo, useState } from "react";
import { Cloud, Database, Info, Laptop, LogOut, Moon, RefreshCw, Shield, Smartphone, Sun, UserRound } from "lucide-react";
import { useApp } from "../../app/AppContext";
import { Badge, Button, Card, CardContent, PageHeader, Section, Switch, cn } from "../../components/ui";
import { ENTITY_TYPES, APP_VERSION, WebManagementApi, type DeviceInstallation, type ManagedSession } from "../../services/core";

export function SettingsPage() {
  const { session, state, online, privacy, setPrivacy, theme, setTheme, refresh, logout } = useApp();
  const management = useMemo(() => new WebManagementApi(), []);
  const [devices, setDevices] = useState<DeviceInstallation[]>([]);
  const [sessions, setSessions] = useState<ManagedSession[]>([]);
  const [managementError, setManagementError] = useState("");
  const [managementLoading, setManagementLoading] = useState(false);
  const modes = ["system", "light", "dark"] as const;
  const totalEntities = ENTITY_TYPES.reduce((total, entityType) => total + Object.values(state.entities[entityType] ?? {}).filter((item) => !item.meta.deletedAt).length, 0);

  async function loadManagement() {
    if (!online || !session) return;
    setManagementLoading(true);
    setManagementError("");
    try {
      const [deviceRows, sessionRows] = await Promise.all([management.devices(), management.sessions()]);
      setDevices(deviceRows);
      setSessions(sessionRows);
    } catch (cause) {
      setManagementError(cause instanceof Error ? cause.message : "无法加载设备与会话");
    } finally {
      setManagementLoading(false);
    }
  }

  useEffect(() => {
    void loadManagement();
    // Management API instance and current session are stable while the page is mounted.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [online, session?.user.id]);

  async function revokeSession(sessionId: string) {
    if (!session) return;
    await management.revokeSession(sessionId, session.csrfToken);
    await loadManagement();
  }

  async function revokeDevice(deviceId: string) {
    if (!session) return;
    await management.revokeDevice(deviceId, session.csrfToken);
    await loadManagement();
  }

  return <div className="page-shell">
    <PageHeader title="设置" description="Profile、Appearance、Cloud & Sync、Devices、Privacy、Security、Data、About 与 Danger Zone。结构参考 Catalyst Settings。" />

    <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_360px]">
      <div className="space-y-6">
        <Section title="Profile" description="当前 LifeTrace Cloud 身份。">
          <Card><CardContent className="pt-5"><div className="flex items-center gap-4"><div className="flex h-11 w-11 items-center justify-center rounded-full bg-muted"><UserRound size={19} /></div><div className="min-w-0"><div className="font-semibold">{session?.user.displayName || "LifeTrace 用户"}</div><div className="truncate text-sm text-muted-foreground">{session?.user.email}</div></div></div></CardContent></Card>
        </Section>

        <Section title="Appearance" description="主题使用语义 Token；system 模式跟随操作系统。">
          <Card><CardContent className="pt-5"><div className="grid gap-2 sm:grid-cols-3">{modes.map((mode) => <button key={mode} onClick={() => void setTheme(mode)} className={cn("rounded-lg border p-4 text-left", theme === mode && "border-primary bg-accent")}><div className="flex items-center gap-2 font-medium">{mode === "dark" ? <Moon size={16} /> : <Sun size={16} />}{({ system: "跟随系统", light: "浅色", dark: "深色" } as const)[mode]}</div><div className="mt-1 text-xs text-muted-foreground">{mode === "system" ? "根据系统深浅色自动切换" : "固定显示主题"}</div></button>)}</div></CardContent></Card>
        </Section>

        <Section title="Cloud & Sync" description="使用现有 LifeTrace Cloud 同步协议和冲突记录。" action={<Button size="sm" variant="outline" onClick={() => void refresh()} disabled={!online}><RefreshCw size={14} />立即同步</Button>}>
          <Card><CardContent className="space-y-3 pt-5"><div className="flex items-center justify-between"><span className="flex items-center gap-2 text-sm"><Cloud size={16} />网络</span><Badge className={online ? "text-success" : "text-warning"}>{online ? "在线" : "离线"}</Badge></div><div className="flex items-center justify-between"><span className="text-sm text-muted-foreground">最近云端加载</span><span className="text-xs">{state.lastLoadedAt ? new Date(state.lastLoadedAt).toLocaleString("zh-CN") : "—"}</span></div><div className="flex items-center justify-between"><span className="text-sm text-muted-foreground">冲突记录</span><span className="text-xs">{state.conflicts.length}</span></div></CardContent></Card>
        </Section>

        <Section title="Devices" description="来自现有 `/api/v1/web/devices` 管理接口。" action={<Button size="sm" variant="ghost" disabled={managementLoading || !online} onClick={() => void loadManagement()}><RefreshCw size={14} />刷新</Button>}>
          <Card>{devices.length ? <div className="divide-y">{devices.map((device) => <div key={device.id} className="flex items-center gap-3 px-4 py-3"><div className="flex h-9 w-9 items-center justify-center rounded-md bg-muted">{device.platform.toLowerCase().includes("mobile") || device.platform.toLowerCase().includes("android") || device.platform.toLowerCase().includes("ios") ? <Smartphone size={16} /> : <Laptop size={16} />}</div><div className="min-w-0 flex-1"><div className="flex flex-wrap items-center gap-2 text-sm font-medium">{device.deviceName}{device.current ? <Badge>当前设备</Badge> : null}</div><div className="mt-0.5 text-xs text-muted-foreground">{device.platform} · 最近在线 {new Date(device.lastSeenAt).toLocaleString("zh-CN")}</div></div>{!device.current && !device.revokedAt ? <Button size="sm" variant="outline" onClick={() => void revokeDevice(device.id)}>撤销</Button> : null}</div>)}</div> : <CardContent className="pt-5"><div className="text-sm text-muted-foreground">{managementError || (managementLoading ? "正在加载设备…" : "没有可显示的设备")}</div></CardContent>}</Card>
        </Section>

        <Section title="Privacy" description="隐私金额仅改变显示，不改变云端业务数据。">
          <Card><CardContent className="pt-5"><div className="flex items-center justify-between gap-4"><div><div className="font-medium">隐藏金额</div><div className="mt-1 text-xs text-muted-foreground">Dashboard 与财务页使用掩码显示金额。</div></div><Switch label="隐藏金额" checked={privacy} onCheckedChange={setPrivacy} /></div></CardContent></Card>
        </Section>

        <Section title="Security" description="查看并撤销当前账号的 Web 会话。">
          <Card>{sessions.length ? <div className="divide-y">{sessions.map((managed) => <div key={managed.id} className="flex items-center gap-3 px-4 py-3"><Shield size={16} className="text-muted-foreground" /><div className="min-w-0 flex-1"><div className="flex flex-wrap items-center gap-2 text-sm font-medium">{managed.appId} · {managed.deviceId}{managed.current ? <Badge>当前会话</Badge> : null}</div><div className="mt-0.5 text-xs text-muted-foreground">最后活动 {new Date(managed.lastSeenAt).toLocaleString("zh-CN")} · {managed.publicDevice ? "公共设备" : "私人设备"}</div></div>{!managed.current && !managed.revokedAt ? <Button size="sm" variant="outline" onClick={() => void revokeSession(managed.id)}>退出此会话</Button> : null}</div>)}</div> : <CardContent className="pt-5"><div className="text-sm text-muted-foreground">{managementError || (managementLoading ? "正在加载会话…" : "没有可显示的会话")}</div></CardContent>}</Card>
        </Section>

        <Section title="Data" description="浏览器只保留当前内存快照，不持久化业务实体。">
          <Card><CardContent className="pt-5"><div className="flex items-start gap-3"><Database size={18} className="mt-0.5 text-muted-foreground" /><div><div className="font-medium">当前云端快照：{totalEntities} 条实体</div><p className="mt-1 text-xs leading-5 text-muted-foreground">覆盖 {ENTITY_TYPES.length} 个同步实体类型。Web 不使用浏览器本地数据库作为业务数据真相源。</p></div></div></CardContent></Card>
        </Section>

        <Section title="About">
          <Card><CardContent className="pt-5"><div className="flex items-start gap-3"><Info size={18} className="mt-0.5 text-muted-foreground" /><div><div className="font-medium">LifeTrace Web {APP_VERSION}</div><p className="mt-1 text-xs leading-5 text-muted-foreground">独立 `apps/web` Personal OS 客户端。财务工作区保留 BeeCount Cloud 上游归属与许可记录。</p></div></div></CardContent></Card>
        </Section>

        <Section title="Danger Zone" description="高风险操作与普通设置分离。">
          <Card className="border-destructive/30"><CardContent className="pt-5"><div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between"><div><div className="font-medium text-destructive">退出当前账号</div><div className="mt-1 text-xs text-muted-foreground">清除当前 Web 会话和内存中的 Cloud 状态。</div></div><Button variant="destructive" onClick={() => void logout()}><LogOut size={15} />退出登录</Button></div></CardContent></Card>
        </Section>
      </div>

      <aside className="space-y-5 xl:sticky xl:top-20 xl:h-fit">
        <Card><CardContent className="pt-5"><div className="flex h-10 w-10 items-center justify-center rounded-full bg-muted"><Laptop size={18} /></div><div className="mt-4 font-semibold">当前会话</div><div className="mt-1 break-all text-xs text-muted-foreground">{session?.user.email}</div><div className="mt-4 space-y-2 text-xs"><div className="flex justify-between"><span className="text-muted-foreground">App</span><span>{session?.session.appId}</span></div><div className="flex justify-between"><span className="text-muted-foreground">Device</span><span className="max-w-40 truncate">{session?.session.deviceId}</span></div><div className="flex justify-between"><span className="text-muted-foreground">Scopes</span><span>{session?.session.scopes.length ?? 0}</span></div></div></CardContent></Card>
        <Card><CardContent className="pt-5"><div className="flex items-center gap-2 font-semibold"><Shield size={16} />安全边界</div><p className="mt-2 text-xs leading-5 text-muted-foreground">认证使用 HttpOnly Cloud Session 与 CSRF 契约；业务记录不落浏览器本地持久化。</p></CardContent></Card>
      </aside>
    </div>
  </div>;
}
