import { useState } from "react";
import { LoaderCircle, Plus, X } from "lucide-react";
import {
  mailApi,
  type MailAccount,
  type MailAccountInput,
  type MailProvider,
} from "@/src/services/mailApi";
import { actionButton, errorMessage, inputStyle, panelStyle, toast } from "./mailStyles";

export function MailAccountDialog({
  onClose,
  onCreated,
}: {
  onClose: () => void;
  onCreated: (account: MailAccount) => void;
}) {
  const [provider, setProvider] = useState<MailProvider>("qq");
  const [emailAddress, setEmailAddress] = useState("");
  const [authorizationCode, setAuthorizationCode] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [imapHost, setImapHost] = useState("");
  const [imapPort, setImapPort] = useState("993");
  const [smtpHost, setSmtpHost] = useState("");
  const [smtpPort, setSmtpPort] = useState("465");
  const [saving, setSaving] = useState(false);

  const submit = async () => {
    if (!emailAddress.trim() || !authorizationCode) {
      toast("请输入邮箱地址和授权码", "error");
      return;
    }
    const input: MailAccountInput = {
      provider,
      emailAddress: emailAddress.trim(),
      displayName: displayName.trim() || undefined,
      authorizationCode,
      ...(provider === "generic" ? {
        imapHost: imapHost.trim(),
        imapPort: Number(imapPort),
        imapSecurity: "tls" as const,
        smtpHost: smtpHost.trim(),
        smtpPort: Number(smtpPort),
        smtpSecurity: "tls" as const,
      } : {}),
    };
    if (provider === "generic" && (!input.imapHost || !input.smtpHost || !Number.isFinite(input.imapPort) || !Number.isFinite(input.smtpPort))) {
      toast("请填写有效的 IMAP/SMTP 地址和端口", "error");
      return;
    }
    setSaving(true);
    try {
      const account = await mailApi.accounts.create(input);
      setAuthorizationCode("");
      onCreated(account);
      toast(account.status === "active" ? "邮箱连接成功" : "邮箱已保存，请检查连接状态");
      onClose();
    } catch (error) {
      setAuthorizationCode("");
      toast(errorMessage(error), "error");
    } finally {
      setSaving(false);
    }
  };

  return (
    <div
      style={{ position: "fixed", inset: 0, zIndex: 60, background: "rgba(0,0,0,.38)", display: "grid", placeItems: "center", padding: 24 }}
      onMouseDown={(event) => event.target === event.currentTarget && !saving && onClose()}
    >
      <section style={{ ...panelStyle, width: "min(620px, 94vw)", maxHeight: "90vh", overflow: "auto", padding: 22, background: "var(--background, #fff)" }} role="dialog" aria-modal="true" aria-label="添加邮箱">
        <header style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 16, marginBottom: 18 }}>
          <div>
            <h2 style={{ margin: 0, fontSize: 20 }}>连接邮箱</h2>
            <p style={{ margin: "6px 0 0", opacity: .64, fontSize: 13 }}>授权码加密保存；连接成功后会同步最近 30 天邮件。</p>
          </div>
          <button type="button" style={actionButton} disabled={saving} onClick={onClose}><X size={16} />关闭</button>
        </header>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(2, minmax(0,1fr))", gap: 12 }}>
          <label>邮箱类型<select style={{ ...inputStyle, marginTop: 6 }} value={provider} onChange={(event) => setProvider(event.target.value as MailProvider)}>
            <option value="qq">QQ 邮箱</option>
            <option value="163">163 邮箱</option>
            <option value="126">126 邮箱</option>
            <option value="yeah">yeah.net</option>
            <option value="generic">通用 IMAP/SMTP</option>
          </select></label>
          <label>显示名称<input style={{ ...inputStyle, marginTop: 6 }} value={displayName} onChange={(event) => setDisplayName(event.target.value)} placeholder="可选" /></label>
          <label>邮箱地址<input style={{ ...inputStyle, marginTop: 6 }} type="email" value={emailAddress} onChange={(event) => setEmailAddress(event.target.value)} placeholder="name@example.com" /></label>
          <label>授权码<input style={{ ...inputStyle, marginTop: 6 }} type="password" autoComplete="new-password" value={authorizationCode} onChange={(event) => setAuthorizationCode(event.target.value)} placeholder="邮箱授权码" /></label>
          {provider === "generic" ? <>
            <label>IMAP 主机<input style={{ ...inputStyle, marginTop: 6 }} value={imapHost} onChange={(event) => setImapHost(event.target.value)} placeholder="imap.example.com" /></label>
            <label>IMAP 端口<input style={{ ...inputStyle, marginTop: 6 }} value={imapPort} onChange={(event) => setImapPort(event.target.value)} inputMode="numeric" /></label>
            <label>SMTP 主机<input style={{ ...inputStyle, marginTop: 6 }} value={smtpHost} onChange={(event) => setSmtpHost(event.target.value)} placeholder="smtp.example.com" /></label>
            <label>SMTP 端口<input style={{ ...inputStyle, marginTop: 6 }} value={smtpPort} onChange={(event) => setSmtpPort(event.target.value)} inputMode="numeric" /></label>
          </> : null}
        </div>

        <footer style={{ display: "flex", justifyContent: "flex-end", gap: 10, marginTop: 20 }}>
          <button type="button" style={actionButton} disabled={saving} onClick={onClose}>取消</button>
          <button type="button" className="hx-btn primary" disabled={saving} onClick={() => void submit()}>{saving ? <LoaderCircle size={16} /> : <Plus size={16} />}添加并测试</button>
        </footer>
      </section>
    </div>
  );
}
