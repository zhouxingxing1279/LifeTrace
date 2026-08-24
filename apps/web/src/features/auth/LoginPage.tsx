import { useEffect, useState, type FormEvent } from "react";
import { Navigate, useNavigate } from "react-router-dom";
import { Leaf, LockKeyhole, WifiOff } from "lucide-react";
import { useApp } from "../../app/AppContext";
import { Button, Card, CardContent, Input } from "../../components/ui";

export function LoginPage() {
  const { session, login, loading, online, error, clearError } = useApp();
  const navigate = useNavigate();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [publicDevice, setPublicDevice] = useState(false);

  useEffect(() => () => clearError(), [clearError]);
  if (session) return <Navigate to="/app/today" replace />;

  async function submit(event: FormEvent) {
    event.preventDefault();
    await login(email.trim(), password, publicDevice);
    navigate("/app/today", { replace: true });
  }

  return <main className="flex min-h-screen items-center justify-center bg-background px-4 py-10">
    <div className="w-full max-w-[420px]">
      <div className="mb-8 flex items-center gap-3">
        <div className="flex h-10 w-10 items-center justify-center rounded-lg border bg-card text-primary"><Leaf size={20} /></div>
        <div>
          <div className="text-lg font-semibold tracking-[-0.02em]">LifeTrace</div>
          <div className="text-xs text-muted-foreground">Personal OS</div>
        </div>
      </div>
      <Card>
        <CardContent className="p-6 sm:p-7">
          <div className="mb-6">
            <div className="eyebrow">Cloud account</div>
            <h1 className="mt-2 text-2xl font-semibold tracking-[-0.025em]">登录你的生活工作台</h1>
            <p className="mt-2 text-sm leading-6 text-muted-foreground">使用现有 LifeTrace Cloud 账号。Web 端只读取云端业务数据，不在浏览器持久化业务记录。</p>
          </div>
          {!online ? <div className="mb-4 flex gap-2 rounded-md border border-warning/30 bg-warning/10 p-3 text-sm"><WifiOff size={17} className="mt-0.5 shrink-0" /><span>当前离线，恢复网络后才能登录。</span></div> : null}
          {error ? <div role="alert" className="mb-4 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">{error}</div> : null}
          <form className="space-y-4" onSubmit={(event) => void submit(event)}>
            <label className="block space-y-1.5 text-sm font-medium">
              <span>邮箱</span>
              <Input type="email" autoComplete="username" value={email} onChange={(event) => setEmail(event.target.value)} required placeholder="name@example.com" />
            </label>
            <label className="block space-y-1.5 text-sm font-medium">
              <span>密码</span>
              <Input type="password" autoComplete="current-password" value={password} onChange={(event) => setPassword(event.target.value)} required placeholder="输入密码" />
            </label>
            <label className="flex cursor-pointer items-start gap-3 rounded-md border bg-muted/25 p-3 text-sm">
              <input className="mt-1" type="checkbox" checked={publicDevice} onChange={(event) => setPublicDevice(event.target.checked)} />
              <span><span className="font-medium">这是公共设备</span><span className="mt-0.5 block text-xs leading-5 text-muted-foreground">使用更严格的会话策略，不建议在共享电脑保存登录状态。</span></span>
            </label>
            <Button className="w-full" type="submit" disabled={loading || !online}><LockKeyhole size={16} />{loading ? "正在登录…" : "登录 LifeTrace"}</Button>
          </form>
        </CardContent>
      </Card>
      <p className="mt-5 text-center text-xs leading-5 text-muted-foreground">界面参考：Preline Login · shadcn/ui Form · Catalyst Typography</p>
    </div>
  </main>;
}
