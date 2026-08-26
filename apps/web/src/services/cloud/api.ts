import {
  ENTITY_TYPES, PROTOCOL_VERSION, REQUESTED_SCOPES,
  SCHEMA_VERSION, WEB_SYNC_CLIENT, clone, uuid,
  type CloudConflict, type CloudState, type DeviceInstallation, type EntityType,
  type FetchLike, type JsonEntity, type ManagedSession, type PullResponse,
  type SyncClientIdentity,
  type PushResult, type SnapshotResponse, type SyncChange, type WebSession,
  type BeeCountIntegrationStatus, type BeeCountLedgerList, type BeeCountLedgerSnapshot,
} from "./types";
import { API_BASE } from "./base";
import { browserFetch } from "./http";
import {
  ClientRequestError,
  bindFetch,
  clientLogger,
} from "../../../src/services/clientObservability";

async function readJson(response: Response): Promise<unknown> {
  const raw = await response.text();
  if (!raw) return null;
  try { return JSON.parse(raw) as unknown; }
  catch { return { message: raw }; }
}

function errorMessage(payload: unknown, fallback: string): string {
  if (!payload || typeof payload !== "object") return fallback;
  const value = payload as Record<string, unknown>;
  if (typeof value.message === "string" && value.message.trim()) return value.message;
  if (value.error && typeof value.error === "object") {
    const message = (value.error as Record<string, unknown>).message;
    if (typeof message === "string" && message.trim()) return message;
  }
  return fallback;
}

function apiUrl(path: string): string {
  return `${API_BASE}${path}`;
}

function safeApiPath(value: string): string {
  return value.split(/[?#]/, 1)[0] || "unknown-request";
}

function cloudTransportError(cause: unknown, fallback: string): Error {
  if (cause instanceof ClientRequestError && cause.category === "programming") {
    return new Error(`LifeTrace 客户端请求执行失败，请查看诊断日志（错误编号 ${cause.errorId}）`, {
      cause,
    });
  }
  return new Error(fallback, { cause });
}

function logTransportFailure(module: string, action: string, cause: unknown): void {
  if (cause instanceof ClientRequestError) return;
  clientLogger.error("cloud.web.transport_failed", {
    module,
    action: safeApiPath(action),
  }, cause);
}

function logHttpFailure(module: string, action: string, response: Response, error: Error): void {
  clientLogger.error("cloud.web.http_failed", {
    module,
    action: safeApiPath(action),
    status: response.status,
    stage: "response.http",
    requestSent: true,
  }, error);
}

export class AuthApi {
  private readonly fetcher: FetchLike;

  constructor(fetcher: FetchLike = browserFetch) {
    this.fetcher = bindFetch(fetcher);
  }

  private async request<T>(url: string, init: RequestInit = {}, csrfToken?: string): Promise<T> {
    const headers = new Headers(init.headers);
    if (init.body !== undefined && !headers.has("content-type")) headers.set("content-type", "application/json");
    if (csrfToken) headers.set("x-csrf-token", csrfToken);
    let response: Response;
    try {
      response = await this.fetcher(apiUrl(url), { ...init, credentials: "include", headers });
    } catch (cause) {
      logTransportFailure("web-auth", url, cause);
      throw cloudTransportError(cause, "无法连接 LifeTrace 云端，请检查网络后重试");
    }
    const payload = await readJson(response);
    if (!response.ok) {
      const error = new Error(errorMessage(payload, `请求失败 (${response.status})`));
      logHttpFailure("web-auth", url, response, error);
      throw error;
    }
    return payload as T;
  }

  login(email: string, password: string, publicDevice: boolean): Promise<WebSession> {
    return this.request("/api/v1/web/session/login", { method: "POST", body: JSON.stringify({ email, password, requestedScopes: REQUESTED_SCOPES, publicDevice }) });
  }

  session(): Promise<WebSession> { return this.request("/api/v1/web/session"); }

  async logout(csrfToken?: string): Promise<void> {
    const token = csrfToken || (await this.request<{ csrfToken: string }>("/api/v1/web/csrf")).csrfToken;
    await this.request("/api/v1/web/session/logout", { method: "POST", body: "{}" }, token);
  }

  async devices(): Promise<DeviceInstallation[]> {
    return (await this.request<{ devices: DeviceInstallation[] }>("/api/v1/auth/devices")).devices;
  }

  renameDevice(deviceId: string, deviceName: string, csrfToken: string): Promise<DeviceInstallation> {
    return this.request(`/api/v1/auth/devices/${encodeURIComponent(deviceId)}`, { method: "PATCH", body: JSON.stringify({ deviceName: deviceName.trim() }) }, csrfToken);
  }

  async revokeDevice(deviceId: string, csrfToken: string): Promise<void> {
    await this.request(`/api/v1/auth/devices/${encodeURIComponent(deviceId)}/revoke`, { method: "POST", body: "{}" }, csrfToken);
  }

  async sessions(): Promise<ManagedSession[]> {
    return (await this.request<{ sessions: ManagedSession[] }>("/api/v1/auth/sessions")).sessions;
  }

  async revokeSession(sessionId: string, csrfToken: string): Promise<void> {
    await this.request(`/api/v1/auth/sessions/${encodeURIComponent(sessionId)}`, { method: "DELETE", body: "{}" }, csrfToken);
  }
}

export class BeeCountFinanceApi {
  private readonly fetcher: FetchLike;

  constructor(fetcher: FetchLike = browserFetch) {
    this.fetcher = bindFetch(fetcher);
  }

  private async get<T>(path: string): Promise<T> {
    let response: Response;
    try {
      response = await this.fetcher(apiUrl(path), { method: "GET", credentials: "include" });
    } catch (cause) {
      logTransportFailure("beecount-finance", path, cause);
      throw cloudTransportError(cause, "无法连接 BeeCount 财务适配器");
    }
    const payload = await readJson(response);
    if (!response.ok) {
      const error = new Error(errorMessage(payload, `BeeCount 财务读取失败 (${response.status})`));
      logHttpFailure("beecount-finance", path, response, error);
      throw error;
    }
    return payload as T;
  }

  status(): Promise<BeeCountIntegrationStatus> {
    return this.get("/api/v1/integrations/beecount/status");
  }

  ledgers(): Promise<BeeCountLedgerList> {
    return this.get("/api/v1/integrations/beecount/ledgers");
  }

  snapshot(ledgerId: string, limit = 200, offset = 0): Promise<BeeCountLedgerSnapshot> {
    const safeLimit = Math.max(1, Math.min(500, Math.trunc(limit)));
    const safeOffset = Math.max(0, Math.trunc(offset));
    return this.get(`/api/v1/integrations/beecount/ledgers/${encodeURIComponent(ledgerId)}/snapshot?limit=${safeLimit}&offset=${safeOffset}`);
  }
}

export class CloudConflictError extends Error {
  constructor(public readonly conflict: CloudConflict) {
    super("数据已在其他设备更新，页面已加载云端最新版本");
    this.name = "CloudConflictError";
  }
}

export class CloudDataStore {
  private state: CloudState = { cursor: null, entities: {}, conflicts: [], lastLoadedAt: null };
  private csrfToken: string;
  private readonly fetcher: FetchLike;

  constructor(
    private readonly userId: string,
    private readonly deviceId: string,
    csrfToken: string,
    fetcher: FetchLike = browserFetch,
    private readonly syncClient: SyncClientIdentity = WEB_SYNC_CLIENT,
  ) {
    this.csrfToken = csrfToken.trim();
    this.fetcher = bindFetch(fetcher);
  }

  setCsrfToken(token: string): void { this.csrfToken = token.trim(); }
  snapshot(): CloudState { return clone(this.state); }
  reset(): void { this.state = { cursor: null, entities: {}, conflicts: [], lastLoadedAt: null }; }
  list<T extends JsonEntity = JsonEntity>(entityType: EntityType): T[] { return Object.values(this.state.entities[entityType] ?? {}) as T[]; }

  private clientInfo() {
    return { ...this.syncClient, protocolVersion: PROTOCOL_VERSION, schemaVersion: SCHEMA_VERSION, deviceId: this.deviceId };
  }

  private async post<T>(url: string, body: unknown): Promise<T> {
    const headers = new Headers({ "content-type": "application/json" });
    if (this.csrfToken) headers.set("x-csrf-token", this.csrfToken);
    let response: Response;
    try {
      response = await this.fetcher(apiUrl(url), { method: "POST", credentials: "include", headers, body: JSON.stringify(body) });
    } catch (cause) {
      logTransportFailure("web-sync", url, cause);
      throw cloudTransportError(cause, "无法连接 LifeTrace 云端，数据尚未保存");
    }
    const payload = await readJson(response);
    if (!response.ok) {
      const error = new Error(errorMessage(payload, `云端请求失败 (${response.status})`));
      logHttpFailure("web-sync", url, response, error);
      throw error;
    }
    return payload as T;
  }

  private put(entityType: EntityType, entity: JsonEntity): void {
    const collection: Record<string, JsonEntity> = this.state.entities[entityType] ?? {};
    collection[entity.meta.id] = clone(entity);
    this.state.entities[entityType] = collection;
  }

  private remove(entityType: EntityType, entityId: string): void { delete this.state.entities[entityType]?.[entityId]; }

  async load(): Promise<CloudState> {
    const loaded: CloudState["entities"] = {};
    let snapshotId: string | null = null;
    let pageToken: string | null = null;
    let snapshotCursor: string | null = null;
    let completed = false;
    let pages = 0;
    while (!completed && pages < 100) {
      pages += 1;
      const result: SnapshotResponse = await this.post("/api/v1/sync/snapshot", { requestId: uuid(), client: this.clientInfo(), entityTypes: ENTITY_TYPES, snapshotId, pageToken, pageSize: 500 });
      snapshotId = result.snapshotId;
      snapshotCursor = result.snapshotCursor;
      for (const item of result.items) {
        const payload = clone(item.payload);
        payload.meta.serverVersion = item.serverVersion;
        const collection: Record<string, JsonEntity> = loaded[item.entityType] ?? {};
        collection[item.entityId] = payload;
        loaded[item.entityType] = collection;
      }
      completed = result.completed;
      pageToken = result.nextPageToken;
      if (!completed && !pageToken) throw new Error("云端快照分页中断");
    }
    if (!completed) throw new Error("云端数据量过大，快照页数超过安全上限");
    this.state = { cursor: snapshotCursor, entities: loaded, conflicts: this.state.conflicts, lastLoadedAt: new Date().toISOString() };
    return this.snapshot();
  }

  async refresh(): Promise<CloudState> {
    if (!this.state.cursor) return this.load();
    let hasMore = true;
    let loops = 0;
    while (hasMore && loops < 50) {
      loops += 1;
      const result: PullResponse = await this.post("/api/v1/sync/pull", { requestId: uuid(), client: this.clientInfo(), afterCursor: this.state.cursor, limit: 500, entityTypes: ENTITY_TYPES });
      for (const change of result.changes) {
        if (change.operation === "delete") this.remove(change.entityType, change.entityId);
        else if (change.payload) {
          const payload = clone(change.payload);
          payload.meta.serverVersion = change.serverVersion;
          this.put(change.entityType, payload);
        }
      }
      this.state.cursor = result.nextCursor;
      hasMore = result.hasMore;
    }
    if (hasMore) throw new Error("云端增量数据量过大，请再次刷新");
    this.state.lastLoadedAt = new Date().toISOString();
    return this.snapshot();
  }

  private prepareUpsert(entityType: EntityType, entity: JsonEntity): { entity: JsonEntity; change: SyncChange } {
    const existing = this.state.entities[entityType]?.[entity.meta.id];
    const now = new Date().toISOString();
    const payload = clone(entity);
    payload.meta = { ...payload.meta, userId: this.userId, updatedAt: now, localVersion: Math.max(1, Number(existing?.meta.localVersion ?? payload.meta.localVersion ?? 0) + (existing ? 1 : 0)), modifiedByDevice: this.deviceId };
    return { entity: payload, change: { changeId: uuid(), entityType, entityId: payload.meta.id, operation: "upsert", baseServerVersion: existing?.meta.serverVersion ?? payload.meta.serverVersion ?? "0", entitySchemaVersion: SCHEMA_VERSION, clientModifiedAt: now, payload, atomicGroupId: null, dependencies: [] } };
  }

  private prepareDelete(entityType: EntityType, entityId: string): SyncChange {
    const existing = this.state.entities[entityType]?.[entityId];
    if (!existing) throw new Error("记录不存在或已被删除");
    return { changeId: uuid(), entityType, entityId, operation: "delete", baseServerVersion: existing.meta.serverVersion ?? "0", entitySchemaVersion: SCHEMA_VERSION, clientModifiedAt: new Date().toISOString(), payload: null, atomicGroupId: null, dependencies: [] };
  }

  private async push(changes: SyncChange[]): Promise<PushResult[]> {
    return (await this.post<{ results: PushResult[] }>("/api/v1/sync/push", { requestId: uuid(), client: this.clientInfo(), changes })).results;
  }

  private applyConflict(result: PushResult, fallbackType: EntityType, fallbackId: string): CloudConflict {
    const entityType = (result.entityType || fallbackType) as EntityType;
    const entityId = result.entityId || fallbackId;
    if (result.serverDeleted) this.remove(entityType, entityId);
    else if (result.serverEntity && typeof result.serverEntity === "object") {
      const serverEntity = clone(result.serverEntity as JsonEntity);
      serverEntity.meta.serverVersion = result.currentServerVersion ?? serverEntity.meta.serverVersion ?? null;
      this.put(entityType, serverEntity);
    }
    const conflict = { entityType, entityId, reason: result.reason || "version_mismatch", occurredAt: new Date().toISOString() };
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
      const results = await this.push(prepared.map(({ change }) => change));
      const byId = new Map(results.map((result) => [result.changeId, result]));
      for (const item of prepared) {
        const result = byId.get(item.change.changeId);
        if (!result) { errors.push(`${item.change.entityId}: 云端未返回结果`); continue; }
        if (result.status === "accepted" || result.status === "duplicate") {
          item.entity.meta.serverVersion = result.serverVersion ?? item.entity.meta.serverVersion ?? "0";
          this.put(entityType, item.entity); saved += 1;
        } else if (result.status === "conflict") {
          this.applyConflict(result, entityType, item.entity.meta.id); errors.push(`${item.entity.meta.id}: 数据冲突`);
        } else errors.push(`${item.entity.meta.id}: ${result.message || result.reason || "保存被拒绝"}`);
      }
    }
    this.state.lastLoadedAt = new Date().toISOString();
    return { state: this.snapshot(), saved, errors };
  }
}
