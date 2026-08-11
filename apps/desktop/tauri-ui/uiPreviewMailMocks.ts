type MockMailAccount = {
  id: string;
  userId: string;
  provider: string;
  emailAddress: string;
  displayName: string;
  imapHost: string;
  imapPort: number;
  imapSecurity: string;
  smtpHost: string;
  smtpPort: number;
  smtpSecurity: string;
  username: string;
  status: string;
  idleSupported: boolean;
  lastValidatedAt: string;
  lastSyncAt: string;
  createdAt: string;
  updatedAt: string;
};

type MockMailMessage = {
  id: string;
  accountId: string;
  folderId: string;
  threadId: string;
  remoteUid: number;
  uidvalidity: number;
  messageId: string;
  subject: string;
  fromJson: Array<{ name: string; address: string }>;
  toJson: Array<{ address: string }>;
  ccJson: Array<{ address: string }>;
  replyToJson: Array<{ address: string }>;
  sentAt: string;
  receivedAt: string;
  flagsJson: string[];
  isRead: boolean;
  isArchived: boolean;
  sizeBytes: number;
  snippet: string;
  bodyText: string;
  bodyHtmlSanitized: string;
  hasAttachments: boolean;
};

type MockAttachment = {
  id: string;
  messageId: string;
  partId: string;
  filename: string;
  mimeType: string;
  sizeBytes: number;
  contentId: null;
  disposition: string;
  checksum: string;
  storageRef: string;
  downloadState: string;
};

const now = new Date();
const iso = (daysAgo = 0, hour = 9, minute = 0) => {
  const value = new Date(now);
  value.setDate(value.getDate() - daysAgo);
  value.setHours(hour, minute, 0, 0);
  return value.toISOString();
};

const account: MockMailAccount = {
  id: "mail-1",
  userId: "preview-user",
  provider: "qq",
  emailAddress: "preview@example.com",
  displayName: "工作邮箱",
  imapHost: "imap.qq.com",
  imapPort: 993,
  imapSecurity: "tls",
  smtpHost: "smtp.qq.com",
  smtpPort: 465,
  smtpSecurity: "tls",
  username: "preview@example.com",
  status: "active",
  idleSupported: true,
  lastValidatedAt: iso(1),
  lastSyncAt: iso(0, 9, 30),
  createdAt: iso(90),
  updatedAt: iso(),
};

let messages: MockMailMessage[] = [
  {
    id: "mail-msg-1",
    accountId: "mail-1",
    folderId: "inbox",
    threadId: "thread-1",
    remoteUid: 101,
    uidvalidity: 1,
    messageId: "<preview-101@github.com>",
    subject: "LifeTrace UI Review",
    fromJson: [{ name: "GitHub", address: "notifications@github.com" }],
    toJson: [{ address: "preview@example.com" }],
    ccJson: [],
    replyToJson: [{ address: "notifications@github.com" }],
    sentAt: iso(0, 10, 5),
    receivedAt: iso(0, 10, 5),
    flagsJson: [],
    isRead: false,
    isArchived: false,
    sizeBytes: 2048,
    snippet: "The latest UI preview build is ready for review.",
    bodyText: "The latest UI preview build is ready for review.\n\nOpen the preview and inspect the desktop layout at 1460 × 850.",
    bodyHtmlSanitized: "<p>The latest UI preview build is ready for review.</p><p>Open the preview and inspect the desktop layout at <strong>1460 × 850</strong>.</p>",
    hasAttachments: false,
  },
  {
    id: "mail-msg-2",
    accountId: "mail-1",
    folderId: "inbox",
    threadId: "thread-2",
    remoteUid: 102,
    uidvalidity: 1,
    messageId: "<preview-102@conference.example.com>",
    subject: "会议资料确认",
    fromJson: [{ name: "Conference", address: "conference@example.com" }],
    toJson: [{ address: "preview@example.com" }],
    ccJson: [],
    replyToJson: [{ address: "conference@example.com" }],
    sentAt: iso(1, 16, 20),
    receivedAt: iso(1, 16, 20),
    flagsJson: ["\\Seen"],
    isRead: true,
    isArchived: false,
    sizeBytes: 18432,
    snippet: "Please confirm the latest poster materials.",
    bodyText: "请确认最新会议资料。附件仅用于 UI Preview 中检查附件列表样式。",
    bodyHtmlSanitized: "<p>请确认最新会议资料。</p><p>附件仅用于 UI Preview 中检查附件列表样式。</p>",
    hasAttachments: true,
  },
];

const attachments: MockAttachment[] = [
  {
    id: "mail-att-1",
    messageId: "mail-msg-2",
    partId: "2",
    filename: "poster-review.pdf",
    mimeType: "application/pdf",
    sizeBytes: 286_720,
    contentId: null,
    disposition: "attachment",
    checksum: "preview-checksum",
    storageRef: "preview/mail-att-1",
    downloadState: "ready",
  },
];

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json; charset=utf-8" },
  });
}

function requestUrl(input: RequestInfo | URL) {
  const raw = input instanceof Request ? input.url : input instanceof URL ? input.toString() : input;
  return new URL(raw, window.location.href);
}

function summary(message: MockMailMessage) {
  const {
    remoteUid: _remoteUid,
    uidvalidity: _uidvalidity,
    messageId: _messageId,
    ccJson: _ccJson,
    replyToJson: _replyToJson,
    flagsJson: _flagsJson,
    sizeBytes: _sizeBytes,
    bodyText: _bodyText,
    bodyHtmlSanitized: _bodyHtmlSanitized,
    ...rest
  } = message;
  return rest;
}

export function installUiPreviewMailMocks() {
  const previousFetch = window.fetch.bind(window);

  window.fetch = (async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = requestUrl(input);
    const pathname = url.pathname;
    const method = (init?.method || (input instanceof Request ? input.method : "GET") || "GET").toUpperCase();

    if (!pathname.startsWith("/api/v1/mail/")) {
      return previousFetch(input, init);
    }

    if (pathname === "/api/v1/mail/accounts") {
      if (method === "GET") return jsonResponse({ items: [account] });
      if (method === "POST") return jsonResponse(account, 201);
    }

    if (pathname === `/api/v1/mail/accounts/${account.id}/folders`) {
      return jsonResponse({
        items: [{
          id: "inbox",
          accountId: account.id,
          remoteName: "INBOX",
          normalizedRole: "inbox",
          lastSeenUid: 102,
          lastSyncAt: account.lastSyncAt,
          syncEnabled: true,
        }],
      });
    }

    if (pathname === `/api/v1/mail/accounts/${account.id}/test`) {
      return jsonResponse({ imapOk: true, smtpOk: true, idleSupported: true, folders: ["INBOX"] });
    }

    if (pathname === `/api/v1/mail/accounts/${account.id}/sync`) {
      account.lastSyncAt = new Date().toISOString();
      return jsonResponse({ ok: true, persisted: messages.length });
    }

    if (pathname === `/api/v1/mail/accounts/${account.id}/send`) {
      return jsonResponse({ ok: true, messageId: `preview-sent-${Date.now()}` });
    }

    if (pathname === `/api/v1/mail/accounts/${account.id}` && method === "DELETE") {
      return jsonResponse({ ok: true });
    }

    if (pathname === "/api/v1/mail/threads") {
      const items = messages
        .filter((message) => !message.isArchived)
        .map((message) => ({
          id: message.threadId,
          accountId: message.accountId,
          normalizedSubject: message.subject,
          latestMessageAt: message.receivedAt,
          messageCount: 1,
          unreadCount: message.isRead ? 0 : 1,
          participantSummary: message.fromJson[0]?.name || message.fromJson[0]?.address || "Preview",
          snippet: message.snippet,
        }));
      return jsonResponse({ items });
    }

    const threadMatch = pathname.match(/^\/api\/v1\/mail\/threads\/([^/]+)\/messages$/);
    if (threadMatch) {
      const threadId = decodeURIComponent(threadMatch[1]);
      return jsonResponse({ items: messages.filter((message) => message.threadId === threadId) });
    }

    if (pathname === "/api/v1/mail/messages") {
      const accountId = url.searchParams.get("accountId");
      const query = (url.searchParams.get("q") || "").trim().toLowerCase();
      const unreadOnly = url.searchParams.get("unreadOnly") === "true";
      const offset = Math.max(Number(url.searchParams.get("offset") || "0") || 0, 0);
      const limit = Math.max(Number(url.searchParams.get("limit") || "500") || 500, 1);

      const filtered = messages.filter((message) => {
        if (message.isArchived) return false;
        if (accountId && message.accountId !== accountId) return false;
        if (unreadOnly && message.isRead) return false;
        if (!query) return true;
        const haystack = [
          message.subject,
          message.snippet,
          ...message.fromJson.flatMap((item) => [item.name, item.address]),
        ].join(" ").toLowerCase();
        return haystack.includes(query);
      });
      const page = filtered.slice(offset, offset + limit).map(summary);
      const nextOffset = offset + page.length;
      return jsonResponse({ items: page, hasMore: nextOffset < filtered.length, nextOffset });
    }

    const attachmentListMatch = pathname.match(/^\/api\/v1\/mail\/messages\/([^/]+)\/attachments$/);
    if (attachmentListMatch) {
      const messageId = decodeURIComponent(attachmentListMatch[1]);
      return jsonResponse({ items: attachments.filter((attachment) => attachment.messageId === messageId) });
    }

    const readMatch = pathname.match(/^\/api\/v1\/mail\/messages\/([^/]+)\/read$/);
    if (readMatch && method === "POST") {
      const messageId = decodeURIComponent(readMatch[1]);
      const target = messages.find((message) => message.id === messageId);
      if (!target) return jsonResponse({ message: "邮件不存在" }, 404);
      let read = true;
      try {
        const body = typeof init?.body === "string" ? JSON.parse(init.body) as { read?: boolean } : null;
        read = body?.read !== false;
      } catch {
        // Keep the default preview state.
      }
      target.isRead = read;
      return jsonResponse({ ok: true, read });
    }

    const archiveMatch = pathname.match(/^\/api\/v1\/mail\/messages\/([^/]+)\/archive$/);
    if (archiveMatch && method === "POST") {
      const messageId = decodeURIComponent(archiveMatch[1]);
      const target = messages.find((message) => message.id === messageId);
      if (!target) return jsonResponse({ message: "邮件不存在" }, 404);
      target.isArchived = true;
      return jsonResponse({ ok: true });
    }

    const messageMatch = pathname.match(/^\/api\/v1\/mail\/messages\/([^/]+)$/);
    if (messageMatch) {
      const messageId = decodeURIComponent(messageMatch[1]);
      const message = messages.find((item) => item.id === messageId);
      return message ? jsonResponse(message) : jsonResponse({ message: "邮件不存在" }, 404);
    }

    const downloadMatch = pathname.match(/^\/api\/v1\/mail\/attachments\/([^/]+)\/content$/);
    if (downloadMatch) {
      const attachmentId = decodeURIComponent(downloadMatch[1]);
      const attachment = attachments.find((item) => item.id === attachmentId);
      if (!attachment) return jsonResponse({ message: "附件不存在" }, 404);
      return new Response(new Blob(["LifeTrace UI Preview attachment"], { type: attachment.mimeType }), {
        status: 200,
        headers: {
          "content-type": attachment.mimeType,
          "content-disposition": `attachment; filename=\"${attachment.filename}\"`,
        },
      });
    }

    return jsonResponse({ ok: true, items: [] });
  }) as typeof window.fetch;
}
