import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Archive,
  ArrowLeft,
  CalendarPlus,
  CheckCircle2,
  ChevronRight,
  Clock3,
  Download,
  Inbox,
  ListTodo,
  LoaderCircle,
  Mail,
  Paperclip,
  Plus,
  RefreshCw,
  Reply,
  Search,
  Send,
  StickyNote,
  X,
} from "lucide-react";
import { browserTimezone, executionApi } from "@/src/services/executionApi";
import {
  mailApi,
  type MailAccount,
  type MailAccountInput,
  type MailAttachment,
  type MailMessage,
  type MailMessageSummary,
  type MailProvider,
} from "@/src/services/mailApi";
import { confirmAction } from "@/src/ui/feedback/confirm";

const card: React.CSSProperties = {
  border: "1px solid var(--line, rgba(128,128,128,.22))",
  borderRadius: 14,
  background: "var(--panel, rgba(255,255,255,.7))",
  overflow: "hidden",
};
const inputStyle: React.CSSProperties = {
  width: "100%",
  minHeight: 38,
  border: "1px solid var(--line, rgba(128,128,128,.28))",
  borderRadius: 9,
  background: "var(--surface, transparent)",
  color: "inherit",
  padding: "8px 10px",
  font: "inherit",
  boxSizing: "border-box",
};
const actionButton: React.CSSProperties = {
  minHeight: 34,
  display: "inline-flex",
  alignItems: "center",
  justifyContent: "center",
  gap: 6,
  border: "1px solid var(--line, rgba(128,128,128,.25))",
  borderRadius: 9,
  background: "transparent",
  color: "inherit",
  padding: "6px 10px",
  cursor: "pointer",
};

function toast(message: string, type: "success" | "error" = "success") {
  window.dispatchEvent(
    new CustomEvent("hengxu-toast", {
      detail: { message, type, duration: type === "error" ? 4500 : 2500 },
    }),
  );
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : "邮件操作失败";
}

function formatTime(value?: string | null) {
  if (!value) return "";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  const today = new Date();
  const sameDay = date.getFullYear() === today.getFullYear()
    && date.getMonth() === today.getMonth()
    && date.getDate() === today.getDate();
  return new Intl.DateTimeFormat("zh-CN", sameDay
    ? { hour: "2-digit", minute: "2-digit" }
    : { month: "numeric", day: "numeric", hour: "2-digit", minute: "2-digit" }).format(date);
}

function formatBytes(value: number) {
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KB`;
  return `${(value / (1024 * 1024)).toFixed(1)} MB`;
}

type AddressCarrier = { fromJson: unknown };
type MailboxIdentity = { email: string; name?: string };

function firstMailbox(value: unknown): MailboxIdentity | null {
  if (Array.isArray(value)) {
    for (const item of value) {
      const found = firstMailbox(item);
      if (found) return found;
    }
    return null;
  }
  if (!value || typeof value !== "object") return null;
  const current = value as Record<string, unknown>;
  const address = typeof current.address === "string"
    ? current.address
    : typeof current.email === "string"
      ? current.email
      : null;
  if (address?.includes("@")) {
    return {
      email: address.trim(),
      name: typeof current.name === "string" && current.name.trim() ? current.name.trim() : undefined,
    };
  }
  for (const nested of Object.values(current)) {
    const found = firstMailbox(nested);
    if (found) return found;
  }
  return null;
}

function collectAddresses(value: unknown): string[] {
  const result: string[] = [];
  const visit = (current: unknown) => {
    if (typeof current === "string") {
      if (current.includes("@")) result.push(current.trim());
      return;
    }
    if (Array.isArray(current)) {
      current.forEach(visit);
      return;
    }
    if (current && typeof current === "object") {
      for (const [key, nested] of Object.entries(current as Record<string, unknown>)) {
        if ((key === "address" || key === "email") && typeof nested === "string" && nested.includes("@")) {
          result.push(nested.trim());
        } else {
          visit(nested);
        }
      }
    }
  };
  visit(value);
  return [...new Set(result)];
}

function senderIdentity(message: AddressCarrier): MailboxIdentity {
  return firstMailbox(message.fromJson) || { email: "未知发件人" };
}

function senderLabel(message: AddressCarrier) {
  const sender = senderIdentity(message);
  return sender.name ? `${sender.name} <${sender.email}>` : sender.email;
}

function providerLabel(provider: string) {
  return { qq: "QQ 邮箱", "163": "163 邮箱", "126": "126 邮箱", yeah: "yeah.net", generic: "IMAP/SMTP" }[provider] || provider;
}

function sourceText(message: MailMessage) {
  return `来源邮件：${message.subject || "无主题"}\n发件人：${senderLabel(message)}\n邮件时间：${formatTime(message.sentAt || message.receivedAt)}\nmail:${message.id}`;
}

function openSafeUrl(href: string) {
  try {
    const parsed = new URL(href, window.location.href);
    if (!["http:", "https:", "mailto:"].includes(parsed.protocol)) {
      toast("已阻止不安全的邮件链接", "error");
      return;
    }
    window.open(parsed.href, "_blank", "noopener,noreferrer");
  } catch {
    toast("邮件链接无效", "error");
  }
}

function PlainTextBody({ text }: { text: string }) {
  const parts = text.split(/(https?:\/\/[^\s<>"']+)/g);
  return (
    <div style={{ whiteSpace: "pre-wrap", overflowWrap: "anywhere", lineHeight: 1.75 }}>
      {parts.map((part, index) => /^https?:\/\//i.test(part) ? (
        <a
          key={`${part}-${index}`}
          href={part}
          onClick={(event) => { event.preventDefault(); openSafeUrl(part); }}
          rel="noopener noreferrer"
        >
          {part}
        </a>
      ) : <span key={`text-${index}`}>{part}</span>)}
    </div>
  );
}

function HtmlBody({ html }: { html: string }) {
  return (
    <div
      style={{ lineHeight: 1.75, overflowWrap: "anywhere" }}
      onClick={(event) => {
        const target = event.target as HTMLElement | null;
        const anchor = target?.closest("a");
        if (!anchor) return;
        const href = anchor.getAttribute("href");
        if (!href) return;
        event.preventDefault();
        openSafeUrl(href);
      }}
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}

function AccountDialog({ onClose, onCreated }: { onClose: () => void; onCreated: (account: MailAccount) => void }) {
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
    <div style={{ position: "fixed", inset: 0, zIndex: 50, background: "rgba(0,0,0,.38)", display: "grid", placeItems: "center", padding: 24 }} onMouseDown={(event) => event.target === event.currentTarget && onClose()}>
      <section style={{ ...card, width: "min(620px, 94vw)", maxHeight: "90vh", overflow: "auto", padding: 22, background: "var(--background, #fff)" }} role="dialog" aria-modal="true" aria-label="添加邮箱">
        <header style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 18 }}>
          <div><h2 style={{ margin: 0, fontSize: 20 }}>添加邮箱</h2><p style={{ margin: "6px 0 0", opacity: .65 }}>授权码只会加密保存，不会显示在账号详情中。</p></div>
          <button type="button" style={actionButton} onClick={onClose}><X size={16} />关闭</button>
        </header>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(2, minmax(0,1fr))", gap: 12 }}>
          <label>邮箱类型<select style={{ ...inputStyle, marginTop: 6 }} value={provider} onChange={(event) => setProvider(event.target.value as MailProvider)}>
            <option value="qq">QQ 邮箱</option><option value="163">163 邮箱</option><option value="126">126 邮箱</option><option value="yeah">yeah.net</option><option value="generic">通用 IMAP/SMTP</option>
          </select></label>
          <label>显示名称<input style={{ ...inputStyle, marginTop: 6 }} value={displayName} onChange={(event) => setDisplayName(event.target.value)} placeholder="可选" /></label>
          <label>邮箱地址<input style={{ ...inputStyle, marginTop: 6 }} type="email" value={emailAddress} onChange={(event) => setEmailAddress(event.target.value)} placeholder="name@example.com" /></label>
          <label>授权码<input style={{ ...inputStyle, marginTop: 6 }} type="password" autoComplete="new-password" value={authorizationCode} onChange={(event) => setAuthorizationCode(event.target.value)} placeholder="邮箱授权码，不是 LifeTrace 密码" /></label>
          {provider === "generic" ? <>
            <label>IMAP 主机<input style={{ ...inputStyle, marginTop: 6 }} value={imapHost} onChange={(event) => setImapHost(event.target.value)} placeholder="imap.example.com" /></label>
            <label>IMAP 端口<input style={{ ...inputStyle, marginTop: 6 }} value={imapPort} onChange={(event) => setImapPort(event.target.value)} inputMode="numeric" /></label>
            <label>SMTP 主机<input style={{ ...inputStyle, marginTop: 6 }} value={smtpHost} onChange={(event) => setSmtpHost(event.target.value)} placeholder="smtp.example.com" /></label>
            <label>SMTP 端口<input style={{ ...inputStyle, marginTop: 6 }} value={smtpPort} onChange={(event) => setSmtpPort(event.target.value)} inputMode="numeric" /></label>
          </> : null}
        </div>
        <footer style={{ display: "flex", justifyContent: "flex-end", gap: 10, marginTop: 20 }}>
          <button type="button" style={actionButton} onClick={onClose}>取消</button>
          <button type="button" className="hx-btn primary" disabled={saving} onClick={() => void submit()}>{saving ? <LoaderCircle size={16} /> : <Plus size={16} />}添加并测试</button>
        </footer>
      </section>
    </div>
  );
}

function ReplyPanel({ account, message, onClose }: { account: MailAccount; message: MailMessage; onClose: () => void }) {
  const [to, setTo] = useState(() => senderIdentity(message).email);
  const [subject, setSubject] = useState(() => /^re:/i.test(message.subject) ? message.subject : `Re: ${message.subject}`);
  const [body, setBody] = useState("");
  const [sending, setSending] = useState(false);

  const send = async () => {
    const recipients = to.split(/[;,\n]/).map((value) => value.trim()).filter(Boolean);
    if (!recipients.length || !body.trim()) {
      toast("请填写收件人和回复内容", "error");
      return;
    }
    const confirmed = await confirmAction({
      title: "确认发送邮件",
      description: `将通过 ${account.emailAddress} 发送给 ${recipients.join(", ")}。发送后会产生外部副作用。`,
      confirmLabel: "确认发送",
    });
    if (!confirmed) return;
    setSending(true);
    try {
      await mailApi.accounts.send(account.id, {
        to: recipients,
        subject: subject.trim(),
        bodyText: body,
        inReplyToMessageId: message.id,
        idempotencyKey: crypto.randomUUID(),
      });
      toast("邮件已发送");
      onClose();
    } catch (error) {
      toast(errorMessage(error), "error");
    } finally {
      setSending(false);
    }
  };

  return <section style={{ ...card, padding: 16, marginTop: 14 }}>
    <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 10 }}><strong>回复邮件</strong><button style={actionButton} type="button" onClick={onClose}><X size={15} /></button></div>
    <label>收件人<input style={{ ...inputStyle, margin: "5px 0 10px" }} value={to} onChange={(event) => setTo(event.target.value)} /></label>
    <label>主题<input style={{ ...inputStyle, margin: "5px 0 10px" }} value={subject} onChange={(event) => setSubject(event.target.value)} /></label>
    <label>正文<textarea style={{ ...inputStyle, marginTop: 5, minHeight: 140, resize: "vertical" }} value={body} onChange={(event) => setBody(event.target.value)} /></label>
    <div style={{ display: "flex", justifyContent: "flex-end", marginTop: 10 }}><button type="button" className="hx-btn primary" disabled={sending} onClick={() => void send()}>{sending ? <LoaderCircle size={16} /> : <Send size={16} />}发送</button></div>
  </section>;
}

type SenderGroup = {
  key: string;
  email: string;
  label: string;
  messages: MailMessageSummary[];
  unreadCount: number;
  latest: MailMessageSummary;
};

type BackTarget = { kind: "inbox" } | { kind: "sender"; senderKey: string };
type MailScreen = BackTarget | { kind: "detail"; messageId: string; back: BackTarget };

function groupBySender(messages: MailMessageSummary[]): SenderGroup[] {
  const map = new Map<string, SenderGroup>();
  for (const message of messages) {
    const identity = senderIdentity(message);
    const hasAddress = identity.email.includes("@");
    const key = hasAddress ? identity.email.toLowerCase() : `unknown:${message.id}`;
    const existing = map.get(key);
    if (existing) {
      existing.messages.push(message);
      if (!message.isRead) existing.unreadCount += 1;
      continue;
    }
    map.set(key, {
      key,
      email: identity.email,
      label: identity.name || identity.email,
      messages: [message],
      unreadCount: message.isRead ? 0 : 1,
      latest: message,
    });
  }
  return [...map.values()].sort((left, right) =>
    new Date(right.latest.receivedAt).getTime() - new Date(left.latest.receivedAt).getTime());
}

function MailRow({ message, onOpen }: { message: MailMessageSummary; onOpen: () => void }) {
  return (
    <button
      type="button"
      onClick={onOpen}
      style={{
        width: "100%",
        border: 0,
        borderBottom: "1px solid var(--line, rgba(128,128,128,.15))",
        background: message.isRead ? "transparent" : "rgba(91,124,255,.055)",
        color: "inherit",
        padding: "14px 16px",
        cursor: "pointer",
        textAlign: "left",
        display: "grid",
        gridTemplateColumns: "minmax(150px, 220px) minmax(0, 1fr) auto",
        alignItems: "center",
        gap: 18,
      }}
    >
      <div style={{ minWidth: 0, display: "flex", gap: 8, alignItems: "center" }}>
        {!message.isRead ? <span aria-label="未读" style={{ width: 7, height: 7, borderRadius: "50%", background: "var(--accent, #5b7cff)", flex: "0 0 auto" }} /> : null}
        <strong style={{ fontSize: 13.5, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{senderIdentity(message).name || senderIdentity(message).email}</strong>
      </div>
      <div style={{ minWidth: 0, display: "flex", alignItems: "center", gap: 8 }}>
        <span style={{ fontSize: 13.5, fontWeight: message.isRead ? 500 : 700, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{message.subject || "无主题"}</span>
        {message.snippet ? <span style={{ fontSize: 12.5, opacity: .52, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>— {message.snippet}</span> : null}
        {message.hasAttachments ? <Paperclip size={14} style={{ opacity: .5, flex: "0 0 auto" }} /> : null}
      </div>
      <div style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 12.5, opacity: .6, whiteSpace: "nowrap" }}>
        {formatTime(message.receivedAt)}<ChevronRight size={15} />
      </div>
    </button>
  );
}

export default function MailActionCenter() {
  const [accounts, setAccounts] = useState<MailAccount[]>([]);
  const [selectedAccountId, setSelectedAccountId] = useState("");
  const [messages, setMessages] = useState<MailMessageSummary[]>([]);
  const [search, setSearch] = useState("");
  const [unreadOnly, setUnreadOnly] = useState(false);
  const [loading, setLoading] = useState(true);
  const [listLoading, setListLoading] = useState(false);
  const [busy, setBusy] = useState(false);
  const [accountDialog, setAccountDialog] = useState(false);
  const [screen, setScreen] = useState<MailScreen>({ kind: "inbox" });
  const [detailMessage, setDetailMessage] = useState<MailMessage | null>(null);
  const [attachments, setAttachments] = useState<MailAttachment[]>([]);
  const [detailLoading, setDetailLoading] = useState(false);
  const [replying, setReplying] = useState(false);

  const selectedAccount = useMemo(() => accounts.find((item) => item.id === selectedAccountId) || null, [accounts, selectedAccountId]);
  const senderGroups = useMemo(() => groupBySender(messages), [messages]);
  const activeSenderGroup = screen.kind === "sender"
    ? senderGroups.find((group) => group.key === screen.senderKey) || null
    : screen.kind === "detail" && screen.back.kind === "sender"
      ? senderGroups.find((group) => group.key === screen.back.senderKey) || null
      : null;

  const loadAccounts = useCallback(async () => {
    const next = await mailApi.accounts.list();
    setAccounts(next);
    setSelectedAccountId((current) => current && next.some((item) => item.id === current) ? current : next[0]?.id || "");
  }, []);

  const loadMessages = useCallback(async (accountId: string, q: string, onlyUnread: boolean) => {
    if (!accountId) {
      setMessages([]);
      return;
    }
    setListLoading(true);
    try {
      const all: MailMessageSummary[] = [];
      let offset = 0;
      for (;;) {
        const page = await mailApi.messages.list({
          accountId,
          q: q.trim() || undefined,
          unreadOnly: onlyUnread || undefined,
          limit: 500,
          offset,
        });
        all.push(...page.items);
        if (!page.hasMore || page.nextOffset <= offset) break;
        offset = page.nextOffset;
      }
      setMessages(all);
    } finally {
      setListLoading(false);
    }
  }, []);

  useEffect(() => {
    void (async () => {
      try {
        await loadAccounts();
      } catch (error) {
        toast(errorMessage(error), "error");
      } finally {
        setLoading(false);
      }
    })();
  }, [loadAccounts]);

  useEffect(() => {
    if (!selectedAccountId) {
      setMessages([]);
      return;
    }
    const timer = window.setTimeout(() => {
      void loadMessages(selectedAccountId, search, unreadOnly).catch((error) => toast(errorMessage(error), "error"));
    }, 250);
    return () => window.clearTimeout(timer);
  }, [selectedAccountId, search, unreadOnly, loadMessages]);

  useEffect(() => {
    if (screen.kind !== "detail") {
      setDetailMessage(null);
      setAttachments([]);
      setReplying(false);
      return;
    }
    let cancelled = false;
    setDetailLoading(true);
    void Promise.all([
      mailApi.messages.get(screen.messageId),
      mailApi.messages.attachments(screen.messageId),
    ]).then(([message, nextAttachments]) => {
      if (cancelled) return;
      setDetailMessage(message);
      setAttachments(nextAttachments);
    }).catch((error) => {
      if (!cancelled) toast(errorMessage(error), "error");
    }).finally(() => {
      if (!cancelled) setDetailLoading(false);
    });
    return () => { cancelled = true; };
  }, [screen]);

  const sync = async () => {
    if (!selectedAccount) return;
    setBusy(true);
    try {
      const result = await mailApi.accounts.sync(selectedAccount.id);
      await Promise.all([loadAccounts(), loadMessages(selectedAccount.id, search, unreadOnly)]);
      toast(`同步完成，新增/更新 ${result.persisted} 封邮件`);
    } catch (error) {
      toast(errorMessage(error), "error");
    } finally {
      setBusy(false);
    }
  };

  const disconnect = async () => {
    if (!selectedAccount) return;
    const confirmed = await confirmAction({ title: "断开邮箱", description: `断开 ${selectedAccount.emailAddress} 后，云端 Worker 将停止同步并使已保存授权凭据失效。`, confirmLabel: "断开" });
    if (!confirmed) return;
    setBusy(true);
    try {
      await mailApi.accounts.disconnect(selectedAccount.id);
      await loadAccounts();
      setScreen({ kind: "inbox" });
      toast("邮箱已断开");
    } catch (error) {
      toast(errorMessage(error), "error");
    } finally {
      setBusy(false);
    }
  };

  const setRead = async () => {
    if (!detailMessage) return;
    const nextRead = !detailMessage.isRead;
    setBusy(true);
    try {
      await mailApi.messages.setRead(detailMessage.id, nextRead);
      setDetailMessage((current) => current ? { ...current, isRead: nextRead } : current);
      setMessages((current) => current.map((item) => item.id === detailMessage.id ? { ...item, isRead: nextRead } : item));
    } catch (error) {
      toast(errorMessage(error), "error");
    } finally {
      setBusy(false);
    }
  };

  const archive = async () => {
    if (!detailMessage) return;
    const confirmed = await confirmAction({ title: "归档邮件", description: "该操作会同步修改远端邮箱中的邮件位置。", confirmLabel: "归档" });
    if (!confirmed) return;
    setBusy(true);
    try {
      await mailApi.messages.archive(detailMessage.id);
      const back = screen.kind === "detail" ? screen.back : { kind: "inbox" as const };
      setMessages((current) => current.filter((item) => item.id !== detailMessage.id));
      setScreen(back);
      toast("邮件已归档");
    } catch (error) {
      toast(errorMessage(error), "error");
    } finally {
      setBusy(false);
    }
  };

  const convert = async (kind: "task" | "event" | "memo" | "waiting") => {
    if (!detailMessage) return;
    const source = sourceText(detailMessage);
    const title = detailMessage.subject || "来自邮件的事项";
    setBusy(true);
    try {
      if (kind === "task") {
        await executionApi.tasks.create({ title, description: `${detailMessage.snippet || ""}\n\n${source}`, priority: "normal", timezone: browserTimezone(), context: `mail:${detailMessage.id}` });
      } else if (kind === "memo") {
        await executionApi.memos.create({ content: `${title}\n\n${detailMessage.bodyText || detailMessage.snippet || ""}\n\n${source}`, context: `mail:${detailMessage.id}`, tags: ["邮件"] });
      } else if (kind === "waiting") {
        await executionApi.waiting.create({ title, description: `${detailMessage.snippet || ""}\n\n${source}`, waitingFor: senderIdentity(detailMessage).email });
      } else {
        const now = new Date();
        const end = new Date(now.getTime() + 30 * 60_000);
        await executionApi.calendar.create({ title, description: `${detailMessage.snippet || ""}\n\n${source}`, isAllDay: false, startAt: now.toISOString(), endAt: end.toISOString(), timezone: browserTimezone() });
      }
      toast({ task: "已创建任务", event: "已创建日历事件", memo: "已创建 Memo", waiting: "已创建等待事项" }[kind]);
    } catch (error) {
      toast(errorMessage(error), "error");
    } finally {
      setBusy(false);
    }
  };

  const downloadAttachment = async (attachment: MailAttachment) => {
    setBusy(true);
    try {
      const blob = await mailApi.messages.downloadAttachment(attachment.id);
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = attachment.filename || "attachment";
      anchor.click();
      window.setTimeout(() => URL.revokeObjectURL(url), 1000);
    } catch (error) {
      toast(errorMessage(error), "error");
    } finally {
      setBusy(false);
    }
  };

  if (loading) {
    return <div style={{ padding: 30, display: "flex", alignItems: "center", gap: 10 }}><LoaderCircle size={18} />正在加载邮件…</div>;
  }

  if (screen.kind === "detail") {
    return (
      <div style={{ display: "grid", gap: 14, maxWidth: 1120, margin: "0 auto", width: "100%" }}>
        <button type="button" style={{ ...actionButton, width: "fit-content" }} onClick={() => setScreen(screen.back)}><ArrowLeft size={16} />{screen.back.kind === "sender" ? "返回邮件列表" : "返回收件箱"}</button>
        {detailLoading ? <section style={{ ...card, padding: 36, display: "flex", alignItems: "center", justifyContent: "center", gap: 10 }}><LoaderCircle size={18} />正在加载邮件内容…</section> : detailMessage && selectedAccount ? <>
          <section style={{ ...card, padding: "22px 24px" }}>
            <h1 style={{ margin: 0, fontSize: 22, lineHeight: 1.4 }}>{detailMessage.subject || "无主题"}</h1>
            <div style={{ display: "grid", gap: 5, marginTop: 14, fontSize: 13.5, opacity: .72 }}>
              <div>发件人：{senderLabel(detailMessage)}</div>
              <div>收件人：{collectAddresses(detailMessage.toJson).join(", ") || "未提供"}</div>
              <div>时间：{formatTime(detailMessage.sentAt || detailMessage.receivedAt)}</div>
            </div>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 18 }}>
              <button type="button" style={actionButton} disabled={busy} onClick={() => void setRead()}>{detailMessage.isRead ? <Mail size={15} /> : <CheckCircle2 size={15} />}{detailMessage.isRead ? "标为未读" : "标为已读"}</button>
              <button type="button" style={actionButton} disabled={busy} onClick={() => void archive()}><Archive size={15} />归档</button>
              <button type="button" style={actionButton} onClick={() => setReplying((value) => !value)}><Reply size={15} />回复</button>
            </div>
          </section>

          <section style={{ ...card, padding: "24px", minHeight: 300 }}>
            {detailMessage.bodyHtmlSanitized ? <HtmlBody html={detailMessage.bodyHtmlSanitized} /> : <PlainTextBody text={detailMessage.bodyText || detailMessage.snippet || "（无正文）"} />}
          </section>

          {attachments.length ? <section style={{ ...card, padding: 18 }}>
            <h3 style={{ margin: "0 0 10px", fontSize: 15 }}>附件 · {attachments.length}</h3>
            <div style={{ display: "grid", gap: 8 }}>
              {attachments.map((attachment) => <button key={attachment.id} type="button" style={{ ...actionButton, justifyContent: "space-between", width: "100%" }} disabled={busy} onClick={() => void downloadAttachment(attachment)}>
                <span style={{ display: "flex", alignItems: "center", gap: 8, minWidth: 0 }}><Paperclip size={15} /><span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{attachment.filename || "未命名附件"}</span><span style={{ opacity: .55 }}>{formatBytes(attachment.sizeBytes)}</span></span><Download size={15} />
              </button>)}
            </div>
          </section> : null}

          <section style={{ ...card, padding: 18 }}>
            <h3 style={{ margin: "0 0 7px", fontSize: 15 }}>转为 LifeTrace 行动</h3>
            <p style={{ margin: "0 0 12px", fontSize: 13, opacity: .62 }}>由你明确选择，不使用 AI 自动判断。</p>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(4, minmax(0, 1fr))", gap: 8 }}>
              <button type="button" style={actionButton} disabled={busy} onClick={() => void convert("task")}><ListTodo size={15} />创建任务</button>
              <button type="button" style={actionButton} disabled={busy} onClick={() => void convert("event")}><CalendarPlus size={15} />创建事件</button>
              <button type="button" style={actionButton} disabled={busy} onClick={() => void convert("memo")}><StickyNote size={15} />创建 Memo</button>
              <button type="button" style={actionButton} disabled={busy} onClick={() => void convert("waiting")}><Clock3 size={15} />等待事项</button>
            </div>
            {replying ? <ReplyPanel account={selectedAccount} message={detailMessage} onClose={() => setReplying(false)} /> : null}
          </section>
        </> : <section style={{ ...card, padding: 36, textAlign: "center" }}>邮件不存在或已无法读取。</section>}
      </div>
    );
  }

  if (screen.kind === "sender") {
    if (!activeSenderGroup) {
      return <div style={{ display: "grid", gap: 12 }}><button type="button" style={{ ...actionButton, width: "fit-content" }} onClick={() => setScreen({ kind: "inbox" })}><ArrowLeft size={16} />返回收件箱</button><section style={{ ...card, padding: 32, textAlign: "center" }}>该发件人当前没有匹配邮件。</section></div>;
    }
    return (
      <div style={{ display: "grid", gap: 14 }}>
        <button type="button" style={{ ...actionButton, width: "fit-content" }} onClick={() => setScreen({ kind: "inbox" })}><ArrowLeft size={16} />返回收件箱</button>
        <section style={{ ...card }}>
          <header style={{ padding: "20px 18px 14px", borderBottom: "1px solid var(--line, rgba(128,128,128,.15))" }}>
            <h1 style={{ margin: 0, fontSize: 22 }}>{activeSenderGroup.label}</h1>
            <p style={{ margin: "5px 0 0", fontSize: 13, opacity: .62 }}>{activeSenderGroup.email} · {activeSenderGroup.messages.length} 封邮件</p>
          </header>
          <div>{activeSenderGroup.messages.map((message) => <MailRow key={message.id} message={message} onOpen={() => setScreen({ kind: "detail", messageId: message.id, back: { kind: "sender", senderKey: activeSenderGroup.key } })} />)}</div>
        </section>
      </div>
    );
  }

  return (
    <div style={{ display: "grid", gap: 14, minHeight: "calc(100vh - 150px)" }}>
      <header style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center", flexWrap: "wrap" }}>
        <div><h1 style={{ margin: 0, fontSize: 24 }}>邮箱</h1><p style={{ margin: "5px 0 0", opacity: .62 }}>最近 30 天收件箱 · 同一发件人自动聚合</p></div>
        <div style={{ display: "flex", gap: 8 }}>
          <button type="button" style={actionButton} onClick={() => setAccountDialog(true)}><Plus size={16} />添加邮箱</button>
          <button type="button" className="hx-btn primary" disabled={!selectedAccount || busy} onClick={() => void sync()}><RefreshCw size={16} />同步</button>
        </div>
      </header>

      {accounts.length === 0 ? <section style={{ ...card, padding: 40, textAlign: "center" }}><Mail size={32} style={{ opacity: .45 }} /><h2>还没有连接邮箱</h2><p style={{ opacity: .62 }}>支持 QQ、163、126、yeah.net 和通用 IMAP/SMTP。</p><button type="button" className="hx-btn primary" onClick={() => setAccountDialog(true)}><Plus size={16} />添加邮箱</button></section> : <>
        <section style={{ ...card, padding: 12, display: "flex", gap: 10, alignItems: "center", flexWrap: "wrap" }}>
          <select style={{ ...inputStyle, width: "auto", minWidth: 220 }} value={selectedAccountId} onChange={(event) => { setSelectedAccountId(event.target.value); setSearch(""); setScreen({ kind: "inbox" }); }}>
            {accounts.map((account) => <option key={account.id} value={account.id}>{providerLabel(account.provider)} · {account.emailAddress}</option>)}
          </select>
          {selectedAccount ? <><span style={{ fontSize: 13, opacity: .72 }}>{selectedAccount.status === "active" ? "已连接" : selectedAccount.status === "degraded" ? "仅部分服务可用" : "待验证"}{selectedAccount.idleSupported ? " · IDLE" : " · 轮询"}</span><span style={{ fontSize: 13, opacity: .62 }}>上次同步 {formatTime(selectedAccount.lastSyncAt) || "尚未同步"}</span><button type="button" style={{ ...actionButton, marginLeft: "auto" }} disabled={busy} onClick={() => void disconnect()}>断开</button></> : null}
        </section>

        <section style={{ ...card }}>
          <div style={{ padding: 12, display: "flex", gap: 10, alignItems: "center", borderBottom: "1px solid var(--line, rgba(128,128,128,.15))" }}>
            <div style={{ position: "relative", flex: 1 }}><Search size={15} style={{ position: "absolute", left: 10, top: 12, opacity: .5 }} /><input aria-label="搜索邮件" style={{ ...inputStyle, paddingLeft: 31 }} value={search} onChange={(event) => setSearch(event.target.value)} placeholder="搜索发件人、主题或摘要" /></div>
            <label style={{ display: "flex", alignItems: "center", gap: 7, fontSize: 13, whiteSpace: "nowrap" }}><input type="checkbox" checked={unreadOnly} onChange={(event) => setUnreadOnly(event.target.checked)} />只看未读</label>
            {listLoading ? <span style={{ display: "inline-flex", alignItems: "center", gap: 6, fontSize: 12.5, opacity: .6 }}><LoaderCircle size={14} />加载中</span> : null}
          </div>

          {senderGroups.length ? <div>
            {senderGroups.map((group) => {
              const grouped = group.messages.length > 1;
              return <button
                key={group.key}
                type="button"
                onClick={() => grouped
                  ? setScreen({ kind: "sender", senderKey: group.key })
                  : setScreen({ kind: "detail", messageId: group.latest.id, back: { kind: "inbox" } })}
                onContextMenu={(event) => {
                  if (!grouped) return;
                  event.preventDefault();
                  setScreen({ kind: "sender", senderKey: group.key });
                }}
                style={{
                  width: "100%",
                  border: 0,
                  borderBottom: "1px solid var(--line, rgba(128,128,128,.15))",
                  background: group.unreadCount ? "rgba(91,124,255,.055)" : "transparent",
                  color: "inherit",
                  padding: "15px 16px",
                  cursor: "pointer",
                  textAlign: "left",
                  display: "grid",
                  gridTemplateColumns: "minmax(160px, 230px) minmax(0, 1fr) auto",
                  alignItems: "center",
                  gap: 18,
                }}
              >
                <div style={{ minWidth: 0, display: "flex", gap: 8, alignItems: "center" }}>
                  {group.unreadCount ? <span aria-label="包含未读邮件" style={{ width: 7, height: 7, borderRadius: "50%", background: "var(--accent, #5b7cff)", flex: "0 0 auto" }} /> : null}
                  <strong style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", fontSize: 13.5 }}>{group.label}</strong>
                  {grouped ? <span style={{ borderRadius: 10, padding: "2px 7px", fontSize: 11.5, background: "rgba(91,124,255,.12)", color: "var(--accent, #5b7cff)", whiteSpace: "nowrap" }}>{group.messages.length} 封</span> : null}
                </div>
                <div style={{ minWidth: 0, display: "flex", alignItems: "center", gap: 8 }}>
                  <span style={{ fontSize: 13.5, fontWeight: group.unreadCount ? 700 : 500, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{group.latest.subject || "无主题"}</span>
                  {group.latest.snippet ? <span style={{ fontSize: 12.5, opacity: .52, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>— {group.latest.snippet}</span> : null}
                  {group.latest.hasAttachments ? <Paperclip size={14} style={{ opacity: .5, flex: "0 0 auto" }} /> : null}
                </div>
                <div style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 12.5, opacity: .6, whiteSpace: "nowrap" }}>{formatTime(group.latest.receivedAt)}<ChevronRight size={15} /></div>
              </button>;
            })}
          </div> : <div style={{ padding: 42, opacity: .55, textAlign: "center" }}><Inbox size={28} /><p>{listLoading ? "正在读取最近 30 天邮件…" : "没有匹配邮件"}</p></div>}
        </section>
      </>}
      {accountDialog ? <AccountDialog onClose={() => setAccountDialog(false)} onCreated={(account) => { setAccounts((current) => [...current, account]); setSelectedAccountId(account.id); setScreen({ kind: "inbox" }); }} /> : null}
    </div>
  );
}
