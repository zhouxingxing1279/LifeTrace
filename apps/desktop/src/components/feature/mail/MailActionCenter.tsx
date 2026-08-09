import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Archive,
  CalendarPlus,
  CheckCircle2,
  Clock3,
  Inbox,
  ListTodo,
  LoaderCircle,
  Mail,
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
  type MailMessage,
  type MailProvider,
  type MailThread,
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
  return new Intl.DateTimeFormat("zh-CN", {
    month: "numeric",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

function collectAddresses(value: unknown): string[] {
  const result: string[] = [];
  const visit = (current: unknown) => {
    if (typeof current === "string") {
      if (current.includes("@")) result.push(current);
      return;
    }
    if (Array.isArray(current)) {
      current.forEach(visit);
      return;
    }
    if (current && typeof current === "object") {
      for (const [key, nested] of Object.entries(current as Record<string, unknown>)) {
        if ((key === "address" || key === "email") && typeof nested === "string" && nested.includes("@")) {
          result.push(nested);
        } else {
          visit(nested);
        }
      }
    }
  };
  visit(value);
  return [...new Set(result)];
}

function senderLabel(message?: MailMessage | null) {
  if (!message) return "";
  return collectAddresses(message.fromJson)[0] || "未知发件人";
}

function providerLabel(provider: string) {
  return { qq: "QQ 邮箱", "163": "163 邮箱", "126": "126 邮箱", yeah: "yeah.net", generic: "IMAP/SMTP" }[provider] || provider;
}

function sourceText(message: MailMessage) {
  return `来源邮件：${message.subject || "无主题"}\n发件人：${senderLabel(message)}\n邮件时间：${formatTime(message.sentAt || message.receivedAt)}\nmail:${message.id}`;
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
          <label>邮箱类型<select style={{ ...inputStyle, marginTop: 6 }} value={provider} onChange={(e) => setProvider(e.target.value as MailProvider)}>
            <option value="qq">QQ 邮箱</option><option value="163">163 邮箱</option><option value="126">126 邮箱</option><option value="yeah">yeah.net</option><option value="generic">通用 IMAP/SMTP</option>
          </select></label>
          <label>显示名称<input style={{ ...inputStyle, marginTop: 6 }} value={displayName} onChange={(e) => setDisplayName(e.target.value)} placeholder="可选" /></label>
          <label>邮箱地址<input style={{ ...inputStyle, marginTop: 6 }} type="email" value={emailAddress} onChange={(e) => setEmailAddress(e.target.value)} placeholder="name@example.com" /></label>
          <label>授权码<input style={{ ...inputStyle, marginTop: 6 }} type="password" autoComplete="new-password" value={authorizationCode} onChange={(e) => setAuthorizationCode(e.target.value)} placeholder="邮箱授权码，不是 LifeTrace 密码" /></label>
          {provider === "generic" ? <>
            <label>IMAP 主机<input style={{ ...inputStyle, marginTop: 6 }} value={imapHost} onChange={(e) => setImapHost(e.target.value)} placeholder="imap.example.com" /></label>
            <label>IMAP 端口<input style={{ ...inputStyle, marginTop: 6 }} value={imapPort} onChange={(e) => setImapPort(e.target.value)} inputMode="numeric" /></label>
            <label>SMTP 主机<input style={{ ...inputStyle, marginTop: 6 }} value={smtpHost} onChange={(e) => setSmtpHost(e.target.value)} placeholder="smtp.example.com" /></label>
            <label>SMTP 端口<input style={{ ...inputStyle, marginTop: 6 }} value={smtpPort} onChange={(e) => setSmtpPort(e.target.value)} inputMode="numeric" /></label>
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
  const [to, setTo] = useState(() => senderLabel(message));
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
    <label>收件人<input style={{ ...inputStyle, margin: "5px 0 10px" }} value={to} onChange={(e) => setTo(e.target.value)} /></label>
    <label>主题<input style={{ ...inputStyle, margin: "5px 0 10px" }} value={subject} onChange={(e) => setSubject(e.target.value)} /></label>
    <label>正文<textarea style={{ ...inputStyle, marginTop: 5, minHeight: 140, resize: "vertical" }} value={body} onChange={(e) => setBody(e.target.value)} /></label>
    <div style={{ display: "flex", justifyContent: "flex-end", marginTop: 10 }}><button type="button" className="hx-btn primary" disabled={sending} onClick={() => void send()}>{sending ? <LoaderCircle size={16} /> : <Send size={16} />}发送</button></div>
  </section>;
}

export default function MailActionCenter() {
  const [accounts, setAccounts] = useState<MailAccount[]>([]);
  const [selectedAccountId, setSelectedAccountId] = useState<string>("");
  const [threads, setThreads] = useState<MailThread[]>([]);
  const [selectedThreadId, setSelectedThreadId] = useState<string>("");
  const [messages, setMessages] = useState<MailMessage[]>([]);
  const [search, setSearch] = useState("");
  const [unreadOnly, setUnreadOnly] = useState(false);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [accountDialog, setAccountDialog] = useState(false);
  const [replying, setReplying] = useState(false);

  const selectedAccount = useMemo(() => accounts.find((item) => item.id === selectedAccountId) || null, [accounts, selectedAccountId]);
  const selectedThread = useMemo(() => threads.find((item) => item.id === selectedThreadId) || null, [threads, selectedThreadId]);
  const latestMessage = messages[messages.length - 1] || null;

  const loadAccounts = useCallback(async () => {
    const next = await mailApi.accounts.list();
    setAccounts(next);
    setSelectedAccountId((current) => current && next.some((item) => item.id === current) ? current : next[0]?.id || "");
  }, []);

  const loadThreads = useCallback(async (accountId: string, q = search, onlyUnread = unreadOnly) => {
    if (!accountId) { setThreads([]); return; }
    const next = await mailApi.threads.list({ accountId, q: q.trim() || undefined, unreadOnly: onlyUnread || undefined, limit: 200 });
    setThreads(next);
    setSelectedThreadId((current) => current && next.some((item) => item.id === current) ? current : next[0]?.id || "");
  }, [search, unreadOnly]);

  useEffect(() => {
    void (async () => {
      try { await loadAccounts(); }
      catch (error) { toast(errorMessage(error), "error"); }
      finally { setLoading(false); }
    })();
  }, [loadAccounts]);

  useEffect(() => {
    if (!selectedAccountId) { setThreads([]); return; }
    void loadThreads(selectedAccountId).catch((error) => toast(errorMessage(error), "error"));
  }, [selectedAccountId, loadThreads]);

  useEffect(() => {
    if (!selectedThreadId) { setMessages([]); return; }
    void mailApi.threads.messages(selectedThreadId).then(setMessages).catch((error) => toast(errorMessage(error), "error"));
  }, [selectedThreadId]);

  const sync = async () => {
    if (!selectedAccount) return;
    setBusy(true);
    try {
      const result = await mailApi.accounts.sync(selectedAccount.id);
      await Promise.all([loadAccounts(), loadThreads(selectedAccount.id)]);
      toast(`同步完成，新增/更新 ${result.persisted} 封邮件`);
    } catch (error) { toast(errorMessage(error), "error"); }
    finally { setBusy(false); }
  };

  const setRead = async () => {
    if (!latestMessage) return;
    setBusy(true);
    try {
      await mailApi.messages.setRead(latestMessage.id, !latestMessage.isRead);
      setMessages((current) => current.map((item) => item.id === latestMessage.id ? { ...item, isRead: !latestMessage.isRead } : item));
      if (selectedAccountId) await loadThreads(selectedAccountId);
    } catch (error) { toast(errorMessage(error), "error"); }
    finally { setBusy(false); }
  };

  const archive = async () => {
    if (!latestMessage) return;
    const confirmed = await confirmAction({ title: "归档邮件", description: "该操作会同步修改远端邮箱中的邮件位置。", confirmLabel: "归档" });
    if (!confirmed) return;
    setBusy(true);
    try {
      await mailApi.messages.archive(latestMessage.id);
      toast("邮件已归档");
      if (selectedAccountId) await loadThreads(selectedAccountId);
    } catch (error) { toast(errorMessage(error), "error"); }
    finally { setBusy(false); }
  };

  const convert = async (kind: "task" | "event" | "memo" | "waiting") => {
    if (!latestMessage) return;
    const source = sourceText(latestMessage);
    const title = latestMessage.subject || "来自邮件的事项";
    setBusy(true);
    try {
      if (kind === "task") {
        await executionApi.tasks.create({ title, description: `${latestMessage.snippet || ""}\n\n${source}`, priority: "normal", timezone: browserTimezone(), context: `mail:${latestMessage.id}` });
      } else if (kind === "memo") {
        await executionApi.memos.create({ content: `${title}\n\n${latestMessage.bodyText || latestMessage.snippet || ""}\n\n${source}`, context: `mail:${latestMessage.id}`, tags: ["邮件"] });
      } else if (kind === "waiting") {
        await executionApi.waiting.create({ title, description: `${latestMessage.snippet || ""}\n\n${source}`, waitingFor: senderLabel(latestMessage) });
      } else {
        const now = new Date();
        const end = new Date(now.getTime() + 30 * 60_000);
        await executionApi.calendar.create({ title, description: `${latestMessage.snippet || ""}\n\n${source}`, isAllDay: false, startAt: now.toISOString(), endAt: end.toISOString(), timezone: browserTimezone() });
      }
      toast({ task: "已创建任务", event: "已创建日历事件", memo: "已创建 Memo", waiting: "已创建等待事项" }[kind]);
    } catch (error) { toast(errorMessage(error), "error"); }
    finally { setBusy(false); }
  };

  const disconnect = async () => {
    if (!selectedAccount) return;
    const confirmed = await confirmAction({ title: "断开邮箱", description: `断开 ${selectedAccount.emailAddress} 后，云端 Worker 将停止同步并使已保存授权凭据失效。`, confirmLabel: "断开" });
    if (!confirmed) return;
    setBusy(true);
    try {
      await mailApi.accounts.disconnect(selectedAccount.id);
      await loadAccounts();
      toast("邮箱已断开");
    } catch (error) { toast(errorMessage(error), "error"); }
    finally { setBusy(false); }
  };

  if (loading) return <div style={{ padding: 30, display: "flex", alignItems: "center", gap: 10 }}><LoaderCircle size={18} />正在加载邮件…</div>;

  return <div style={{ display: "grid", gap: 14, minHeight: "calc(100vh - 150px)" }}>
    <header style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center", flexWrap: "wrap" }}>
      <div><h1 style={{ margin: 0, fontSize: 24 }}>邮件行动中心</h1><p style={{ margin: "5px 0 0", opacity: .62 }}>同步、阅读并把邮件手动转成 LifeTrace 行动。</p></div>
      <div style={{ display: "flex", gap: 8 }}>
        <button type="button" style={actionButton} onClick={() => setAccountDialog(true)}><Plus size={16} />添加邮箱</button>
        <button type="button" className="hx-btn primary" disabled={!selectedAccount || busy} onClick={() => void sync()}><RefreshCw size={16} />同步</button>
      </div>
    </header>

    {accounts.length === 0 ? <section style={{ ...card, padding: 40, textAlign: "center" }}><Mail size={32} style={{ opacity: .45 }} /><h2>还没有连接邮箱</h2><p style={{ opacity: .62 }}>首期支持 QQ、163、126、yeah.net 和通用 IMAP/SMTP。</p><button type="button" className="hx-btn primary" onClick={() => setAccountDialog(true)}><Plus size={16} />添加邮箱</button></section> : <>
      <section style={{ ...card, padding: 12, display: "flex", gap: 10, alignItems: "center", flexWrap: "wrap" }}>
        <select style={{ ...inputStyle, width: "auto", minWidth: 220 }} value={selectedAccountId} onChange={(e) => { setSelectedAccountId(e.target.value); setSelectedThreadId(""); }}>
          {accounts.map((account) => <option key={account.id} value={account.id}>{providerLabel(account.provider)} · {account.emailAddress}</option>)}
        </select>
        {selectedAccount ? <><span style={{ fontSize: 13, opacity: .72 }}>{selectedAccount.status === "active" ? "已连接" : selectedAccount.status === "degraded" ? "仅部分服务可用" : "待验证"}{selectedAccount.idleSupported ? " · IDLE" : " · 轮询"}</span><span style={{ fontSize: 13, opacity: .62 }}>上次同步 {formatTime(selectedAccount.lastSyncAt) || "尚未同步"}</span><button type="button" style={{ ...actionButton, marginLeft: "auto" }} disabled={busy} onClick={() => void disconnect()}>断开</button></> : null}
      </section>

      <div style={{ display: "grid", gridTemplateColumns: "minmax(260px, .8fr) minmax(320px, 1fr) minmax(420px, 1.55fr)", gap: 12, minHeight: 610 }}>
        <section style={{ ...card, padding: 12 }}>
          <div style={{ display: "flex", gap: 8, marginBottom: 10 }}><div style={{ position: "relative", flex: 1 }}><Search size={15} style={{ position: "absolute", left: 10, top: 12, opacity: .5 }} /><input aria-label="搜索邮件" style={{ ...inputStyle, paddingLeft: 31 }} value={search} onChange={(e) => setSearch(e.target.value)} onKeyDown={(e) => e.key === "Enter" && selectedAccountId && void loadThreads(selectedAccountId)} placeholder="搜索主题或正文" /></div><button type="button" style={actionButton} onClick={() => selectedAccountId && void loadThreads(selectedAccountId)}>搜索</button></div>
          <label style={{ display: "flex", alignItems: "center", gap: 7, fontSize: 13, marginBottom: 10 }}><input type="checkbox" checked={unreadOnly} onChange={(e) => { setUnreadOnly(e.target.checked); if (selectedAccountId) void loadThreads(selectedAccountId, search, e.target.checked); }} />只看未读</label>
          <div style={{ display: "grid", gap: 6, maxHeight: 540, overflow: "auto" }}>{threads.length ? threads.map((thread) => <button key={thread.id} type="button" onClick={() => { setSelectedThreadId(thread.id); setReplying(false); }} style={{ textAlign: "left", border: selectedThreadId === thread.id ? "1px solid var(--accent, #5b7cff)" : "1px solid transparent", borderRadius: 10, background: selectedThreadId === thread.id ? "rgba(91,124,255,.08)" : "transparent", color: "inherit", padding: 10, cursor: "pointer" }}><div style={{ display: "flex", gap: 7, alignItems: "center" }}><strong style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", flex: 1 }}>{thread.normalizedSubject || "无主题"}</strong>{thread.unreadCount > 0 ? <span style={{ minWidth: 20, height: 20, borderRadius: 10, display: "grid", placeItems: "center", fontSize: 11, background: "var(--accent, #5b7cff)", color: "white" }}>{thread.unreadCount}</span> : null}</div><div style={{ marginTop: 5, fontSize: 12, opacity: .58 }}>{formatTime(thread.latestMessageAt)} · {thread.messageCount} 封</div><p style={{ margin: "6px 0 0", fontSize: 13, opacity: .72, overflow: "hidden", display: "-webkit-box", WebkitLineClamp: 2, WebkitBoxOrient: "vertical" }}>{thread.snippet || ""}</p></button>) : <div style={{ padding: 24, opacity: .55, textAlign: "center" }}><Inbox size={24} /><p>没有匹配邮件</p></div>}</div>
        </section>

        <section style={{ ...card, padding: 12, overflow: "auto" }}>
          <h3 style={{ margin: "3px 0 10px", fontSize: 15 }}>{selectedThread?.normalizedSubject || "邮件线程"}</h3>
          {messages.length ? <div style={{ display: "grid", gap: 9 }}>{messages.map((message) => <article key={message.id} style={{ border: "1px solid var(--line, rgba(128,128,128,.18))", borderRadius: 10, padding: 11, background: message.isRead ? "transparent" : "rgba(91,124,255,.05)" }}><div style={{ display: "flex", justifyContent: "space-between", gap: 8 }}><strong style={{ fontSize: 13 }}>{senderLabel(message)}</strong><span style={{ fontSize: 12, opacity: .55 }}>{formatTime(message.sentAt || message.receivedAt)}</span></div><p style={{ margin: "7px 0 0", whiteSpace: "pre-wrap", overflowWrap: "anywhere", lineHeight: 1.55, fontSize: 13.5 }}>{message.bodyText || message.snippet || "（无纯文本正文）"}</p>{message.hasAttachments ? <div style={{ marginTop: 8, fontSize: 12, opacity: .65 }}>含附件 · 当前按需保存元数据</div> : null}</article>)}</div> : <div style={{ padding: 30, textAlign: "center", opacity: .55 }}>选择一个邮件线程查看内容</div>}
        </section>

        <aside style={{ ...card, padding: 14, overflow: "auto" }}>
          {latestMessage && selectedAccount ? <><div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}><button type="button" style={actionButton} disabled={busy} onClick={() => void setRead()}>{latestMessage.isRead ? <Mail size={15} /> : <CheckCircle2 size={15} />}{latestMessage.isRead ? "标为未读" : "标为已读"}</button><button type="button" style={actionButton} disabled={busy} onClick={() => void archive()}><Archive size={15} />归档</button><button type="button" style={actionButton} onClick={() => setReplying((value) => !value)}><Reply size={15} />回复</button></div><h3 style={{ margin: "20px 0 8px", fontSize: 15 }}>转为 LifeTrace 行动</h3><p style={{ margin: "0 0 10px", fontSize: 13, opacity: .62 }}>由你明确选择，不使用 AI 自动判断。</p><div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}><button type="button" style={actionButton} disabled={busy} onClick={() => void convert("task")}><ListTodo size={15} />创建任务</button><button type="button" style={actionButton} disabled={busy} onClick={() => void convert("event")}><CalendarPlus size={15} />创建事件</button><button type="button" style={actionButton} disabled={busy} onClick={() => void convert("memo")}><StickyNote size={15} />创建 Memo</button><button type="button" style={actionButton} disabled={busy} onClick={() => void convert("waiting")}><Clock3 size={15} />等待事项</button></div>{replying ? <ReplyPanel account={selectedAccount} message={latestMessage} onClose={() => setReplying(false)} /> : null}<section style={{ marginTop: 18, paddingTop: 14, borderTop: "1px solid var(--line, rgba(128,128,128,.18))" }}><h3 style={{ margin: "0 0 7px", fontSize: 15 }}>{latestMessage.subject || "无主题"}</h3><div style={{ fontSize: 13, lineHeight: 1.65, opacity: .7 }}><div>发件人：{senderLabel(latestMessage)}</div><div>接收：{formatTime(latestMessage.receivedAt)}</div><div>远端 UID：{latestMessage.remoteUid}</div></div></section></> : <div style={{ padding: 28, textAlign: "center", opacity: .55 }}><Mail size={28} /><p>选择邮件后可处理行动</p></div>}
        </aside>
      </div>
    </>}
    {accountDialog ? <AccountDialog onClose={() => setAccountDialog(false)} onCreated={(account) => { setAccounts((current) => [...current, account]); setSelectedAccountId(account.id); }} /> : null}
  </div>;
}
