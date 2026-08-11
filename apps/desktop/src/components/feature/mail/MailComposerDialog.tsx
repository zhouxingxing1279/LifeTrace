import { useEffect, useMemo, useState } from "react";
import { ChevronDown, ChevronUp, LoaderCircle, Send, X } from "lucide-react";
import { mailApi, type MailAccount, type MailMessage } from "@/src/services/mailApi";
import { confirmAction } from "@/src/ui/feedback/confirm";
import { senderIdentity, senderLabel } from "./mailModel";
import { actionButton, errorMessage, inputStyle, panelStyle, toast } from "./mailStyles";

type Props = {
  accounts: MailAccount[];
  initialAccountId?: string | null;
  replyMessage?: MailMessage | null;
  onClose: () => void;
  onSent?: () => void;
};

function splitRecipients(value: string) {
  return value.split(/[;,\n]/).map((item) => item.trim()).filter(Boolean);
}

export function MailComposerDialog({ accounts, initialAccountId, replyMessage, onClose, onSent }: Props) {
  const replyAccountId = replyMessage?.accountId || null;
  const fallbackAccountId = initialAccountId && accounts.some((item) => item.id === initialAccountId)
    ? initialAccountId
    : accounts[0]?.id || "";
  const [accountId, setAccountId] = useState(replyAccountId || fallbackAccountId);
  const [to, setTo] = useState(() => replyMessage ? senderIdentity(replyMessage).email : "");
  const [cc, setCc] = useState("");
  const [bcc, setBcc] = useState("");
  const [subject, setSubject] = useState(() => replyMessage
    ? (/^re:/i.test(replyMessage.subject) ? replyMessage.subject : `Re: ${replyMessage.subject}`)
    : "");
  const [body, setBody] = useState("");
  const [showCopies, setShowCopies] = useState(false);
  const [sending, setSending] = useState(false);

  const account = useMemo(() => accounts.find((item) => item.id === accountId) || null, [accounts, accountId]);

  useEffect(() => {
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !sending) onClose();
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [onClose, sending]);

  const send = async () => {
    const recipients = splitRecipients(to);
    if (!account || !recipients.length || !body.trim()) {
      toast("请选择发件邮箱，并填写收件人和正文", "error");
      return;
    }
    const confirmed = await confirmAction({
      title: replyMessage ? "确认回复邮件" : "确认发送邮件",
      description: `将通过 ${account.emailAddress} 发送给 ${recipients.join(", ")}。发送后会产生外部副作用。`,
      confirmLabel: "确认发送",
    });
    if (!confirmed) return;

    setSending(true);
    try {
      await mailApi.accounts.send(account.id, {
        to: recipients,
        cc: splitRecipients(cc),
        bcc: splitRecipients(bcc),
        subject: subject.trim(),
        bodyText: body,
        inReplyToMessageId: replyMessage?.id || null,
        idempotencyKey: crypto.randomUUID(),
      });
      toast(replyMessage ? "回复已发送" : "邮件已发送");
      onSent?.();
      onClose();
    } catch (error) {
      toast(errorMessage(error), "error");
    } finally {
      setSending(false);
    }
  };

  return (
    <div
      style={{ position: "fixed", inset: 0, zIndex: 70, background: "var(--apple-scrim, rgba(0,0,0,.48))", display: "grid", placeItems: "center", padding: 24 }}
      onMouseDown={(event) => event.target === event.currentTarget && !sending && onClose()}
    >
      <section
        style={{ ...panelStyle, width: "min(820px, 95vw)", height: "min(820px, calc(100dvh - 48px))", maxHeight: "calc(100dvh - 48px)", display: "flex", flexDirection: "column", padding: 22, background: "var(--ui-bg-surface)", color: "var(--ui-foreground)", boxShadow: "0 24px 70px rgba(0,0,0,.32)" }}
        role="dialog"
        aria-modal="true"
        aria-label={replyMessage ? "回复邮件" : "写邮件"}
      >
        <header style={{ display: "flex", alignItems: "flex-start", justifyContent: "space-between", gap: 16, marginBottom: 18 }}>
          <div style={{ minWidth: 0 }}>
            <h2 style={{ margin: 0, fontSize: 20 }}>{replyMessage ? "回复邮件" : "写邮件"}</h2>
            <p style={{ margin: "6px 0 0", opacity: .62, fontSize: 13, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {replyMessage ? `回复 ${senderLabel(replyMessage)}` : "从任一已连接邮箱发送新邮件"}
            </p>
          </div>
          <button type="button" style={actionButton} disabled={sending} onClick={onClose}><X size={16} />关闭</button>
        </header>

        <div style={{ display: "grid", gap: 11, minHeight: 0, overflowY: "auto", overscrollBehavior: "contain", paddingRight: 6, scrollbarGutter: "stable" }}>
          <label>发件邮箱
            <select
              style={{ ...inputStyle, marginTop: 6 }}
              value={accountId}
              disabled={Boolean(replyAccountId) || sending}
              onChange={(event) => setAccountId(event.target.value)}
            >
              {accounts.map((item) => <option key={item.id} value={item.id}>{item.displayName || item.emailAddress} · {item.emailAddress}</option>)}
            </select>
          </label>
          <label>收件人<input autoFocus={!replyMessage} style={{ ...inputStyle, marginTop: 6 }} value={to} onChange={(event) => setTo(event.target.value)} placeholder="多个地址可用逗号或分号分隔" /></label>

          <button type="button" style={{ ...actionButton, width: "fit-content", border: 0, paddingLeft: 0 }} onClick={() => setShowCopies((value) => !value)}>
            {showCopies ? <ChevronUp size={15} /> : <ChevronDown size={15} />}抄送 / 密送
          </button>

          {showCopies ? <div style={{ display: "grid", gridTemplateColumns: "repeat(2,minmax(0,1fr))", gap: 10 }}>
            <label>抄送<input style={{ ...inputStyle, marginTop: 6 }} value={cc} onChange={(event) => setCc(event.target.value)} /></label>
            <label>密送<input style={{ ...inputStyle, marginTop: 6 }} value={bcc} onChange={(event) => setBcc(event.target.value)} /></label>
          </div> : null}

          <label>主题<input style={{ ...inputStyle, marginTop: 6 }} value={subject} onChange={(event) => setSubject(event.target.value)} placeholder="邮件主题" /></label>
          <label>正文<textarea autoFocus={Boolean(replyMessage)} style={{ ...inputStyle, marginTop: 6, minHeight: 300, resize: "vertical", lineHeight: 1.6 }} value={body} onChange={(event) => setBody(event.target.value)} placeholder="输入邮件正文…" /></label>
        </div>

        <footer style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: 10, marginTop: 18, flex: "0 0 auto" }}>
          <span style={{ fontSize: 12.5, opacity: .55 }}>当前发送接口暂不支持添加新附件。</span>
          <div style={{ display: "flex", gap: 10 }}>
            <button type="button" style={actionButton} disabled={sending} onClick={onClose}>取消</button>
            <button type="button" className="hx-btn primary" disabled={sending || !account} onClick={() => void send()}>{sending ? <LoaderCircle size={16} /> : <Send size={16} />}发送</button>
          </div>
        </footer>
      </section>
    </div>
  );
}
