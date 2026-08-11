import { useEffect, useState } from "react";
import {
  Archive,
  ArrowLeft,
  CalendarPlus,
  CheckCircle2,
  Clock3,
  Download,
  ListTodo,
  LoaderCircle,
  Mail,
  Paperclip,
  Reply,
  StickyNote,
} from "lucide-react";
import { browserTimezone, executionApi } from "@/src/services/executionApi";
import { mailApi, type MailAccount, type MailAttachment, type MailMessage } from "@/src/services/mailApi";
import { confirmAction } from "@/src/ui/feedback/confirm";
import { collectAddresses, senderIdentity, senderLabel } from "./mailModel";
import { actionButton, errorMessage, formatBytes, formatTime, panelStyle, toast } from "./mailStyles";

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

function PlainBody({ text }: { text: string }) {
  const parts = text.split(/(https?:\/\/[^\s<>"']+)/g);
  return <div style={{ whiteSpace: "pre-wrap", overflowWrap: "anywhere", lineHeight: 1.75 }}>
    {parts.map((part, index) => /^https?:\/\//i.test(part)
      ? <a key={`${part}-${index}`} href={part} onClick={(event) => { event.preventDefault(); openSafeUrl(part); }} rel="noopener noreferrer">{part}</a>
      : <span key={`text-${index}`}>{part}</span>)}
  </div>;
}

function HtmlBody({ html }: { html: string }) {
  return <div
    style={{ lineHeight: 1.75, overflowWrap: "anywhere" }}
    onClick={(event) => {
      const target = event.target as HTMLElement | null;
      const anchor = target?.closest("a");
      const href = anchor?.getAttribute("href");
      if (!href) return;
      event.preventDefault();
      openSafeUrl(href);
    }}
    dangerouslySetInnerHTML={{ __html: html }}
  />;
}

export function MailMessageDetail({
  messageId,
  accounts,
  backLabel,
  onBack,
  onReply,
  onMessagePatch,
  onArchived,
}: {
  messageId: string;
  accounts: MailAccount[];
  backLabel: string;
  onBack: () => void;
  onReply: (message: MailMessage) => void;
  onMessagePatch: (id: string, patch: Partial<MailMessage>) => void;
  onArchived: (id: string) => void;
}) {
  const [message, setMessage] = useState<MailMessage | null>(null);
  const [attachments, setAttachments] = useState<MailAttachment[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    void Promise.all([
      mailApi.messages.get(messageId),
      mailApi.messages.attachments(messageId),
    ]).then(([nextMessage, nextAttachments]) => {
      if (cancelled) return;
      setMessage(nextMessage);
      setAttachments(nextAttachments);
    }).catch((error) => {
      if (!cancelled) toast(errorMessage(error), "error");
    }).finally(() => {
      if (!cancelled) setLoading(false);
    });
    return () => { cancelled = true; };
  }, [messageId]);

  const account = message ? accounts.find((item) => item.id === message.accountId) || null : null;

  const setRead = async () => {
    if (!message) return;
    const nextRead = !message.isRead;
    setBusy(true);
    try {
      await mailApi.messages.setRead(message.id, nextRead);
      setMessage((current) => current ? { ...current, isRead: nextRead } : current);
      onMessagePatch(message.id, { isRead: nextRead });
    } catch (error) {
      toast(errorMessage(error), "error");
    } finally {
      setBusy(false);
    }
  };

  const archive = async () => {
    if (!message) return;
    const confirmed = await confirmAction({ title: "归档邮件", description: "该操作会同步修改远端邮箱中的邮件位置。", confirmLabel: "归档" });
    if (!confirmed) return;
    setBusy(true);
    try {
      await mailApi.messages.archive(message.id);
      onArchived(message.id);
      toast("邮件已归档");
    } catch (error) {
      toast(errorMessage(error), "error");
    } finally {
      setBusy(false);
    }
  };

  const convert = async (kind: "task" | "event" | "memo" | "waiting") => {
    if (!message) return;
    const source = sourceText(message);
    const title = message.subject || "来自邮件的事项";
    setBusy(true);
    try {
      if (kind === "task") {
        await executionApi.tasks.create({ title, description: `${message.snippet || ""}\n\n${source}`, priority: "normal", timezone: browserTimezone(), context: `mail:${message.id}` });
      } else if (kind === "memo") {
        await executionApi.memos.create({ content: `${title}\n\n${message.bodyText || message.snippet || ""}\n\n${source}`, context: `mail:${message.id}`, tags: ["邮件"] });
      } else if (kind === "waiting") {
        await executionApi.waiting.create({ title, description: `${message.snippet || ""}\n\n${source}`, waitingFor: senderIdentity(message).email });
      } else {
        const now = new Date();
        const end = new Date(now.getTime() + 30 * 60_000);
        await executionApi.calendar.create({ title, description: `${message.snippet || ""}\n\n${source}`, isAllDay: false, startAt: now.toISOString(), endAt: end.toISOString(), timezone: browserTimezone() });
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

  return (
    <div style={{ display: "grid", gap: 14, maxWidth: 1120, margin: "0 auto", width: "100%" }}>
      <button type="button" style={{ ...actionButton, width: "fit-content" }} onClick={onBack}><ArrowLeft size={16} />{backLabel}</button>
      {loading ? <section style={{ ...panelStyle, padding: 40, display: "flex", justifyContent: "center", gap: 9 }}><LoaderCircle size={18} />正在加载邮件内容…</section> : message ? <>
        <section style={{ ...panelStyle, padding: "22px 24px" }}>
          <h1 style={{ margin: 0, fontSize: 22, lineHeight: 1.4 }}>{message.subject || "无主题"}</h1>
          <div style={{ display: "grid", gap: 5, marginTop: 14, fontSize: 13, opacity: .7 }}>
            <div>发件人：{senderLabel(message)}</div>
            <div>收件人：{collectAddresses(message.toJson).join(", ") || "未提供"}</div>
            <div>邮箱：{account?.displayName || account?.emailAddress || "未知账户"}</div>
            <div>时间：{formatTime(message.sentAt || message.receivedAt)}</div>
          </div>
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 18 }}>
            <button type="button" style={actionButton} disabled={busy} onClick={() => void setRead()}>{message.isRead ? <Mail size={15} /> : <CheckCircle2 size={15} />}{message.isRead ? "标为未读" : "标为已读"}</button>
            <button type="button" style={actionButton} disabled={busy} onClick={() => void archive()}><Archive size={15} />归档</button>
            <button type="button" style={actionButton} disabled={busy || !account} onClick={() => onReply(message)}><Reply size={15} />回复</button>
          </div>
        </section>

        <section style={{ ...panelStyle, padding: 24, minHeight: 320 }}>
          {message.bodyHtmlSanitized ? <HtmlBody html={message.bodyHtmlSanitized} /> : <PlainBody text={message.bodyText || message.snippet || "（无正文）"} />}
        </section>

        {attachments.length ? <section style={{ ...panelStyle, padding: 18 }}>
          <h3 style={{ margin: "0 0 10px", fontSize: 15 }}>附件 · {attachments.length}</h3>
          <div style={{ display: "grid", gap: 8 }}>
            {attachments.map((attachment) => <button key={attachment.id} type="button" style={{ ...actionButton, justifyContent: "space-between", width: "100%" }} disabled={busy} onClick={() => void downloadAttachment(attachment)}>
              <span style={{ display: "flex", alignItems: "center", gap: 8, minWidth: 0 }}><Paperclip size={15} /><span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{attachment.filename || "未命名附件"}</span><span style={{ opacity: .55 }}>{formatBytes(attachment.sizeBytes)}</span></span><Download size={15} />
            </button>)}
          </div>
        </section> : null}

        <section style={{ ...panelStyle, padding: 18 }}>
          <h3 style={{ margin: "0 0 7px", fontSize: 15 }}>转为 LifeTrace 行动</h3>
          <p style={{ margin: "0 0 12px", fontSize: 13, opacity: .6 }}>由你明确选择，不自动判断邮件意图。</p>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(4, minmax(0,1fr))", gap: 8 }}>
            <button type="button" style={actionButton} disabled={busy} onClick={() => void convert("task")}><ListTodo size={15} />创建任务</button>
            <button type="button" style={actionButton} disabled={busy} onClick={() => void convert("event")}><CalendarPlus size={15} />创建事件</button>
            <button type="button" style={actionButton} disabled={busy} onClick={() => void convert("memo")}><StickyNote size={15} />创建 Memo</button>
            <button type="button" style={actionButton} disabled={busy} onClick={() => void convert("waiting")}><Clock3 size={15} />等待事项</button>
          </div>
        </section>
      </> : <section style={{ ...panelStyle, padding: 40, textAlign: "center" }}>邮件不存在或已无法读取。</section>}
    </div>
  );
}
