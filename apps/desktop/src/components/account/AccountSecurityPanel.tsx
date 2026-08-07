import { useState } from "react";
import { KeyRound, LogOut, ShieldCheck } from "lucide-react";
import { useCloudAuthStore } from "@/src/stores/useCloudAuthStore";
import { confirmAction } from "@/src/ui/feedback/confirm";

export default function AccountSecurityPanel() {
  const auth = useCloudAuthStore();
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [message, setMessage] = useState("");

  const changePassword = async (event: React.FormEvent) => {
    event.preventDefault();
    setMessage("");
    const minLength = auth.capabilities?.passwordMinLength ?? 15;
    if (newPassword !== confirmPassword) {
      setMessage("两次输入的新密码不一致");
      return;
    }
    if (newPassword.length < minLength) {
      setMessage(`新密码至少需要 ${minLength} 个字符`);
      return;
    }
    if (await auth.changePassword(currentPassword, newPassword)) {
      setCurrentPassword("");
      setNewPassword("");
      setConfirmPassword("");
      setMessage("密码已更新");
    }
  };

  const logoutAll = async () => {
    const accepted = await confirmAction({
      title: "退出所有设备？",
      description: "这会撤销当前账号的全部登录会话。所有设备下次使用云同步时都需要重新登录。",
      confirmLabel: "退出所有设备",
    });
    if (!accepted) return;
    await auth.logout(true);
    window.location.reload();
  };

  return <section id="settings-security" className="hx-settings-page-section">
    <header><h2>账户与安全</h2><p>{auth.authenticated ? `当前登录：${auth.user?.email}` : "登录后可以管理账号安全。"}</p></header>
    {!auth.authenticated ? <div className="hx-setting-rows"><div className="hx-setting-row"><div><ShieldCheck /><span><strong>LifeTrace 账号</strong><small>当前未登录</small></span></div><button type="button" className="hx-btn primary" onClick={() => window.dispatchEvent(new Event("lifetrace:open-auth"))}>登录 / 注册</button></div></div> : <>
      <form className="hx-settings-standard-form" onSubmit={changePassword}>
        <div className="hx-setting-rows">
          <label className="hx-setting-row"><div><KeyRound /><span><strong>当前密码</strong><small>修改密码前需要再次验证。</small></span></div><input type="password" value={currentPassword} onChange={(event) => setCurrentPassword(event.target.value)} autoComplete="current-password" /></label>
          <label className="hx-setting-row"><div><KeyRound /><span><strong>新密码</strong><small>使用足够长且不重复的密码。</small></span></div><input type="password" value={newPassword} onChange={(event) => setNewPassword(event.target.value)} autoComplete="new-password" /></label>
          <label className="hx-setting-row"><div><KeyRound /><span><strong>确认新密码</strong></span></div><input type="password" value={confirmPassword} onChange={(event) => setConfirmPassword(event.target.value)} autoComplete="new-password" /></label>
        </div>
        {message && <p className="hx-inline-message" role="status">{message}</p>}
        <footer className="hx-settings-page-actions"><button className="hx-btn primary" disabled={auth.loading || !currentPassword || !newPassword || !confirmPassword}>修改密码</button></footer>
      </form>
      <div className="hx-settings-danger-zone"><div><strong>退出所有设备</strong><p>撤销当前账号的所有 Refresh Token 和登录会话。</p></div><button type="button" className="hx-btn secondary danger" disabled={auth.loading} onClick={() => void logoutAll()}><LogOut />退出所有设备</button></div>
    </>}
  </section>;
}
