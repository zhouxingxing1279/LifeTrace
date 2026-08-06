export const APP_VERSION = "0.2.1";
export const APP_ID = "lifetrace-web";
export const PROTOCOL_VERSION = 1;
export const SCHEMA_VERSION = 1;

export const ENTITY_TYPES = [
  "finance.account",
  "finance.category",
  "finance.transaction",
  "note.folder",
  "note.note",
  "note.tag",
  "note.tag_relation",
  "english.article",
  "english.highlight",
  "english.learning_record",
  "english.vocabulary",
  "file.metadata",
  "user.preference",
] as const;

export type EntityType = (typeof ENTITY_TYPES)[number];
export type JsonEntity = Record<string, unknown> & { meta: EntityMeta };
export type FetchLike = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;

export interface EntityMeta {
  id: string;
  userId: string;
  createdAt: string;
  updatedAt: string;
  localVersion: number;
  serverVersion?: string | null;
  modifiedByDevice?: string | null;
  deletedAt?: string | null;
}

export interface AuthUser {
  id: string;
  email: string;
  displayName?: string | null;
}

export interface AuthSession {
  id: string;
  appId: string;
  deviceId: string;
  scopes: string[];
  idleExpiresAt: string;
  absoluteExpiresAt: string;
  publicDevice: boolean;
}

export interface WebSession {
  user: AuthUser;
  session: AuthSession;
  csrfToken: string;
}

export interface DeviceInstallation {
  id: string;
  externalDeviceId: string;
  deviceGroupId?: string | null;
  deviceName: string;
  appId: string;
  platform: string;
  status: string;
  clientVersion?: string | null;
  firstSeenAt: string;
  lastSeenAt: string;
  lastLoginAt?: string | null;
  lastSyncAt?: string | null;
  revokedAt?: string | null;
  current: boolean;
}

export interface ManagedSession {
  id: string;
  appId: string;
  deviceId: string;
  sessionType: string;
  status: string;
  scopes: string[];
  publicDevice: boolean;
  createdAt: string;
  lastSeenAt: string;
  idleExpiresAt: string;
  absoluteExpiresAt: string;
  revokedAt?: string | null;
  current: boolean;
}

export interface CloudConflict {
  entityType: EntityType;
  entityId: string;
  reason: string;
  occurredAt: string;
}

export interface CloudState {
  cursor: string | null;
  entities: Partial<Record<EntityType, Record<string, JsonEntity>>>;
  conflicts: CloudConflict[];
  lastLoadedAt: string | null;
}

export interface SyncChange {
  changeId: string;
  entityType: EntityType;
  entityId: string;
  operation: "upsert" | "delete";
  baseServerVersion: string;
  entitySchemaVersion: number;
  clientModifiedAt: string;
  payload: JsonEntity | null;
  atomicGroupId: null;
  dependencies: Array<{ entityType: string; entityId: string }>;
}

export interface SearchHit {
  id: string;
  entityType: EntityType;
  title: string;
  subtitle: string;
  updatedAt: string;
  route: string;
}

export const REQUESTED_SCOPES = [
  "sync:read",
  "sync:write",
  "finance:read",
  "finance:write",
  "notes:read",
  "notes:write",
  "english:read",
  "english:write",
  "files:read",
  "files:write",
] as const;

export const EMPTY_CLOUD_STATE: CloudState = {
  cursor: null,
  entities: {},
  conflicts: [],
  lastLoadedAt: null,
};

function deepClone<T>(value: T): T {
  if (typeof structuredClone === "function") return structuredClone(value);
  return JSON.parse(JSON.stringify(value)) as T;
}

function parseErrorPayload(payload: unknown, fallback: string): string {
  if (!payload || typeof payload !== "object") return fallback;
  const object = payload as Record<string, unknown>;
  if (typeof object.message === "string" && object.message.trim()) return object.message;
  if (object.error && typeof object.error === "object") {
    const message = (object.error as Record<string, unknown>).message;
    if (typeof message === "string" && message.trim()) return message;
  }
  return fallback;
}

async function readJson(response: Response): Promise<unknown> {
  const text = await response.text();
  if (!text) return null;
  try {
    return JSON.parse(text) as unknown;
  } catch {
    return { message: text };
  }
}

export function uuid(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") return crypto.randomUUID();
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (character) => {
    const random = Math.floor(Math.random() * 16);
    const value = character === "x" ? random : (random & 0x3) | 0x8;
    return value.toString(16);
  });
}

export function localDate(date: Date = new Date()): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export function amountToCents(value: string): number {
  const normalized = value.trim().replace(/,/g, "");
  if (!/^-?\d+(\.\d{0,2})?$/.test(normalized)) throw new Error("金额格式无效，最多保留两位小数");
  const negative = normalized.startsWith("-");
  const unsigned = negative ? normalized.slice(1) : normalized;
  const [whole, fraction = ""] = unsigned.split(".");
  const cents = Number(whole) * 100 + Number(fraction.padEnd(2, "0"));
  if (!Number.isSafeInteger(cents)) throw new Error("金额超出安全范围");
  return negative ? -cents : cents;
}

export function formatMoney(cents: number, currency = "CNY", masked = false): string {
  if (masked) return "••••";
  return new Intl.NumberFormat("zh-CN", {
    style: "currency",
    currency,
    maximumFractionDigits: 2,
  }).format(cents / 100);
}

export function baseMeta(userId: string, deviceId: string, id = uuid(), now = new Date().toISOString()): EntityMeta {
  return {
    id,
    userId,
    createdAt: now,
    updatedAt: now,
    localVersion: 1,
    serverVersion: null,
    modifiedByDevice: deviceId,
    deletedAt: null,
  };
}

export function createFinanceAccount(userId: string, deviceId: string, name: string): JsonEntity {
  return {
    meta: baseMeta(userId, deviceId),
    name: name.trim() || "默认账户",
    accountType: "cash",
    color: "#49715d",
    icon: "wallet",
    isArchived: false,
    currency: "CNY",
    openingBalanceCents: 0,
    balanceAt: new Date().toISOString(),
    last4: null,
  };
}

export function createFinanceCategory(
  userId: string,
  deviceId: string,
  name: string,
  categoryType: "expense" | "income",
): JsonEntity {
  return {
    meta: baseMeta(userId, deviceId),
    name: name.trim(),
    categoryType,
    parentId: null,
    icon: categoryType === "expense" ? "receipt" : "coins",
    color: categoryType === "expense" ? "#b86b55" : "#49715d",
    isSystem: false,
    isArchived: false,
  };
}

export interface TransactionInput {
  accountId?: string | null;
  toAccountId?: string | null;
  categoryId?: string | null;
  amount: string;
  type: "expense" | "income" | "refund" | "fee";
  occurredAt?: string;
  localDate?: string;
  status?: "candidate" | "confirmed" | "ignored" | "duplicate";
  sourceType?: string;
  merchant?: string | null;
  item?: string | null;
  counterparty?: string | null;
  note?: string | null;
  externalTransactionId?: string | null;
}

export function createTransaction(userId: string, deviceId: string, input: TransactionInput): JsonEntity {
  const cents = Math.abs(amountToCents(input.amount));
  if (cents === 0) throw new Error("金额必须大于 0");
  const occurredAt = input.occurredAt ?? new Date().toISOString();
  return {
    meta: baseMeta(userId, deviceId),
    transactionType: input.type,
    amountCents: cents,
    currency: "CNY",
    occurredAt,
    localDate: input.localDate ?? occurredAt.slice(0, 10),
    status: input.status ?? "confirmed",
    sourceType: input.sourceType ?? "web_manual",
    accountId: input.accountId ?? null,
    toAccountId: input.toAccountId ?? null,
    categoryId: input.categoryId ?? null,
    merchant: input.merchant?.trim() || null,
    item: input.item?.trim() || null,
    counterparty: input.counterparty?.trim() || null,
    note: input.note?.trim() || null,
    externalTransactionId: input.externalTransactionId?.trim() || null,
  };
}

export function createBudgetPreference(
  userId: string,
  deviceId: string,
  month: string,
  amount: string,
  categoryId: string | null = null,
): JsonEntity {
  const amountCents = Math.abs(amountToCents(amount));
  if (!/^\d{4}-\d{2}$/.test(month)) throw new Error("预算月份格式必须为 YYYY-MM");
  return {
    meta: baseMeta(userId, deviceId),
    preferenceKey: `finance.budget.${month}.${categoryId ?? "all"}`,
    value: { month, categoryId, amountCents, warningThreshold: 0.8 },
  };
}

export function createNoteFolder(userId: string, deviceId: string, name: string, sortOrder = 0): JsonEntity {
  return {
    meta: baseMeta(userId, deviceId),
    name: name.trim(),
    icon: "folder",
    color: "#8a765b",
    sortOrder,
  };
}

export function createNoteTag(userId: string, deviceId: string, name: string): JsonEntity {
  return {
    meta: baseMeta(userId, deviceId),
    name: name.trim(),
    color: "#49715d",
  };
}

export function createNoteTagRelation(userId: string, deviceId: string, noteId: string, tagId: string): JsonEntity {
  return {
    meta: baseMeta(userId, deviceId, `${noteId}:${tagId}`),
    noteId,
    tagId,
  };
}

export interface NoteContent {
  html: string;
  text: string;
  json: unknown;
  markdown?: string;
}

export function createNote(
  userId: string,
  deviceId: string,
  title: string,
  content: string | NoteContent,
  folderId: string | null = null,
): JsonEntity {
  const value: NoteContent = typeof content === "string"
    ? {
        html: content.trim() ? `<p>${escapeHtml(content.trim()).replace(/\n/g, "<br>")}</p>` : "",
        text: content.trim(),
        json: { type: "doc", content: content.trim() },
        markdown: content.trim(),
      }
    : content;
  return {
    meta: baseMeta(userId, deviceId),
    noteType: "quick",
    title: title.trim() || null,
    contentJson: value.json,
    contentHtml: value.html,
    contentText: value.text,
    contentMarkdown: value.markdown ?? value.text,
    summary: value.text.slice(0, 160),
    isPinned: false,
    isFavorite: false,
    isArchived: false,
    folderId,
    aiSummary: null,
    aiTags: null,
    embeddingStatus: null,
    lastAiProcessedAt: null,
  };
}

export function createVocabulary(userId: string, deviceId: string, word: string, definition: string): JsonEntity {
  const displayWord = word.trim();
  if (!displayWord) throw new Error("请输入单词");
  const cleanDefinition = definition.trim();
  return {
    meta: baseMeta(userId, deviceId),
    normalizedWord: displayWord.toLocaleLowerCase("en-US"),
    displayWord,
    definition: cleanDefinition,
    phonetic: "",
    partOfSpeech: "",
    selectedMeanings: cleanDefinition ? [cleanDefinition] : [],
    lemma: displayWord.toLocaleLowerCase("en-US"),
    notes: "",
    masteryLevel: 0,
    reviewStage: 0,
    reviewCount: 0,
    correctCount: 0,
    incorrectCount: 0,
    encounterCount: 1,
    status: "LEARNING",
    tags: [],
    sourceArticleId: null,
    sourceArticleTitle: null,
    sourceSentence: null,
    frequencyRank: null,
    lastReviewedAt: null,
    nextReviewAt: null,
    metadata: null,
  };
}

export function createEnglishHighlight(
  userId: string,
  deviceId: string,
  articleId: string,
  selectedText: string,
  note = "",
): JsonEntity {
  return {
    meta: baseMeta(userId, deviceId),
    articleId,
    blockId: null,
    selectedText: selectedText.trim(),
    startOffset: null,
    endOffset: null,
    prefix: null,
    suffix: null,
    color: "yellow",
    note: note.trim() || null,
  };
}

export function createEnglishLearningRecord(
  userId: string,
  deviceId: string,
  articleId: string,
  summary: string,
  readingTimeSeconds: number,
  newWords: string[] = [],
): JsonEntity {
  return {
    meta: baseMeta(userId, deviceId),
    articleId,
    analysisId: null,
    recordDate: localDate(),
    readingTimeSeconds: Math.max(0, Math.round(readingTimeSeconds)),
    summary: summary.trim(),
    newWords,
    completionStatus: "completed",
    readingStatus: "completed",
    startedAt: null,
    completedAt: new Date().toISOString(),
    score: null,
  };
}

export function createFileMetadata(
  userId: string,
  deviceId: string,
  file: { name: string; type: string; size: number; sha256: string },
): JsonEntity {
  return {
    meta: baseMeta(userId, deviceId),
    originalName: file.name,
    mimeType: file.type || "application/octet-stream",
    sizeBytes: file.size,
    sha256: file.sha256,
    storageState: "pending_upload",
    createdByDevice: deviceId,
  };
}

function escapeHtml(value: string): string {
  return value.replace(/[&<>'"]/g, (character) => {
    const map: Record<string, string> = {
      "&": "&amp;",
      "<": "&lt;",
      ">": "&gt;",
      "'": "&#39;",
      '"': "&quot;",
    };
    return map[character] ?? character;
  });
}

export class AuthApi {
  constructor(private readonly fetcher: FetchLike = fetch) {}

  private async request<T>(url: string, init: RequestInit = {}, csrfToken?: string): Promise<T> {
    const headers = new Headers(init.headers);
    if (init.body !== undefined && !headers.has("content-type")) headers.set("content-type", "application/json");
    if (csrfToken) headers.set("x-csrf-token", csrfToken);
    let response: Response;
    try {
      response = await this.fetcher(url, { ...init, credentials: "include", headers });
    } catch {
      throw new Error("无法连接 LifeTrace 云端，请检查网络后重试");
    }
    const payload = await readJson(response);
    if (!response.ok) throw new Error(parseErrorPayload(payload, `请求失败 (${response.status})`));
    return payload as T;
  }

  login(email: string, password: string, publicDevice: boolean): Promise<WebSession> {
    return this.request<WebSession>("/api/v1/web/session/login", {
      method: "POST",
      body: JSON.stringify({ email, password, requestedScopes: REQUESTED_SCOPES, publicDevice }),
    });
  }

  session(): Promise<WebSession> {
    return this.request<WebSession>("/api/v1/web/session");
  }

  async logout(csrfToken?: string): Promise<void> {
    const token = csrfToken || (await this.request<{ csrfToken: string }>("/api/v1/web/csrf")).csrfToken;
    await this.request("/api/v1/web/session/logout", { method: "POST", body: "{}" }, token);
  }

  async devices(): Promise<DeviceInstallation[]> {
    const response = await this.request<{ devices: DeviceInstallation[] }>("/api/v1/auth/devices");
    return response.devices;
  }

  async renameDevice(deviceId: string, deviceName: string, csrfToken: string): Promise<DeviceInstallation> {
    return this.request(`/api/v1/auth/devices/${encodeURIComponent(deviceId)}`, {
      method: "PATCH",
      body: JSON.stringify({ deviceName: deviceName.trim() }),
    }, csrfToken);
  }

  async revokeDevice(deviceId: string, csrfToken: string): Promise<void> {
    await this.request(`/api/v1/auth/devices/${encodeURIComponent(deviceId)}/revoke`, {
      method: "POST",
      body: "{}",
    }, csrfToken);
  }

  async sessions(): Promise<ManagedSession[]> {
    const response = await this.request<{ sessions: ManagedSession[] }>("/api/v1/auth/sessions");
    return response.sessions;
  }

  async revokeSession(sessionId: string, csrfToken: string): Promise<void> {
    await this.request(`/api/v1/auth/sessions/${encodeURIComponent(sessionId)}`, {
      method: "DELETE",
      body: "{}",
    }, csrfToken);
  }
}

interface PushResult {
  status: string;
  changeId?: string;
  entityType?: string;
  entityId?: string;
  serverVersion?: string;
  currentServerVersion?: string;
  serverEntity?: unknown;
  serverDeleted?: boolean;
  reason?: string;
  message?: string;
}

export class CloudConflictError extends Error {
  constructor(public readonly conflict: CloudConflict) {
    super("数据已在其他设备更新，页面已加载云端最新版本");
    this.name = "CloudConflictError";
  }
}

export class CloudDataStore {
  private state: CloudState = deepClone(EMPTY_CLOUD_STATE);
  private csrfToken: string;

  constructor(
    private readonly userId: string,
    private readonly deviceId: string,
    csrfToken: string,
    private readonly fetcher: FetchLike = fetch,
  ) {
    this.csrfToken = csrfToken.trim();
  }

  setCsrfToken(token: string): void {
    this.csrfToken = token.trim();
  }

  snapshot(): CloudState {
    return deepClone(this.state);
  }

  reset(): void {
    this.state = deepClone(EMPTY_CLOUD_STATE);
  }

  list<T extends JsonEntity = JsonEntity>(entityType: EntityType): T[] {
    return Object.values(this.state.entities[entityType] ?? {}) as T[];
  }

  private clientInfo() {
    return {
      appId: APP_ID,
      clientVersion: APP_VERSION,
      platform: "web",
      protocolVersion: PROTOCOL_VERSION,
      schemaVersion: SCHEMA_VERSION,
      deviceId: this.deviceId,
    };
  }

  private async post<T>(url: string, body: unknown): Promise<T> {
    const headers = new Headers({ "content-type": "application/json" });
    if (this.csrfToken) headers.set("x-csrf-token", this.csrfToken);
    let response: Response;
    try {
      response = await this.fetcher(url, {
        method: "POST",
        credentials: "include",
        headers,
        body: JSON.stringify(body),
      });
    } catch {
      throw new Error("无法连接 LifeTrace 云端，数据尚未保存");
    }
    const payload = await readJson(response);
    if (!response.ok) throw new Error(parseErrorPayload(payload, `云端请求失败 (${response.status})`));
    return payload as T;
  }

  private put(entityType: EntityType, entity: JsonEntity): void {
    const collection = (this.state.entities[entityType] ??= {});
    collection[entity.meta.id] = deepClone(entity);
  }

  private remove(entityType: EntityType, entityId: string): void {
    delete this.state.entities[entityType]?.[entityId];
  }

  async load(): Promise<CloudState> {
    const entities: CloudState["entities"] = {};
    let snapshotId: string | null = null;
    let pageToken: string | null = null;
    let snapshotCursor: string | null = null;
    let completed = false;
    let pages = 0;

    while (!completed && pages < 100) {
      pages += 1;
      const response = await this.post<{
        snapshotId: string;
        snapshotCursor: string;
        items: Array<{ entityType: EntityType; entityId: string; serverVersion: string; payload: JsonEntity }>;
        nextPageToken: string | null;
        completed: boolean;
      }>("/api/v1/sync/snapshot", {
        requestId: uuid(),
        client: this.clientInfo(),
        entityTypes: ENTITY_TYPES,
        snapshotId,
        pageToken,
        pageSize: 500,
      });
      snapshotId = response.snapshotId;
      snapshotCursor = response.snapshotCursor;
      for (const item of response.items) {
        const payload = deepClone(item.payload);
        payload.meta.serverVersion = item.serverVersion;
        const collection = (entities[item.entityType] ??= {});
        collection[item.entityId] = payload;
      }
      completed = response.completed;
      pageToken = response.nextPageToken;
      if (!completed && !pageToken) throw new Error("云端快照分页中断");
    }
    if (!completed) throw new Error("云端数据量过大，快照页数超过安全上限");

    this.state = {
      cursor: snapshotCursor,
      entities,
      conflicts: this.state.conflicts,
      lastLoadedAt: new Date().toISOString(),
    };
    return this.snapshot();
  }

  async refresh(): Promise<CloudState> {
    if (!this.state.cursor) return this.load();
    let hasMore = true;
    let loops = 0;
    while (hasMore && loops < 50) {
      loops += 1;
      const response = await this.post<{
        changes: Array<{
          entityType: EntityType;
          entityId: string;
          operation: "upsert" | "delete";
          serverVersion: string;
          payload: JsonEntity | null;
        }>;
        nextCursor: string;
        hasMore: boolean;
      }>("/api/v1/sync/pull", {
        requestId: uuid(),
        client: this.clientInfo(),
        afterCursor: this.state.cursor,
        limit: 500,
        entityTypes: ENTITY_TYPES,
      });
      for (const change of response.changes) {
        if (change.operation === "delete") this.remove(change.entityType, change.entityId);
        else if (change.payload) {
          const payload = deepClone(change.payload);
          payload.meta.serverVersion = change.serverVersion;
          this.put(change.entityType, payload);
        }
      }
      this.state.cursor = response.nextCursor;
      hasMore = response.hasMore;
    }
    if (hasMore) throw new Error("云端增量数据量过大，请再次刷新");
    this.state.lastLoadedAt = new Date().toISOString();
    return this.snapshot();
  }

  private prepareUpsert(entityType: EntityType, entity: JsonEntity): { entity: JsonEntity; change: SyncChange } {
    const existing = this.state.entities[entityType]?.[entity.meta.id];
    const now = new Date().toISOString();
    const payload = deepClone(entity);
    payload.meta = {
      ...payload.meta,
      userId: this.userId,
      updatedAt: now,
      localVersion: Math.max(1, Number(existing?.meta.localVersion ?? payload.meta.localVersion ?? 0) + (existing ? 1 : 0)),
      modifiedByDevice: this.deviceId,
    };
    const change: SyncChange = {
      changeId: uuid(),
      entityType,
      entityId: payload.meta.id,
      operation: "upsert",
      baseServerVersion: existing?.meta.serverVersion ?? payload.meta.serverVersion ?? "0",
      entitySchemaVersion: SCHEMA_VERSION,
      clientModifiedAt: now,
      payload,
      atomicGroupId: null,
      dependencies: [],
    };
    return { entity: payload, change };
  }

  private prepareDelete(entityType: EntityType, entityId: string): SyncChange {
    const existing = this.state.entities[entityType]?.[entityId];
    if (!existing) throw new Error("记录不存在或已被删除");
    return {
      changeId: uuid(),
      entityType,
      entityId,
      operation: "delete",
      baseServerVersion: existing.meta.serverVersion ?? "0",
      entitySchemaVersion: SCHEMA_VERSION,
      clientModifiedAt: new Date().toISOString(),
      payload: null,
      atomicGroupId: null,
      dependencies: [],
    };
  }

  private async push(changes: SyncChange[]): Promise<PushResult[]> {
    const response = await this.post<{ results: PushResult[] }>("/api/v1/sync/push", {
      requestId: uuid(),
      client: this.clientInfo(),
      changes,
    });
    return response.results;
  }

  private applyConflict(result: PushResult, fallbackType: EntityType, fallbackId: string): CloudConflict {
    const entityType = (result.entityType || fallbackType) as EntityType;
    const entityId = result.entityId || fallbackId;
    if (result.serverDeleted) this.remove(entityType, entityId);
    else if (result.serverEntity && typeof result.serverEntity === "object") {
      const serverEntity = deepClone(result.serverEntity as JsonEntity);
      serverEntity.meta.serverVersion = result.currentServerVersion ?? serverEntity.meta.serverVersion ?? null;
      this.put(entityType, serverEntity);
    }
    const conflict: CloudConflict = {
      entityType,
      entityId,
      reason: result.reason || "version_mismatch",
      occurredAt: new Date().toISOString(),
    };
    this.state.conflicts = [conflict, ...this.state.conflicts].slice(0, 20);
    return conflict;
  }

  async upsert(entityType: EntityType, entity: JsonEntity): Promise<CloudState> {
    const prepared = this.prepareUpsert(entityType, entity);
    const [result] = await this.push([prepared.change]);
    if (!result) throw new Error("云端未返回保存结果");
    if (result.status === "accepted" || result.status === "duplicate") {
      prepared.entity.meta.serverVersion = result.serverVersion ?? prepared.entity.meta.serverVersion ?? "0";
      this.put(entityType, prepared.entity);
      this.state.lastLoadedAt = new Date().toISOString();
      return this.snapshot();
    }
    if (result.status === "conflict") throw new CloudConflictError(this.applyConflict(result, entityType, entity.meta.id));
    throw new Error(result.message || result.reason || "云端拒绝保存该记录");
  }

  async delete(entityType: EntityType, entityId: string): Promise<CloudState> {
    const change = this.prepareDelete(entityType, entityId);
    const [result] = await this.push([change]);
    if (!result) throw new Error("云端未返回删除结果");
    if (result.status === "accepted" || result.status === "duplicate") {
      this.remove(entityType, entityId);
      this.state.lastLoadedAt = new Date().toISOString();
      return this.snapshot();
    }
    if (result.status === "conflict") throw new CloudConflictError(this.applyConflict(result, entityType, entityId));
    throw new Error(result.message || result.reason || "云端拒绝删除该记录");
  }

  async batchUpsert(entityType: EntityType, entities: JsonEntity[]): Promise<{ state: CloudState; saved: number; errors: string[] }> {
    let saved = 0;
    const errors: string[] = [];
    for (let offset = 0; offset < entities.length; offset += 100) {
      const prepared = entities.slice(offset, offset + 100).map((entity) => this.prepareUpsert(entityType, entity));
      const results = await this.push(prepared.map((item) => item.change));
      const byChangeId = new Map(results.map((result) => [result.changeId, result]));
      for (const item of prepared) {
        const result = byChangeId.get(item.change.changeId);
        if (!result) {
          errors.push(`${item.change.entityId}: 云端未返回结果`);
          continue;
        }
        if (result.status === "accepted" || result.status === "duplicate") {
          item.entity.meta.serverVersion = result.serverVersion ?? item.entity.meta.serverVersion ?? "0";
          this.put(entityType, item.entity);
          saved += 1;
        } else if (result.status === "conflict") {
          this.applyConflict(result, entityType, item.entity.meta.id);
          errors.push(`${item.entity.meta.id}: 数据冲突`);
        } else {
          errors.push(`${item.entity.meta.id}: ${result.message || result.reason || "保存被拒绝"}`);
        }
      }
    }
    this.state.lastLoadedAt = new Date().toISOString();
    return { state: this.snapshot(), saved, errors };
  }
}

function text(entity: JsonEntity, key: string): string {
  const value = entity[key];
  return typeof value === "string" ? value : "";
}

export function searchEntities(state: CloudState, query: string): SearchHit[] {
  const needle = query.trim().toLocaleLowerCase();
  if (!needle) return [];
  const hits: SearchHit[] = [];
  const add = (entityType: EntityType, entity: JsonEntity, title: string, subtitle: string, route: string) => {
    if (`${title} ${subtitle}`.toLocaleLowerCase().includes(needle)) {
      hits.push({ id: entity.meta.id, entityType, title: title || "未命名记录", subtitle, updatedAt: entity.meta.updatedAt, route });
    }
  };
  for (const entity of Object.values(state.entities["finance.transaction"] ?? {})) {
    add("finance.transaction", entity, text(entity, "merchant") || text(entity, "item") || text(entity, "note") || "财务流水", `${text(entity, "localDate")} ${text(entity, "counterparty")}`, "/finance/transactions");
  }
  for (const entity of Object.values(state.entities["note.note"] ?? {})) {
    add("note.note", entity, text(entity, "title") || "无标题笔记", text(entity, "contentText") || text(entity, "summary"), `/notes/${entity.meta.id}`);
  }
  for (const entity of Object.values(state.entities["english.article"] ?? {})) {
    add("english.article", entity, text(entity, "title") || "English article", text(entity, "summary") || text(entity, "content"), `/english/articles/${entity.meta.id}`);
  }
  for (const entity of Object.values(state.entities["english.vocabulary"] ?? {})) {
    add("english.vocabulary", entity, text(entity, "displayWord"), text(entity, "definition"), "/english/vocabulary");
  }
  return hits.sort((a, b) => b.updatedAt.localeCompare(a.updatedAt)).slice(0, 50);
}

export function findProbableDuplicate(transaction: JsonEntity, existing: JsonEntity[]): JsonEntity | null {
  const externalId = text(transaction, "externalTransactionId");
  if (externalId) {
    const exact = existing.find((item) => text(item, "externalTransactionId") === externalId);
    if (exact) return exact;
  }
  const amount = Number(transaction.amountCents ?? 0);
  const date = text(transaction, "localDate");
  const merchant = text(transaction, "merchant").toLocaleLowerCase();
  return existing.find((item) => {
    const sameCore = Number(item.amountCents ?? 0) === amount && text(item, "localDate") === date;
    if (!sameCore) return false;
    const existingMerchant = text(item, "merchant").toLocaleLowerCase();
    return !merchant || !existingMerchant || merchant === existingMerchant;
  }) ?? null;
}
