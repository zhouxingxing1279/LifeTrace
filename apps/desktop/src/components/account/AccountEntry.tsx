import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
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
type AuthDialogMode = "login" | "register";

function initials(value?: string | null) {
  const text = value?.trim();
  if (!text) return "LT";
  return text.slice(0, 2).toUpperCase();
}

function openSettings(section: SettingsSection) {
  window.location.hash = `settings-${section}`;
  const settingsButton = Array.from(document.querySelectorAll<HTMLButtonElement>(".hx-sidebar-foot > button"))
    .find((button) => button.textContent?.includes("设置"));
  settingsButton?.click();
  window.setTimeout(() => document.getElementById(`settings-${section}`)?.scrollIntoView({ block: "start" }), 60);
}

function AccountDialog({ initialMode, close }: { initialMode: AuthDialogMode; close: () => void }) {
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
    close();
    // 登录可能切换到另一个用户专属 SQLite Profile。刷新 WebView，确保所有模块
    // （包括有内部缓存的笔记、英语等）重新从当前 Profile 读取，而不是残留上一个用户的数据。
    window.setTimeout(() => window.location.reload(), 50);
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
      setMessage("密码重置请求已提交");
    }
  };

  return <div className="hx-account-overlay" role="presentation" onMouseDown={(event) => {
    if (event.target === event.currentTarget && !auth.loading) close();
  }}>
    <section className="hx-account-dialog" role="dialog" aria-modal="true" aria-labelledby="hx-account-dialog-title">
      <header className="hx-account-dialog-head">
        <div><span className="hx-account-mark">LT</span><div><h2 id="hx-account-dialog-title">{mode === "register" ? "创建 LifeTrace 账号" : "登录 LifeTrace"}</h2><p>每个账号拥有独立的数据空间</p></div></div>
        <button type="button" aria-label="关闭" disabled={auth.loading} onClick={close}><X /></button>
      </header>

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
    </section>
  </div>;
}

function AccountEntry() {
  const auth = useCloudAuthStore();
  const [menuOpen, setMenuOpen] = useState(false);
  const [dialog, setDialog] = useState<AuthDialogMode | null>(null);
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
      ? "离线 · 保持登录"
      : auth.authenticated
        ? "已登录"
        : "同步你的数据";

  const logout = async () => {
    setMenuOpen(false);
    const accepted = await confirmAction({
      title: "退出 LifeTrace？",
      description: "退出后当前账号的数据仍会安全保存在本机和云端，但不会向其他账号显示。",
      confirmLabel: "退出登录",
    });
    if (!accepted) return;
    await auth.logout(false);
    window.location.reload();
  };

  const hasIdentity = Boolean(auth.user && (auth.authenticated || auth.phase === "offline"));

  return <div className="hx-account-entry" ref={rootRef}>
    {hasIdentity ? <>
      {menuOpen && <div className="hx-account-menu" role="menu">
        <header><span>{initials(name)}</span><div><strong>{name}</strong><small>{auth.user?.email}</small></div></header>
        <button type="button" onClick={() => { setMenuOpen(false); openSettings("security"); }}><ShieldCheck />账户与安全<ChevronRight /></button>
        <button type="button" onClick={() => { setMenuOpen(false); openSettings("sync"); }}><Cloud />同步状态<ChevronRight /></button>
        <button type="button" onClick={() => { setMenuOpen(false); openSettings("general"); }}><Settings />设置<ChevronRight /></button>
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

export function AccountEntryHost() {
  const [target, setTarget] = useState<Element | null>(null);

  useEffect(() => {
    void useCloudAuthStore.getState().initialize();
    const locate = () => {
      const next = document.querySelector(".hx-sidebar-foot");
      if (next) setTarget(next);
      return Boolean(next);
    };
    if (locate()) return;
    const observer = new MutationObserver(() => {
      if (locate()) observer.disconnect();
    });
    observer.observe(document.body, { childList: true, subtree: true });
    return () => observer.disconnect();
  }, []);

  return target ? createPortal(<AccountEntry />, target) : null;
}

export default AccountEntry;
