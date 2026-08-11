import { useState } from "react";
import {
  Archive,
  ChevronDown,
  ChevronRight,
  FileText,
  Inbox,
  MailOpen,
  Plus,
  Send,
  Settings2,
  ShieldAlert,
  Trash2,
} from "lucide-react";
import type { MailAccount, MailFolder } from "@/src/services/mailApi";
import {
  accountLabel,
  folderRoleLabel,
  type MailCollectionSource,
  providerLabel,
  visibleFolders,
} from "./mailModel";
import { actionButton } from "./mailStyles";

function roleIcon(role: string) {
  if (role === "sent") return <Send size={14} />;
  if (role === "archive") return <Archive size={14} />;
  if (role === "drafts") return <FileText size={14} />;
  if (role === "trash") return <Trash2 size={14} />;
  if (role === "spam") return <ShieldAlert size={14} />;
  return <MailOpen size={14} />;
}

function sourceEquals(left: MailCollectionSource, right: MailCollectionSource) {
  if (left.kind !== right.kind) return false;
  if (left.kind === "unified" || left.kind === "unread") return true;
  if (left.kind === "account" && right.kind === "account") return left.accountId === right.accountId;
  if (left.kind === "folder" && right.kind === "folder") return left.folderId === right.folderId;
  return false;
}

function NavButton({ active, children, onClick, disabled = false }: { active: boolean; children: React.ReactNode; onClick: () => void; disabled?: boolean }) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      style={{
        width: "100%",
        border: 0,
        borderRadius: 8,
        padding: "8px 10px",
        display: "flex",
        alignItems: "center",
        gap: 8,
        textAlign: "left",
        color: "inherit",
        background: active ? "rgba(91,124,255,.12)" : "transparent",
        fontWeight: active ? 700 : 500,
        cursor: disabled ? "not-allowed" : "pointer",
        opacity: disabled ? .42 : 1,
      }}
    >{children}</button>
  );
}

export function MailSidebar({
  accounts,
  foldersByAccount,
  activeSource,
  onSelectSource,
  onCompose,
  onAddAccount,
  onDisconnect,
}: {
  accounts: MailAccount[];
  foldersByAccount: Record<string, MailFolder[]>;
  activeSource: MailCollectionSource;
  onSelectSource: (source: MailCollectionSource) => void;
  onCompose: () => void;
  onAddAccount: () => void;
  onDisconnect: (account: MailAccount) => void;
}) {
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});

  return (
    <aside style={{ width: "100%", minWidth: 0, display: "flex", flexDirection: "column", gap: 14, padding: "14px 10px", borderRight: "1px solid var(--line, rgba(128,128,128,.16))", background: "var(--panel, rgba(255,255,255,.45))" }}>
      <button type="button" className="hx-btn primary" onClick={onCompose} disabled={!accounts.length} style={{ width: "100%", minHeight: 38 }}>
        <Plus size={16} />写邮件
      </button>

      <nav style={{ display: "grid", gap: 3 }}>
        <NavButton active={sourceEquals(activeSource, { kind: "unified" })} onClick={() => onSelectSource({ kind: "unified" })}>
          <Inbox size={16} />统一收件箱
        </NavButton>
        <NavButton active={sourceEquals(activeSource, { kind: "unread" })} onClick={() => onSelectSource({ kind: "unread" })}>
          <MailOpen size={16} />未读邮件
        </NavButton>
      </nav>

      <div style={{ height: 1, background: "var(--line, rgba(128,128,128,.14))" }} />

      <div style={{ minHeight: 0, overflow: "auto" }}>
        <div style={{ padding: "0 8px 7px", fontSize: 11.5, fontWeight: 700, opacity: .48, letterSpacing: ".08em" }}>邮箱账户</div>
        <div style={{ display: "grid", gap: 4 }}>
          {accounts.map((account) => {
            const isExpanded = expanded[account.id] ?? true;
            const folders = visibleFolders(foldersByAccount[account.id] || []);
            const accountSource: MailCollectionSource = { kind: "account", accountId: account.id };
            return (
              <section key={account.id} style={{ borderRadius: 9, border: "1px solid var(--line, rgba(128,128,128,.1))", overflow: "hidden" }}>
                <div style={{ display: "flex", alignItems: "center", gap: 4, padding: "5px 5px 3px 7px" }}>
                  <button
                    type="button"
                    onClick={() => setExpanded((current) => ({ ...current, [account.id]: !isExpanded }))}
                    style={{ border: 0, background: "transparent", color: "inherit", padding: 3, cursor: "pointer", display: "grid", placeItems: "center" }}
                    aria-label={isExpanded ? "收起邮箱" : "展开邮箱"}
                  >
                    {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                  </button>
                  <div style={{ minWidth: 0, flex: 1 }}>
                    <div style={{ fontSize: 12.5, fontWeight: 700, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{accountLabel(account)}</div>
                    <div style={{ fontSize: 10.5, opacity: .5, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{providerLabel(account.provider)} · {account.status === "active" ? "已连接" : account.status === "degraded" ? "部分可用" : "待验证"}</div>
                  </div>
                  <button
                    type="button"
                    title="断开邮箱"
                    onClick={() => onDisconnect(account)}
                    style={{ border: 0, background: "transparent", color: "inherit", opacity: .45, padding: 4, cursor: "pointer", display: "grid", placeItems: "center" }}
                  ><Settings2 size={13} /></button>
                </div>

                {isExpanded ? <div style={{ padding: "0 5px 6px 20px", display: "grid", gap: 2 }}>
                  <NavButton active={sourceEquals(activeSource, accountSource)} onClick={() => onSelectSource(accountSource)}>
                    <Inbox size={14} />收件箱
                  </NavButton>
                  {folders.map((folder) => {
                    const source: MailCollectionSource = {
                      kind: "folder",
                      accountId: account.id,
                      folderId: folder.id,
                      role: folder.normalizedRole,
                      name: folderRoleLabel(folder.normalizedRole) === "其他文件夹" ? folder.remoteName : folderRoleLabel(folder.normalizedRole),
                    };
                    return <NavButton key={folder.id} active={sourceEquals(activeSource, source)} disabled={!folder.syncEnabled} onClick={() => onSelectSource(source)}>
                      {roleIcon(folder.normalizedRole)}
                      <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{source.name}</span>
                      {!folder.syncEnabled ? <span style={{ marginLeft: "auto", fontSize: 10 }}>未同步</span> : null}
                    </NavButton>;
                  })}
                </div> : null}
              </section>
            );
          })}
        </div>
      </div>

      <button type="button" style={{ ...actionButton, width: "100%", marginTop: "auto" }} onClick={onAddAccount}><Plus size={15} />添加邮箱</button>
    </aside>
  );
}
