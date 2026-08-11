import { ChevronRight, Inbox, Paperclip } from "lucide-react";
import type { MailAccount, MailMessageSummary } from "@/src/services/mailApi";
import {
  accountLabel,
  groupBySender,
  senderIdentity,
  shouldAggregateBySender,
  type MailCollectionSource,
} from "./mailModel";
import { formatTime } from "./mailStyles";

function accountName(accounts: MailAccount[], accountId: string) {
  const account = accounts.find((item) => item.id === accountId);
  return account ? accountLabel(account) : "邮箱";
}

function MailRow({
  message,
  accounts,
  showAccount,
  onOpen,
}: {
  message: MailMessageSummary;
  accounts: MailAccount[];
  showAccount: boolean;
  onOpen: () => void;
}) {
  const sender = senderIdentity(message);
  return (
    <button
      type="button"
      onClick={onOpen}
      style={{
        width: "100%",
        border: 0,
        borderBottom: "1px solid var(--line, rgba(128,128,128,.13))",
        background: message.isRead ? "transparent" : "rgba(91,124,255,.055)",
        color: "inherit",
        padding: "13px 15px",
        cursor: "pointer",
        textAlign: "left",
        display: "grid",
        gridTemplateColumns: "minmax(150px, 220px) minmax(0, 1fr) auto",
        alignItems: "center",
        gap: 16,
      }}
    >
      <div style={{ minWidth: 0, display: "grid", gap: 3 }}>
        <div style={{ minWidth: 0, display: "flex", gap: 7, alignItems: "center" }}>
          {!message.isRead ? <span aria-label="未读" style={{ width: 7, height: 7, borderRadius: "50%", background: "var(--accent, #5b7cff)", flex: "0 0 auto" }} /> : null}
          <strong style={{ fontSize: 13.5, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{sender.name || sender.email}</strong>
        </div>
        {showAccount ? <span style={{ paddingLeft: message.isRead ? 0 : 14, fontSize: 10.5, opacity: .47, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{accountName(accounts, message.accountId)}</span> : null}
      </div>
      <div style={{ minWidth: 0, display: "flex", alignItems: "center", gap: 8 }}>
        <span style={{ fontSize: 13.5, fontWeight: message.isRead ? 500 : 700, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{message.subject || "无主题"}</span>
        {message.snippet ? <span style={{ fontSize: 12.5, opacity: .5, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>— {message.snippet}</span> : null}
        {message.hasAttachments ? <Paperclip size={14} style={{ opacity: .5, flex: "0 0 auto" }} /> : null}
      </div>
      <div style={{ display: "flex", alignItems: "center", gap: 7, fontSize: 12, opacity: .58, whiteSpace: "nowrap" }}>{formatTime(message.receivedAt)}<ChevronRight size={14} /></div>
    </button>
  );
}

export function MailMessageList({
  messages,
  source,
  senderKey,
  accounts,
  loading,
  onOpenMessage,
  onOpenSender,
}: {
  messages: MailMessageSummary[];
  source: MailCollectionSource;
  senderKey?: string;
  accounts: MailAccount[];
  loading: boolean;
  onOpenMessage: (message: MailMessageSummary) => void;
  onOpenSender: (senderKey: string) => void;
}) {
  const showAccount = source.kind === "unified" || source.kind === "unread";
  const groups = shouldAggregateBySender(source) ? groupBySender(messages) : [];

  if (senderKey) {
    const group = groups.find((item) => item.key === senderKey);
    const concrete = group?.messages || [];
    if (!concrete.length) {
      return <Empty loading={loading} label="该发件人当前没有匹配邮件" />;
    }
    return <div>{concrete.map((message) => <MailRow key={message.id} message={message} accounts={accounts} showAccount={showAccount} onOpen={() => onOpenMessage(message)} />)}</div>;
  }

  if (!messages.length) return <Empty loading={loading} label="没有匹配邮件" />;

  if (!shouldAggregateBySender(source)) {
    return <div>{messages.map((message) => <MailRow key={message.id} message={message} accounts={accounts} showAccount={false} onOpen={() => onOpenMessage(message)} />)}</div>;
  }

  return (
    <div>
      {groups.map((group) => {
        const grouped = group.messages.length > 1;
        return (
          <button
            key={group.key}
            type="button"
            onClick={() => grouped ? onOpenSender(group.key) : onOpenMessage(group.latest)}
            onContextMenu={(event) => {
              if (!grouped) return;
              event.preventDefault();
              onOpenSender(group.key);
            }}
            style={{
              width: "100%",
              border: 0,
              borderBottom: "1px solid var(--line, rgba(128,128,128,.13))",
              background: group.unreadCount ? "rgba(91,124,255,.055)" : "transparent",
              color: "inherit",
              padding: "14px 15px",
              cursor: "pointer",
              textAlign: "left",
              display: "grid",
              gridTemplateColumns: "minmax(160px, 230px) minmax(0, 1fr) auto",
              alignItems: "center",
              gap: 16,
            }}
          >
            <div style={{ minWidth: 0, display: "grid", gap: 3 }}>
              <div style={{ minWidth: 0, display: "flex", gap: 7, alignItems: "center" }}>
                {group.unreadCount ? <span aria-label="包含未读邮件" style={{ width: 7, height: 7, borderRadius: "50%", background: "var(--accent, #5b7cff)", flex: "0 0 auto" }} /> : null}
                <strong style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", fontSize: 13.5 }}>{group.label}</strong>
                {grouped ? <span style={{ borderRadius: 10, padding: "2px 7px", fontSize: 11, background: "rgba(91,124,255,.12)", color: "var(--accent, #5b7cff)", whiteSpace: "nowrap" }}>{group.messages.length} 封</span> : null}
              </div>
              {showAccount ? <span style={{ paddingLeft: group.unreadCount ? 14 : 0, fontSize: 10.5, opacity: .47, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>最近到达 {accountName(accounts, group.latest.accountId)}</span> : null}
            </div>
            <div style={{ minWidth: 0, display: "flex", alignItems: "center", gap: 8 }}>
              <span style={{ fontSize: 13.5, fontWeight: group.unreadCount ? 700 : 500, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{group.latest.subject || "无主题"}</span>
              {group.latest.snippet ? <span style={{ fontSize: 12.5, opacity: .5, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>— {group.latest.snippet}</span> : null}
              {group.latest.hasAttachments ? <Paperclip size={14} style={{ opacity: .5, flex: "0 0 auto" }} /> : null}
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 7, fontSize: 12, opacity: .58, whiteSpace: "nowrap" }}>{formatTime(group.latest.receivedAt)}<ChevronRight size={14} /></div>
          </button>
        );
      })}
    </div>
  );
}

function Empty({ loading, label }: { loading: boolean; label: string }) {
  return (
    <div style={{ padding: 48, opacity: .55, textAlign: "center" }}>
      <Inbox size={28} />
      <p>{loading ? "正在读取最近 30 天邮件…" : label}</p>
    </div>
  );
}
