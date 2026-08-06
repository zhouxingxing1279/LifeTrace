export const APP_VERSION = "0.2.1";
export const APP_ID = "lifetrace-web";
export const PROTOCOL_VERSION = 1;
export const SCHEMA_VERSION = 1;
export const ENTITY_TYPES = [
  "finance.account",
  "finance.transaction",
  "note.note",
  "english.article",
  "english.vocabulary",
] as const;

export type EntityType = (typeof ENTITY_TYPES)[number];
export type JsonRecord = Record<string, unknown>;

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

export interface SyncEntity extends JsonRecord {
  meta: EntityMeta;
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

export interface SyncChange {
  changeId: string;
  entityType: EntityType;
  entityId: string;
  operation: "upsert" | "delete";
  baseServerVersion: string;
  entitySchemaVersion: number;
  clientModifiedAt: string;
  payload: SyncEntity | null;
  atomicGroupId: null;
  dependencies: Array<{ entityType: string; entityId: string }>;
}

export interface ConflictRecord {
  entityType: EntityType;
  entityId: string;
  reason: string;
  resolvedAt: string;
}

export interface PersistedState {
  cursor: string | null;
  entities: Partial<Record<EntityType, Record<string, SyncEntity>>>;
  outbox: SyncChange[];
  conflicts: ConflictRecord[];
  lastSyncedAt: string | null;
}

export interface StorageLike {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

export interface FetchLike {
  (input: RequestInfo | URL, init?: RequestInit): Promise<Response>;
}

export const EMPTY_STATE: PersistedState = {
  cursor: null,
  entities: {},
  outbox: [],
  conflicts: [],
  lastSyncedAt: null,
};

const REQUESTED_SCOPES = [
  "sync:read",
  "sync:write",
  "finance:read",
  "finance:write",
  "notes:read",
  "notes:write",
  "english:read",
  "english:write",
];

function cloneState(state: PersistedState): PersistedState {
  return JSON.parse(JSON.stringify(state)) as PersistedState;
}

function parseErrorPayload(payload: unknown, fallback: string): string {
  if (!payload || typeof payload !== "object") return fallback;
  const object = payload as Record<string, unknown>;
  const direct = object.message;
  if (typeof direct === "string" && direct.trim()) return direct;
  const error = object.error;
  if (error && typeof error === "object") {
    const message = (error as Record<string, unknown>).message;
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
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
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
  if (!/^-?\d+(\.\d{0,2})?$/.test(normalized)) {
    throw new Error("金额格式无效，最多保留两位小数");
  }
  const negative = normalized.startsWith("-");
  const unsigned = negative ? normalized.slice(1) : normalized;
  const [whole, fraction = ""] = unsigned.split(".");
  const cents = Number(whole) * 100 + Number(fraction.padEnd(2, "0"));
  if (!Number.isSafeInteger(cents)) throw new Error("金额超出安全范围");
  return negative ? -cents : cents;
}

export function formatMoney(cents: number, currency = "CNY"): string {
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

export function createFinanceAccount(userId: string, deviceId: string, name: string): SyncEntity {
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

export function createTransaction(
  userId: string,
  deviceId: string,
  input: { accountId?: string | null; amount: string; type: "expense" | "income"; note: string },
): SyncEntity {
  const cents = Math.abs(amountToCents(input.amount));
  if (cents === 0) throw new Error("金额必须大于 0");
  const now = new Date();
  return {
    meta: baseMeta(userId, deviceId),
    transactionType: input.type,
    amountCents: cents,
    currency: "CNY",
    occurredAt: now.toISOString(),
    localDate: localDate(now),
    status: "confirmed",
    sourceType: "web_manual",
    accountId: input.accountId || null,
    toAccountId: null,
    categoryId: null,
    merchant: null,
    item: null,
    counterparty: null,
    note: input.note.trim() || null,
    externalTransactionId: null,
  };
}

export function createNote(userId: string, deviceId: string, title: string, content: string): SyncEntity {
  const cleanContent = content.trim();
  return {
    meta: baseMeta(userId, deviceId),
    noteType: "quick",
    title: title.trim() || null,
    contentJson: { type: "doc", content: cleanContent },
    contentHtml: cleanContent ? `<p>${escapeHtml(cleanContent).replace(/\n/g, "<br>")}</p>` : "",
    contentText: cleanContent,
    contentMarkdown: cleanContent,
    summary: cleanContent.slice(0, 120),
    isPinned: false,
    isFavorite: false,
    isArchived: false,
    folderId: null,
    aiSummary: null,
    aiTags: null,
    embeddingStatus: null,
    lastAiProcessedAt: null,
  };
}

export function createVocabulary(
  userId: string,
  deviceId: string,
  word: string,
  definition: string,
): SyncEntity {
  const displayWord = word.trim();
  if (!displayWord) throw new Error("请输入单词");
  return {
    meta: baseMeta(userId, deviceId),
    normalizedWord: displayWord.toLocaleLowerCase("en-US"),
    displayWord,
    definition: definition.trim(),
    phonetic: "",
    partOfSpeech: "",
    selectedMeanings: definition.trim() ? [definition.trim()] : [],
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

function escapeHtml(value: string): string {
  return value.replace(/[&<>'"]/g, (character) => {
    const map: Record<string, string> = { "&": "&amp;", "<": "&lt;", ">": "&gt;", "'": "&#39;", '"': "&quot;" };
    return map[character] ?? character;
  });
}

export class AuthApi {
  constructor(private readonly fetcher: FetchLike = fetch) {}

  private async request<T>(url: string, init: RequestInit = {}): Promise<T> {
    const response = await this.fetcher(url, {
      ...init,
      credentials: "include",
      headers: { "content-type": "application/json", ...(init.headers ?? {}) },
    });
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
    await this.request<unknown>("/api/v1/web/session/logout", {
      method: "POST",
      headers: { "x-csrf-token": token },
      body: "{}",
    });
  }
}

export class WebSyncStore {
  private state: PersistedState;
  private readonly key: string;
  private csrfToken = "";

  constructor(
    private readonly userId: string,
    private readonly deviceId: string,
    private readonly storage: StorageLike,
    private readonly fetcher: FetchLike = fetch,
  ) {
    this.key = `lifetrace:web:v1:${userId}`;
    this.state = this.load();
  }

  private load(): PersistedState {
    const raw = this.storage.getItem(this.key);
    if (!raw) return cloneState(EMPTY_STATE);
    try {
      const parsed = JSON.parse(raw) as Partial<PersistedState>;
      return {
        cursor: typeof parsed.cursor === "string" ? parsed.cursor : null,
        entities: parsed.entities ?? {},
        outbox: Array.isArray(parsed.outbox) ? parsed.outbox : [],
        conflicts: Array.isArray(parsed.conflicts) ? parsed.conflicts : [],
        lastSyncedAt: typeof parsed.lastSyncedAt === "string" ? parsed.lastSyncedAt : null,
      };
    } catch {
      return cloneState(EMPTY_STATE);
    }
  }

  private save(): void {
    this.storage.setItem(this.key, JSON.stringify(this.state));
  }

  snapshot(): PersistedState {
    return cloneState(this.state);
  }

  setCsrfToken(token: string): void {
    this.csrfToken = token.trim();
  }

  clear(): void {
    this.storage.removeItem(this.key);
    this.state = cloneState(EMPTY_STATE);
  }

  list<T extends SyncEntity = SyncEntity>(entityType: EntityType): T[] {
    return Object.values(this.state.entities[entityType] ?? {}) as T[];
  }

  queueUpsert(entityType: EntityType, entity: SyncEntity): SyncChange {
    const now = new Date().toISOString();
    const existing = this.state.entities[entityType]?.[entity.meta.id];
    const payload = cloneState(entity as unknown as PersistedState) as unknown as SyncEntity;
    payload.meta = {
      ...payload.meta,
      userId: this.userId,
      updatedAt: now,
      localVersion: Math.max(1, Number(existing?.meta.localVersion ?? payload.meta.localVersion ?? 0) + (existing ? 1 : 0)),
      modifiedByDevice: this.deviceId,
    };
    const baseServerVersion = existing?.meta.serverVersion ?? payload.meta.serverVersion ?? "0";
    this.put(entityType, payload);
    const change: SyncChange = {
      changeId: uuid(),
      entityType,
      entityId: payload.meta.id,
      operation: "upsert",
      baseServerVersion,
      entitySchemaVersion: SCHEMA_VERSION,
      clientModifiedAt: now,
      payload,
      atomicGroupId: null,
      dependencies: [],
    };
    this.state.outbox.push(change);
    this.save();
    return change;
  }

  queueDelete(entityType: EntityType, entityId: string): SyncChange | null {
    const existing = this.state.entities[entityType]?.[entityId];
    if (!existing) return null;
    delete this.state.entities[entityType]?.[entityId];
    const change: SyncChange = {
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
    this.state.outbox.push(change);
    this.save();
    return change;
  }

  private put(entityType: EntityType, entity: SyncEntity): void {
    const collection = (this.state.entities[entityType] ??= {});
    collection[entity.meta.id] = entity;
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
    const headers: Record<string, string> = { "content-type": "application/json" };
    if (this.csrfToken) headers["x-csrf-token"] = this.csrfToken;
    const response = await this.fetcher(url, {
      method: "POST",
      credentials: "include",
      headers,
      body: JSON.stringify(body),
    });
    const payload = await readJson(response);
    if (!response.ok) throw new Error(parseErrorPayload(payload, `同步失败 (${response.status})`));
    return payload as T;
  }

  async sync(): Promise<PersistedState> {
    if (this.state.outbox.length) await this.push();
    await this.pull();
    this.state.lastSyncedAt = new Date().toISOString();
    this.save();
    return this.snapshot();
  }

  private async push(): Promise<void> {
    const batch = this.state.outbox.slice(0, 100);
    if (!batch.length) return;
    const response = await this.post<{
      results: Array<Record<string, unknown>>;
      latestCursor: string;
    }>("/api/v1/sync/push", {
      requestId: uuid(),
      client: this.clientInfo(),
      changes: batch,
    });

    const completed = new Set<string>();
    for (const result of response.results) {
      const status = String(result.status ?? "");
      const changeId = String(result.changeId ?? "");
      const entityType = String(result.entityType ?? "") as EntityType;
      const entityId = String(result.entityId ?? "");
      if (status === "accepted" || status === "duplicate") {
        completed.add(changeId);
        const entity = this.state.entities[entityType]?.[entityId];
        if (entity) entity.meta.serverVersion = String(result.serverVersion ?? entity.meta.serverVersion ?? "0");
      } else if (status === "conflict") {
        completed.add(changeId);
        const serverEntity = result.serverEntity;
        if (result.serverDeleted === true) {
          delete this.state.entities[entityType]?.[entityId];
        } else if (serverEntity && typeof serverEntity === "object") {
          const entity = serverEntity as SyncEntity;
          entity.meta.serverVersion = String(result.currentServerVersion ?? entity.meta.serverVersion ?? "0");
          this.put(entityType, entity);
        }
        this.state.conflicts.unshift({
          entityType,
          entityId,
          reason: String(result.reason ?? "server_version_mismatch"),
          resolvedAt: new Date().toISOString(),
        });
      }
    }
    this.state.outbox = this.state.outbox.filter((change) => !completed.has(change.changeId));
    // `latestCursor` is informational for push. Advancing the pull cursor here
    // would skip remote changes committed after our last successful pull.
    this.save();
    if (this.state.outbox.length && completed.size > 0) await this.push();
  }

  private async pull(): Promise<void> {
    let hasMore = true;
    let loops = 0;
    while (hasMore && loops < 25) {
      loops += 1;
      const response = await this.post<{
        changes: Array<{
          entityType: EntityType;
          entityId: string;
          operation: "upsert" | "delete";
          serverVersion: string;
          payload: SyncEntity | null;
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
        if (change.operation === "delete") {
          delete this.state.entities[change.entityType]?.[change.entityId];
        } else if (change.payload) {
          change.payload.meta.serverVersion = change.serverVersion;
          this.put(change.entityType, change.payload);
        }
      }
      this.state.cursor = response.nextCursor;
      hasMore = response.hasMore;
      this.save();
    }
    if (hasMore) throw new Error("同步数据量过大，请再次点击同步继续拉取");
  }
}

export function getOrCreateDeviceId(storage: StorageLike): string {
  const key = "lifetrace:web:device-id";
  const existing = storage.getItem(key);
  if (existing) return existing;
  const created = uuid();
  storage.setItem(key, created);
  return created;
}
