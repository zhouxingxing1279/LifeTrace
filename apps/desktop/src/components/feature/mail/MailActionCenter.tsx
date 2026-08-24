import { useCallback, useEffect, useMemo, useState } from "react";
import { ArrowLeft, Inbox, LoaderCircle, RefreshCw, Search } from "lucide-react";
import { mailApi, type MailAccount, type MailFolder, type MailMessage, type MailMessageSummary } from "@/src/services/mailApi";
import { confirmAction } from "@/src/ui/feedback/confirm";
import { MailAccountDialog } from "./MailAccountDialog";
import { MailComposerDialog } from "./MailComposerDialog";
import { MailMessageDetail } from "./MailMessageDetail";
import { MailMessageList } from "./MailMessageList";
import { MailSidebar } from "./MailSidebar";
import {
  groupBySender,
  sourceAccountId,
  sourceQuery,
  sourceSubtitle,
  sourceTitle,
  type MailCollectionSource,
  type MailListContext,
  type MailScreen,
} from "./mailModel";
import { actionButton, errorMessage, inputStyle, panelStyle, toast } from "./mailStyles";

type ComposerState = {
  initialAccountId?: string | null;
  replyMessage?: MailMessage | null;
};

const initialContext: MailListContext = { source: { kind: "unified" } };

export default function MailActionCenter() {
  const [accounts, setAccounts] = useState<MailAccount[]>([]);
  const [foldersByAccount, setFoldersByAccount] = useState<Record<string, MailFolder[]>>({});
  const [messages, setMessages] = useState<MailMessageSummary[]>([]);
  const [screen, setScreen] = useState<MailScreen>({ kind: "list", context: initialContext });
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(true);
  const [listLoading, setListLoading] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [accountDialog, setAccountDialog] = useState(false);
  const [composer, setComposer] = useState<ComposerState | null>(null);

  const activeContext = screen.kind === "list" ? screen.context : screen.back;
  const activeSource = activeContext.source;
  const senderGroups = useMemo(() => groupBySender(messages), [messages]);
  const activeSenderGroup = activeContext.senderKey
    ? senderGroups.find((group) => group.key === activeContext.senderKey) || null
    : null;

  const loadAccounts = useCallback(async () => {
    const nextAccounts = await mailApi.accounts.list();
    const folderEntries = await Promise.all(nextAccounts.map(async (account) => {
      try {
        return [account.id, await mailApi.accounts.folders(account.id)] as const;
      } catch {
        return [account.id, []] as const;
      }
    }));
    setAccounts(nextAccounts);
    setFoldersByAccount(Object.fromEntries(folderEntries));
    return nextAccounts;
  }, []);

  const loadMessages = useCallback(async (source: MailCollectionSource, q: string) => {
    setListLoading(true);
    try {
      const items: MailMessageSummary[] = [];
      let offset = 0;
      for (;;) {
        const page = await mailApi.messages.list({
          ...sourceQuery(source, q),
          limit: 500,
          offset,
        });
        items.push(...page.items);
        if (!page.hasMore || page.nextOffset <= offset) break;
        offset = page.nextOffset;
      }
      setMessages(items);
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
    if (!accounts.length) {
      setMessages([]);
      return;
    }
    const delay = search.trim() ? 180 : 0;
    const timer = window.setTimeout(() => {
      void loadMessages(activeSource, search).catch((error) => toast(errorMessage(error), "error"));
    }, delay);
    return () => window.clearTimeout(timer);
  }, [accounts.length, activeSource, search, loadMessages]);

  const selectSource = (source: MailCollectionSource) => {
    setSearch("");
    setScreen({ kind: "list", context: { source } });
  };

  const openSender = (senderKey: string) => {
    setScreen({ kind: "list", context: { source: activeSource, senderKey } });
  };

  const openMessage = (message: MailMessageSummary) => {
    setScreen({ kind: "detail", messageId: message.id, back: activeContext });
  };

  const sync = async () => {
    if (!accounts.length) return;
    const scopedAccountId = sourceAccountId(activeSource);
    const targetAccounts = scopedAccountId
      ? accounts.filter((account) => account.id === scopedAccountId)
      : accounts.filter((account) => account.status !== "disabled");
    if (!targetAccounts.length) return;

    setSyncing(true);
    try {
      let persisted = 0;
      for (const account of targetAccounts) {
        const result = await mailApi.accounts.sync(account.id);
        persisted += result.persisted;
      }
      await loadAccounts();
      await loadMessages(activeSource, search);
      toast(targetAccounts.length > 1
        ? `已同步 ${targetAccounts.length} 个邮箱，新增/更新 ${persisted} 封邮件`
        : `同步完成，新增/更新 ${persisted} 封邮件`);
    } catch (error) {
      toast(errorMessage(error), "error");
    } finally {
      setSyncing(false);
    }
  };

  const disconnect = async (account: MailAccount) => {
    const confirmed = await confirmAction({
      title: "断开邮箱",
      description: `断开 ${account.emailAddress} 后，云端 Worker 将停止同步并使已保存授权凭据失效。`,
      confirmLabel: "断开",
    });
    if (!confirmed) return;
    try {
      await mailApi.accounts.disconnect(account.id);
      await loadAccounts();
      if (sourceAccountId(activeSource) === account.id) {
        setSearch("");
        setScreen({ kind: "list", context: initialContext });
      } else {
        await loadMessages(activeSource, search);
      }
      toast("邮箱已断开");
    } catch (error) {
      toast(errorMessage(error), "error");
    }
  };

  const patchMessage = (id: string, patch: Partial<MailMessage>) => {
    setMessages((current) => current.map((item) => item.id === id
      ? { ...item, ...(typeof patch.isRead === "boolean" ? { isRead: patch.isRead } : {}) }
      : item));
  };

  const archiveMessage = (id: string) => {
    setMessages((current) => current.filter((item) => item.id !== id));
    if (screen.kind === "detail") setScreen({ kind: "list", context: screen.back });
  };

  const initialComposeAccountId = sourceAccountId(activeSource);

  return (
    <div
      className="mail-action-center"
      style={{
        display: "grid",
        gridTemplateColumns: "244px minmax(0, 1fr)",
        minHeight: 0,
        height: "100%",
        overflow: "hidden",
        background: "transparent",
      }}
    >
      <MailSidebar
        accounts={accounts}
        foldersByAccount={foldersByAccount}
        activeSource={activeSource}
        onSelectSource={selectSource}
        onCompose={() => setComposer({ initialAccountId: initialComposeAccountId })}
        onAddAccount={() => setAccountDialog(true)}
        onDisconnect={(account) => void disconnect(account)}
      />

      <main style={{
        minWidth: 0,
        minHeight: 0,
        padding: 18,
        overflowX: "hidden",
        overflowY: screen.kind === "detail" ? "auto" : "hidden",
        overscrollBehavior: "contain",
        scrollbarGutter: screen.kind === "detail" ? "stable" : "auto",
      }}>
        {loading ? <div style={{ minHeight: 360, display: "grid", placeItems: "center" }}><span style={{ display: "inline-flex", alignItems: "center", gap: 9 }}><LoaderCircle size={18} />正在加载邮件…</span></div> : accounts.length === 0 ? (
          <section style={{ ...panelStyle, minHeight: 360, display: "grid", placeItems: "center", textAlign: "center", padding: 40 }}>
            <div><Inbox size={34} style={{ opacity: .42 }} /><h2>连接第一个邮箱</h2><p style={{ opacity: .62 }}>连接后会进入统一收件箱，并同步最近 30 天邮件。</p><button type="button" className="hx-btn primary" onClick={() => setAccountDialog(true)}>添加邮箱</button></div>
          </section>
        ) : screen.kind === "detail" ? (
          <MailMessageDetail
            messageId={screen.messageId}
            accounts={accounts}
            backLabel={screen.back.senderKey ? "返回发件人邮件列表" : "返回邮件列表"}
            onBack={() => setScreen({ kind: "list", context: screen.back })}
            onReply={(message) => setComposer({ initialAccountId: message.accountId, replyMessage: message })}
            onMessagePatch={patchMessage}
            onArchived={archiveMessage}
          />
        ) : (
          <div style={{ display: "grid", gridTemplateRows: "auto minmax(0, 1fr)", gap: 14, height: "100%", minHeight: 0 }}>
            <header style={{ display: "flex", alignItems: "flex-start", justifyContent: "space-between", gap: 14 }}>
              <div style={{ minWidth: 0 }}>
                {activeContext.senderKey ? <button type="button" style={{ ...actionButton, marginBottom: 10 }} onClick={() => setScreen({ kind: "list", context: { source: activeSource } })}><ArrowLeft size={15} />返回 {sourceTitle(activeSource, accounts)}</button> : null}
                <h1 style={{ margin: 0, fontSize: 23, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{activeSenderGroup?.label || sourceTitle(activeSource, accounts)}</h1>
                <p style={{ margin: "5px 0 0", fontSize: 13, opacity: .58 }}>
                  {activeSenderGroup
                    ? `${activeSenderGroup.email} · ${activeSenderGroup.messages.length} 封邮件 · ${sourceSubtitle(activeSource, accounts)}`
                    : sourceSubtitle(activeSource, accounts)}
                </p>
              </div>
              <button type="button" style={actionButton} disabled={syncing} onClick={() => void sync()}><RefreshCw size={15} />{syncing ? "同步中…" : sourceAccountId(activeSource) ? "同步邮箱" : "同步全部"}</button>
            </header>

            <section style={{
              minHeight: 0,
              display: "flex",
              flexDirection: "column",
              overflow: "hidden",
              borderTop: "1px solid var(--ui-border)",
              borderBottom: "1px solid var(--ui-border)",
              background: "transparent",
            }}>
              <div style={{ padding: 11, display: "flex", gap: 10, alignItems: "center", borderBottom: "1px solid var(--ui-border)", flex: "0 0 auto" }}>
                <div style={{ position: "relative", flex: 1 }}>
                  <Search size={15} style={{ position: "absolute", left: 10, top: 12, opacity: .45 }} />
                  <input aria-label="搜索邮件" style={{ ...inputStyle, paddingLeft: 31 }} value={search} onChange={(event) => setSearch(event.target.value)} placeholder="搜索发件人、主题或摘要" />
                </div>
                <span style={{ fontSize: 11.5, opacity: .48, whiteSpace: "nowrap" }}>{messages.length} 封</span>
                {listLoading ? <span style={{ display: "inline-flex", alignItems: "center", gap: 5, fontSize: 11.5, opacity: .55 }}><LoaderCircle size={13} />加载中</span> : null}
              </div>

              <div style={{ minHeight: 0, overflowY: "auto", overscrollBehavior: "contain", scrollbarGutter: "stable" }}>
                <MailMessageList
                  messages={messages}
                  source={activeSource}
                  senderKey={activeContext.senderKey}
                  accounts={accounts}
                  loading={listLoading}
                  onOpenMessage={openMessage}
                  onOpenSender={openSender}
                />
              </div>
            </section>
          </div>
        )}
      </main>

      {accountDialog ? <MailAccountDialog
        onClose={() => setAccountDialog(false)}
        onCreated={(account) => {
          setAccounts((current) => [...current.filter((item) => item.id !== account.id), account]);
          void loadAccounts().then(() => loadMessages({ kind: "unified" }, search)).catch((error) => toast(errorMessage(error), "error"));
          setScreen({ kind: "list", context: initialContext });
        }}
      /> : null}

      {composer ? <MailComposerDialog
        accounts={accounts}
        initialAccountId={composer.initialAccountId}
        replyMessage={composer.replyMessage}
        onClose={() => setComposer(null)}
      /> : null}
    </div>
  );
}
