import { useState } from "react";
import { Cloud, LogIn, LogOut, ShieldCheck } from "lucide-react";
import { useCloudAuthStore } from "@/src/stores/useCloudAuthStore";

export default function CloudAccountPanel() {
  const auth = useCloudAuthStore();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");

  return <article className="hx-panel hx-cloud-account">
    <header className="hx-panel-head"><div><span className="hx-kicker">可选云账号</span><h2>LifeTrace Cloud</h2></div><Cloud /></header>
    <div className="hx-panel-body">
      <p className="hx-cloud-local-note"><ShieldCheck /> 无论是否登录，SQLite 本地数据和全部本地功能都可继续使用。</p>
      <label>云服务地址<input value={auth.origin} placeholder="https://cloud.example.com" onChange={event=>auth.setOrigin(event.target.value)} /></label>
      {!auth.authenticated ? <>
        <label>邮箱<input type="email" autoComplete="username" value={email} onChange={event=>setEmail(event.target.value)} /></label>
        <label>密码<input type="password" autoComplete="current-password" value={password} onChange={event=>setPassword(event.target.value)} /></label>
        <button className="hx-btn primary" disabled={auth.loading||!auth.origin||!email||!password} onClick={async()=>{await auth.login(email,password);setPassword("")}}><LogIn /> {auth.loading?"正在登录…":"登录云账号"}</button>
      </> : <>
        <div className="hx-cloud-user"><strong>{auth.user?.displayName||auth.user?.email}</strong><small>{auth.user?.email}</small><span>当前会话：{auth.session?.status} · 权限 {auth.scopes.length} 项</span></div>
        <div className="hx-settings-actions"><button className="hx-btn secondary" disabled={auth.loading} onClick={()=>auth.logout(false)}><LogOut />退出当前设备</button><button className="hx-btn secondary" disabled={auth.loading} onClick={()=>auth.logout(true)}>退出全部设备</button></div>
      </>}
      {auth.error&&<p className="hx-inline-message" role="alert">{auth.error}</p>}
      <small>Access Token 仅保存在内存；Refresh Token 仅保存在 Windows Credential Manager，不写入 SQLite、JSON 或 localStorage。</small>
    </div>
  </article>;
}
