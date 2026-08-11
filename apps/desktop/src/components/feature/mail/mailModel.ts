import type { MailAccount, MailFolder, MailMessage, MailMessageSummary } from "@/src/services/mailApi";

export type MailCollectionSource =
  | { kind: "unified" }
  | { kind: "unread" }
  | { kind: "account"; accountId: string }
  | { kind: "folder"; accountId: string; folderId: string; role: string; name: string };

export type MailListContext = {
  source: MailCollectionSource;
  senderKey?: string;
};

export type MailScreen =
  | { kind: "list"; context: MailListContext }
  | { kind: "detail"; messageId: string; back: MailListContext };

export type MailboxIdentity = { email: string; name?: string };

export type SenderGroup = {
  key: string;
  email: string;
  label: string;
  messages: MailMessageSummary[];
  unreadCount: number;
  latest: MailMessageSummary;
};

export function firstMailbox(value: unknown): MailboxIdentity | null {
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

export function collectAddresses(value: unknown): string[] {
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

export function senderIdentity(message: Pick<MailMessageSummary, "fromJson"> | Pick<MailMessage, "fromJson">): MailboxIdentity {
  return firstMailbox(message.fromJson) || { email: "未知发件人" };
}

export function senderLabel(message: Pick<MailMessageSummary, "fromJson"> | Pick<MailMessage, "fromJson">) {
  const sender = senderIdentity(message);
  return sender.name ? `${sender.name} <${sender.email}>` : sender.email;
}

export function groupBySender(messages: MailMessageSummary[]): SenderGroup[] {
  const map = new Map<string, SenderGroup>();
  for (const message of messages) {
    const identity = senderIdentity(message);
    const hasAddress = identity.email.includes("@");
    const key = hasAddress ? identity.email.toLowerCase() : `unknown:${message.id}`;
    const existing = map.get(key);
    if (existing) {
      existing.messages.push(message);
      if (!message.isRead) existing.unreadCount += 1;
      if (new Date(message.receivedAt).getTime() > new Date(existing.latest.receivedAt).getTime()) {
        existing.latest = message;
      }
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
  return [...map.values()]
    .map((group) => ({
      ...group,
      messages: [...group.messages].sort((left, right) =>
        new Date(right.receivedAt).getTime() - new Date(left.receivedAt).getTime()),
    }))
    .sort((left, right) =>
      new Date(right.latest.receivedAt).getTime() - new Date(left.latest.receivedAt).getTime());
}

export function sourceQuery(source: MailCollectionSource, search: string) {
  const q = search.trim() || undefined;
  if (source.kind === "unified") return { q };
  if (source.kind === "unread") return { q, unreadOnly: true };
  if (source.kind === "account") return { q, accountId: source.accountId };
  return { q, accountId: source.accountId, folderId: source.folderId };
}

export function sourceAccountId(source: MailCollectionSource): string | null {
  return source.kind === "account" || source.kind === "folder" ? source.accountId : null;
}

export function shouldAggregateBySender(source: MailCollectionSource) {
  return source.kind !== "folder" || source.role === "inbox";
}

export function providerLabel(provider: string) {
  return { qq: "QQ 邮箱", "163": "163 邮箱", "126": "126 邮箱", yeah: "yeah.net", generic: "IMAP/SMTP" }[provider] || provider;
}

export function accountLabel(account: MailAccount) {
  return account.displayName?.trim() || account.emailAddress;
}

export function sourceTitle(
  source: MailCollectionSource,
  accounts: MailAccount[],
) {
  if (source.kind === "unified") return "统一收件箱";
  if (source.kind === "unread") return "未读邮件";
  const account = accounts.find((item) => item.id === source.accountId);
  if (source.kind === "account") return account ? accountLabel(account) : "收件箱";
  return source.name || folderRoleLabel(source.role);
}

export function sourceSubtitle(source: MailCollectionSource, accounts: MailAccount[]) {
  if (source.kind === "unified") return `${accounts.length} 个邮箱 · 最近 30 天`;
  if (source.kind === "unread") return `${accounts.length} 个邮箱中的未读邮件 · 最近 30 天`;
  const account = accounts.find((item) => item.id === source.accountId);
  if (source.kind === "account") return `${account?.emailAddress || "邮箱"} · 收件箱 · 最近 30 天`;
  return `${account?.emailAddress || "邮箱"} · ${folderRoleLabel(source.role)} · 最近 30 天`;
}

export function folderRoleLabel(role: string) {
  return {
    inbox: "收件箱",
    sent: "已发送",
    drafts: "草稿",
    trash: "垃圾箱",
    spam: "垃圾邮件",
    archive: "归档",
    other: "其他文件夹",
  }[role] || role;
}

export function visibleFolders(folders: MailFolder[]) {
  const order: Record<string, number> = { sent: 1, archive: 2, drafts: 3, trash: 4, spam: 5, other: 6 };
  return folders
    .filter((folder) => folder.normalizedRole !== "inbox")
    .sort((left, right) => {
      const roleDiff = (order[left.normalizedRole] ?? 99) - (order[right.normalizedRole] ?? 99);
      return roleDiff || left.remoteName.localeCompare(right.remoteName);
    });
}
