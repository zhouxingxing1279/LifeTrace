import { useState, type FormEvent } from "react";
import { Button, Card, Input } from "../../design-system/ui";
import type { CloudSession } from "../../api/cloud";
import type { PlatformAdapter } from "../../platform";

export function LoginPage({ platform, onAuthenticated }: { platform: PlatformAdapter; onAuthenticated: (session: CloudSession) => Promise<void> }) {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    if (!platform.login || !email.trim() || !password) return;
    setLoading(true); setError("");
    try {
      const session = await platform.login(email.trim(), password);
      if (!session.authenticated) throw new Error(session.error || "登录失败");
      await onAuthenticated(session);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "登录失败");
    } finally {
      setLoading(false);
    }
  };

  return <main className="lt-auth-page"><Card className="lt-auth-card"><div className="lt-caption">LIFETRACE V2</div><h1>Sign in</h1><p className="lt-muted">使用 LifeTrace Cloud 账户进入统一个人工作区。</p><form className="lt-form-grid" onSubmit={submit}><label>Email<Input name="email" type="email" autoComplete="email" value={email} onChange={(event) => setEmail(event.target.value)} required /></label><label>Password<Input name="password" type="password" autoComplete="current-password" value={password} onChange={(event) => setPassword(event.target.value)} required /></label>{error ? <p role="alert" className="lt-error-text">{error}</p> : null}<Button type="submit" disabled={loading}>{loading ? "Signing in…" : "Continue"}</Button></form></Card></main>;
}
