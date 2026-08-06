import { FormEvent, useEffect, useMemo, useState } from "react";
import { AuthApi, type WebSession } from "./core";
import { RegistrationApi, type AuthCapabilities } from "./registration";
import { Notice } from "./ui";

type AuthMode = "login" | "register";

interface AuthScreenProps {
  auth: AuthApi;
  error: string;
  onAuthenticated: (session: WebSession) => void;
}

export function AuthScreen({ auth, error, onAuthenticated }: AuthScreenProps) {
  const registration = useMemo(() => new RegistrationApi(), []);
  const [mode, setMode] = useState<AuthMode>("login");
  const [capabilities, setCapabilities] = useState<AuthCapabilities | null>(null);
  const [email, setEmail] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [inviteToken, setInviteToken] = useState("");
  const [publicDevice, setPublicDevice] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [localError, setLocalError] = useState("");

  useEffect(() => {
    let active = true;
    registration.capabilities()
      .then((value) => {
        if (!active) return;
        setCapabilities(value);
        if (value.registrationMode === "disabled") setMode("login");
      })
      .catch((cause: unknown) => {
        if (active) setLocalError(cause instanceof Error ? cause.message : "无法读取注册配置");
      });
    return () => { active = false; };
  }, [registration]);

  const registrationEnabled = capabilities?.registrationMode !== "disabled";
  const inviteRequired = capabilities?.registrationMode === "invite";
  const minimumLength = capabilities?.passwordMinLength ?? 12;
  const maximumBytes = capabilities?.passwordMaxBytes ?? 1024;

  function switchMode(next: AuthMode) {
    if (next === "register" && !registrationEnabled) return;
    setMode(next);
    setPassword("");
    setConfirmPassword("");
    setLocalError("");
  }

  function validateRegistration(): string | null {
    if (!displayName.trim()) return "请输入昵称";
    if (password.length < minimumLength) return `密码至少需要 ${minimumLength} 个字符`;
    if (new TextEncoder().encode(password).length > maximumBytes) return `密码不能超过 ${maximumBytes} 字节`;
    if (password !== confirmPassword) return "两次输入的密码不一致";
    if (inviteRequired && !inviteToken.trim()) return "当前为邀请注册，请输入邀请码";
    return null;
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    setLocalError("");
    if (!navigator.onLine) {
      setLocalError("网页端需要联网才能完成账号认证");
      return;
    }
    if (mode === "register") {
      const validationError = validateRegistration();
      if (validationError) {
        setLocalError(validationError);
        return;
      }
    }
    setSubmitting(true);
    try {
      const session = mode === "login"
        ? await auth.login(email.trim(), password, publicDevice)
        : await registration.register({ email, password, displayName, inviteToken, publicDevice });
      onAuthenticated(session);
    } catch (cause) {
      setLocalError(cause instanceof Error ? cause.message : mode === "login" ? "登录失败" : "注册失败");
    } finally {
      setSubmitting(false);
    }
  }

  return <div className="login-page">
    <section className="login-intro">
      <div className="brand large"><span className="brand-mark">L</span><span><strong>LifeTrace</strong><small>Cloud Web</small></span></div>
      <div><p className="eyebrow">LIFETRACE CLOUD</p><h1>{mode === "login" ? "继续管理你的个人生活数据。" : "创建你的 LifeTrace 云端账户。"}</h1><p>网页端只在联网时工作。业务数据不写入 IndexedDB 或 localStorage，所有保存操作直接提交到 LifeTrace Cloud。</p></div>
      <div className="feature-row"><span>云端直写</span><span>跨端一致</span><span>公共设备保护</span></div>
    </section>
    <section className="login-card-wrap">
      <form className="login-card" onSubmit={(event) => void submit(event)}>
        <p className="eyebrow">{mode === "login" ? "欢迎回来" : "开始使用"}</p>
        <h2>{mode === "login" ? "登录 LifeTrace" : "注册 LifeTrace"}</h2>
        {mode === "register" && <label>昵称<input type="text" required autoComplete="name" maxLength={80} value={displayName} onChange={(event) => setDisplayName(event.target.value)} placeholder="你的显示名称" /></label>}
        <label>邮箱<input type="email" required autoComplete="email" value={email} onChange={(event) => setEmail(event.target.value)} placeholder="name@example.com" /></label>
        <label>密码<input type="password" required minLength={mode === "register" ? minimumLength : undefined} autoComplete={mode === "login" ? "current-password" : "new-password"} value={password} onChange={(event) => setPassword(event.target.value)} placeholder={mode === "register" ? `至少 ${minimumLength} 个字符` : "输入密码"} /></label>
        {mode === "register" && <label>确认密码<input type="password" required minLength={minimumLength} autoComplete="new-password" value={confirmPassword} onChange={(event) => setConfirmPassword(event.target.value)} placeholder="再次输入密码" /></label>}
        {mode === "register" && inviteRequired && <label>邀请码<input type="text" required autoComplete="off" value={inviteToken} onChange={(event) => setInviteToken(event.target.value)} placeholder="输入管理员提供的邀请码" /></label>}
        <label className="checkbox"><input type="checkbox" checked={publicDevice} onChange={(event) => setPublicDevice(event.target.checked)} /><span>这是公共设备，将使用更短的会话有效期</span></label>
        {(localError || error) && <Notice kind="error">{localError || error}</Notice>}
        <button className="primary-button full" disabled={submitting || (mode === "register" && !capabilities)}>{submitting ? (mode === "login" ? "登录中…" : "注册中…") : (mode === "login" ? "登录" : "创建账户")}</button>
        <div className="auth-switch">
          {mode === "login" && registrationEnabled && <><span>还没有账户？</span><button type="button" className="link-button" onClick={() => switchMode("register")}>立即注册</button></>}
          {mode === "register" && <><span>已经有账户？</span><button type="button" className="link-button" onClick={() => switchMode("login")}>返回登录</button></>}
          {mode === "login" && capabilities?.registrationMode === "disabled" && <span>当前服务未开放用户注册</span>}
        </div>
        <small>认证使用 HttpOnly Cookie；退出后页面内存中的业务数据立即清空。</small>
      </form>
    </section>
  </div>;
}
