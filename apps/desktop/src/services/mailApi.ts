import { cloudAuthClient } from "@/src/services/cloudAuth";

export type MailProvider = "qq" | "163" | "126" | "yeah" | "generic";
export type MailSecurity = "tls" | "starttls";

export type MailAccount = {
  id: string;
  userId: string;
  provider: MailProvider | string;
  emailAddress: string;
  displayName?: string | null;
  imapHost: string;
  imapPort: number;
  imapSecurity: MailSecurity | string;
  smtpHost: string;
  smtpPort: number;
  smtpSecurity: MailSecurity | string;
  username: string;
  status: "validating" | "active" | "degraded" | "disabled" | string;
  idleSupported: boolean;
  lastValidatedAt?: string | null;
  lastSyncAt?: string | null;
  lastErrorCode?: string | null;
  createdAt: string;
  updatedAt: string;
};

export type MailAccountInput = {
  provider: MailProvider;
  emailAddress: string;
  displayName?: string;
  username?: string;
  authorizationCode: string;
  imapHost?: string;
  imapPort?: number;
  imapSecurity?: MailSecurity;
  smtpHost?: string;
  smtpPort?: number;
  smtpSecurity?: MailSecurity;
};

export type MailFolder = {
  id: string;
  accountId: string;
  remoteName: string;
  normalizedRole: "inbox" | "sent" | "drafts" | "trash" | "spam" | "archive" | "other" | string;
  uidvalidity?: number | null;
  uidnext?: number | null;
  lastSeenUid: number;
  lastSyncAt?: string | null;
  syncEnabled: boolean;
};

export type MailThread = {
  id: string;
  accountId: string;
  normalizedSubject: string;
  latestMessageAt?: string | null;
  messageCount: number;
  unreadCount: number;
  participantSummary?: string | null;
  snippet?: string | null;
};

export type MailMessage = {
  id: string;
  accountId: string;
  folderId: string;
  threadId: string;
  remoteUid: number;
  uidvalidity: number;
  messageId?: string | null;
  inReplyTo?: string | null;
  subject: string;
  fromJson: unknown;
  toJson: unknown;
  ccJson: unknown;
  replyToJson: unknown;
  sentAt?: string | null;
  receivedAt: string;
  flagsJson: unknown;
  isRead: boolean;
  isArchived: boolean;
  sizeBytes?: number | null;
  snippet?: string | null;
  bodyText?: string | null;
  bodyHtmlSanitized?: string | null;
  hasAttachments: boolean;
};

export type MailAttachment = {
  id: string;
  messageId: string;
  partId: string;
  filename?: string | null;
  mimeType?: string | null;
  sizeBytes: number;
  contentId?: string | null;
  disposition?: string | null;
  checksum?: string | null;
  storageRef?: string | null;
  downloadState: string;
};

export type ConnectionTestResult = {
  imapOk: boolean;
  smtpOk: boolean;
  idleSupported: boolean;
  folders: string[];
};

export type SendMailInput = {
  to: string[];
  cc?: string[];
  bcc?: string[];
  subject: string;
  bodyText: string;
  inReplyToMessageId?: string | null;
  idempotencyKey: string;
};

type ErrorPayload = { message?: string; code?: string; error?: string };

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const response = await cloudAuthClient.request(path, init);
  const raw = await response.text();
  let payload: unknown = null;
  if (raw) {
    try {
      payload = JSON.parse(raw);
    } catch {
      payload = raw;
    }
  }
  if (!response.ok) {
    const error = payload as ErrorPayload | string | null;
    const message = typeof error === "string" ? error : error?.message || error?.error || error?.code;
    throw new Error(message || `邮件服务请求失败（${response.status}）`);
  }
  return payload as T;
}

function json(method: string, body?: unknown): RequestInit {
  return {
    method,
    headers: { "content-type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  };
}

function query(path: string, values: Record<string, string | number | boolean | null | undefined>) {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(values)) {
    if (value !== undefined && value !== null && value !== "") params.set(key, String(value));
  }
  const suffix = params.toString();
  return suffix ? `${path}?${suffix}` : path;
}

export const mailApi = {
  accounts: {
    list: async () => (await request<{ items: MailAccount[] }>("/api/v1/mail/accounts")).items,
    create: (input: MailAccountInput) => request<MailAccount>("/api/v1/mail/accounts", json("POST", input)),
    test: (id: string) => request<ConnectionTestResult>(`/api/v1/mail/accounts/${encodeURIComponent(id)}/test`, json("POST", {})),
    sync: (id: string) => request<{ ok: true; persisted: number }>(`/api/v1/mail/accounts/${encodeURIComponent(id)}/sync`, json("POST", {})),
    disconnect: (id: string) => request<{ ok: true }>(`/api/v1/mail/accounts/${encodeURIComponent(id)}`, { method: "DELETE" }),
    folders: async (id: string) => (await request<{ items: MailFolder[] }>(`/api/v1/mail/accounts/${encodeURIComponent(id)}/folders`)).items,
    send: (id: string, input: SendMailInput) => request<{ ok: true; messageId: string }>(`/api/v1/mail/accounts/${encodeURIComponent(id)}/send`, json("POST", input)),
  },
  threads: {
    list: async (options: { accountId?: string; folderId?: string; q?: string; unreadOnly?: boolean; limit?: number; offset?: number } = {}) =>
      (await request<{ items: MailThread[] }>(query("/api/v1/mail/threads", options))).items,
    messages: async (threadId: string) => (await request<{ items: MailMessage[] }>(`/api/v1/mail/threads/${encodeURIComponent(threadId)}/messages`)).items,
  },
  messages: {
    get: (id: string) => request<MailMessage>(`/api/v1/mail/messages/${encodeURIComponent(id)}`),
    attachments: async (id: string) => (await request<{ items: MailAttachment[] }>(`/api/v1/mail/messages/${encodeURIComponent(id)}/attachments`)).items,
    setRead: (id: string, read: boolean) => request<{ ok: true; read: boolean }>(`/api/v1/mail/messages/${encodeURIComponent(id)}/read`, json("POST", { read })),
    archive: (id: string) => request<{ ok: true }>(`/api/v1/mail/messages/${encodeURIComponent(id)}/archive`, json("POST", {})),
  },
};
