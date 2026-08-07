import { useEffect, useRef, useState } from "react";
import {
  ChevronRight,
  Cloud,
  Eye,
  EyeOff,
  LoaderCircle,
  LogIn,
  LogOut,
  Server,
  Settings,
  ShieldCheck,
  UserCircle,
  X,
} from "lucide-react";
import { useCloudAuthStore } from "@/src/stores/useCloudAuthStore";
import { confirmAction } from "@/src/ui/feedback/confirm";

export type SettingsSection = "general" | "sync" | "ai" | "translation" | "security" | "about";

type AuthDialogMode = "login" | "register" | "binding";

function initials(value?: string | null) {
  const text = value?.trim();
  if (!text) return "LT";
  return text.slice(0, 2).toUpperCase();
}

function AccountDialog({ initialMode, close }: { initialMode: "login" | "register"; close: () => void }) {
  const auth = useCloudAuthStore();
  const [mode, setMode] = useState<AuthDialogMode>(initialMode);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [inviteToken, setInviteToken] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [advanced, setAdvanced] = useState(false);
  const [message, setMessage] = useState("");
  const emailRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    auth.clearError();
    void auth.loadCapabilities();
    const timer = window.setTimeout(() => emailRef.current?.focus(), 40);
    return () => window.clearTimeout(timer);
  }, []);

  const registrationMode = auth.capabilities?.registrationMode;
  const registrationAllowed = registrationMode !== "disabled";
  const passwordMinLength = auth.capabilities?.passwordMinLength ?? 15;

  useEffect(() => {
    if (mode === "register" && registrationMode === "disabled") setMode("login");
  }, [mode, registrationMode]);

  const completeAuth = () => {
    setPassword("");
    setConfirmPassword("");
    if (useCloudAuthStore.getState().binding?.bindingRequired) setMode("binding");
    else close();
  };

  const submitLogin = async (event: React.FormEvent) => {
    event.preventDefault();
    setMessage("");
    if (await auth.login(email.trim(), password)) completeAuth();
  };

  const submitRegister = async (event: React.FormEvent) => {
    event.preventDefault();
    setMessage("");
    if (password !== confirmPassword) {
      setMessage("两次输入的密码不一致");
      return;
    }
    if (password.length < passwordMinLength) {
      setMessage(`密码至少需要 ${passwordMinLength} 个字符`);
      return;
    }
    if (await auth.register({
      email: email.trim(),
      password,
      displayName: displayName.trim() || undefined,
      inviteToken: inviteToken.trim() || undefined,
    })) completeAuth();
  };

  const forgotPassword = async () => {
    if (!email.trim()) {
      setMessage("先填写需要找回密码的邮箱");
      emailRef.current?.focus();
      return;
    }
    if (await auth.forgotPassword(email.trim())) {
      setMessage("密码重置请求已提交，请按服务端提供的方式继续操作");
    }
  };

  return <div className="hx-account-overlay" role="presentation" onMouseDown={(event) => {
    if (event.target === event.currentTarget && !auth.loading) close();
  }}>
    <section className="hx-account-dialog" role="dialog" aria-modal="true" aria-labelledby="hx-account-dialog-title">
      <header className="hx-account-dialog-head">
        <div><span className="hx-account-mark">LT</span><div><h2 id="hx-account-dialog-title">{mode === "binding" ? "连接你的数据" : mode === "register" ? "创建 LifeTrace 账号" : "登录 LifeTrace"}</h2><p>{mode === "binding" ? "选择这台电脑上现有数据的归属" : "登录后可在设备之间同步个人记录"}</p></div></div>
        <button type="button" aria-label="关闭" disabled={auth.loading} onClick={close}><X /></button>
      </header>

      {mode === "binding" ? <div className="hx-account-binding">
        <div className="hx-account-binding-option">
          <strong>关联这台电脑上的现有数据</strong>
          <p>保留当前本地记录，并把这份资料与刚登录的账号关联。适合已经使用过 LifeTrace 的设备。</p>
          <button className="hx-btn primary" disabled={auth.loading} onClick={async () => { await auth.bindCurrentProfile(); if (!useCloudAuthStore.getState().binding?.bindingRequired) close(); }}>关联并开始同步</button>
        </div>
        <div className="hx-account-binding-option">
          <strong>使用新的空白云端资料</strong>
          <p>当前本地资料仍保留在这台电脑，新账号从空白资料开始。</p>
          <button className="hx-btn secondary" disabled={auth.loading} onClick={async () => { await auth.createCloudProfile(); if (!useCloudAuthStore.getState().binding?.bindingRequired) close(); }}>创建空白资料</button>
        </div>
        {auth.error && <p className="hx-account-error" role="alert">{auth.error}</p>}
      </div> : <>
        <div className="hx-account-tabs" role="tablist">
          <button type="button" className={mode === "login" ? "active" : ""} onClick={() => { setMode("login"); setMessage(""); auth.clearError(); }}>登录</button>
          {registrationAllowed && <button type="button" className={mode === "register" ? "active" : ""} onClick={() => { setMode("register"); setMessage(""); auth.clearError(); }}>注册</button>}
        </div>

        <form className="hx-account-form" onSubmit={mode === "register" ? submitRegister : submitLogin}>
          {mode === "register" && <label>昵称<input value={displayName} onChange={(event) => setDisplayName(event.target.value)} autoComplete="name" placeholder="你希望显示的名字" /></label>}
          <label>邮箱<input ref={emailRef} type="email" required value={email} onChange={(event) => setEmail(event.target.value)} autoComplete="username" placeholder="name@example.com" /></label>
          <label>密码<div className="hx-account-password"><input type={showPassword ? "text" : "password"} required minLength={mode === "register" ? passwordMinLength : undefined} value={password} onChange={(event) => setPassword(event.target.value)} autoComplete={mode === "register" ? "new-password" : "current-password"} placeholder={mode === "register" ? `至少 ${passwordMinLength} 个字符` : "输入密码"} /><button type="button" aria-label={showPassword ? "隐藏密码" : "显示密码"} onClick={() => setShowPassword((value) => !value)}>{showPassword ? <EyeOff /> : <Eye />}</button></div></label>
          {mode === "register" && <label>确认密码<input type={showPassword ? "text" : "password"} required value={confirmPassword} onChange={(event) => setConfirmPassword(event.target.value)} autoComplete="new-password" placeholder="再次输入密码" /></label>}
          {mode === "register" && registrationMode === "invite" && <label>邀请码<input required value={inviteToken} onChange={(event) => setInviteToken(event.target.value)} autoComplete="off" placeholder="输入管理员提供的邀请码" /></label>}

          {mode === "login" && <button className="hx-account-link" type="button" onClick={() => void forgotPassword()}>忘记密码？</button>}
          {(message || auth.error) && <p className="hx-account-error" role="alert">{message || auth.error}</p>}

          <button className="hx-btn primary hx-account-submit" disabled={auth.loading || !email.trim() || !password}>
            {auth.loading ? <><LoaderCircle className="spin" />正在处理…</> : mode === "register" ? "创建账号" : <><LogIn />登录</>}
          </button>
        </form>

        <div className="hx-account-server">
          <button type="button" onClick={() => setAdvanced((value) => !value)}><Server />服务器设置<ChevronRight className={advanced ? "open" : ""} /></button>
          {advanced && <label>LifeTrace 服务地址<input value={auth.origin} onChange={(event) => auth.setOrigin(event.target.value)} placeholder="https://api.example.com" /><small>普通使用无需修改。开发环境默认连接本机 8787 端口。</small></label>}
        </div>
      </>}
    </section>
  </div>;
}

export default function AccountEntry({ onOpenSettings }: { onOpenSettings: (section: SettingsSection) => void }) {
  const auth = useCloudAuthStore();
  const [menuOpen, setMenuOpen] = useState(false);
  const [dialog, setDialog] = useState<"login" | "register" | null>(null);
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const openAuth = () => setDialog("login");
    window.addEventListener("lifetrace:open-auth", openAuth);
    return () => window.removeEventListener("lifetrace:open-auth", openAuth);
  }, []);

  useEffect(() => {
    if (!menuOpen) return;
    const close = (event: MouseEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setMenuOpen(false);
    };
    window.addEventListener("mousedown", close);
    return () => window.removeEventListener("mousedown", close);
  }, [menuOpen]);

  const name = auth.user?.displayName || auth.user?.email;
  const status = auth.phase === "bootstrapping" || auth.phase === "refreshing"
    ? "正在恢复账户…"
    : auth.phase === "offline" && auth.user
      ? "离线 · 等待恢复"
      : auth.authenticated
        ? auth.binding?.bindingRequired ? "需要完成数据连接" : "已登录"
        : "同步你的数据";

  const logout = async () => {
    setMenuOpen(false);
    const accepted = await confirmAction({
      title: "退出 LifeTrace？",
      description: "退出后本机数据仍会保留，但会停止当前账号的云端同步。",
      confirmLabel: "退出登录",
    });
    if (accepted) await auth.logout(false);
  };

  const openSettings = (section: SettingsSection) => {
    setMenuOpen(false);
    onOpenSettings(section);
  };

  const hasIdentity = Boolean(auth.user && (auth.authenticated || auth.phase === "offline"));

  return <div className="hx-account-entry" ref={rootRef}>
    {hasIdentity ? <>
      {menuOpen && <div className="hx-account-menu" role="menu">
        <header><span>{initials(name)}</span><div><strong>{name}</strong><small>{auth.user?.email}</small></div></header>
        {auth.binding?.bindingRequired && <button type="button" className="attention" onClick={() => setDialog("login")}><Cloud />完成数据连接<ChevronRight /></button>}
        <button type="button" onClick={() => openSettings("security")}><ShieldCheck />账户与安全<ChevronRight /></button>
        <button type="button" onClick={() => openSettings("sync")}><Cloud />同步状态<ChevronRight /></button>
        <button type="button" onClick={() => openSettings("general")}><Settings />设置<ChevronRight /></button>
        <div className="hx-account-menu-separator" />
        <button type="button" className="danger" disabled={auth.loading} onClick={() => void logout()}><LogOut />退出登录</button>
      </div>}
      <button type="button" className="hx-account-button" aria-expanded={menuOpen} onClick={() => setMenuOpen((value) => !value)}>
        <span className="hx-account-avatar">{initials(name)}</span><div><strong>{name}</strong><small>{status}</small></div><ChevronRight className={menuOpen ? "open" : ""} />
      </button>
    </> : <button type="button" className="hx-account-button" onClick={() => setDialog("login")}>
      <span className="hx-account-avatar anonymous"><UserCircle /></span><div><strong>{auth.phase === "bootstrapping" || auth.phase === "refreshing" ? "正在恢复账户…" : "登录 / 注册"}</strong><small>{status}</small></div><ChevronRight />
    </button>}
    {dialog && <AccountDialog initialMode={dialog} close={() => setDialog(null)} />}
  </div>;
}
