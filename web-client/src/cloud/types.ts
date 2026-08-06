export const APP_VERSION = "0.2.1";
export const APP_ID = "lifetrace-web";
export const PROTOCOL_VERSION = 1;
export const SCHEMA_VERSION = 1;

export const ENTITY_TYPES = [
  "finance.account", "finance.category", "finance.transaction",
  "note.folder", "note.note", "note.tag", "note.tag_relation",
  "english.article", "english.highlight", "english.learning_record",
  "english.vocabulary", "file.metadata", "user.preference",
] as const;

export type EntityType = (typeof ENTITY_TYPES)[number];
export type FetchLike = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;
export type JsonEntity = Record<string, unknown> & { meta: EntityMeta };

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

export interface AuthUser { id: string; email: string; displayName?: string | null; }
export interface AuthSession {
  id: string; appId: string; deviceId: string; scopes: string[];
  idleExpiresAt: string; absoluteExpiresAt: string; publicDevice: boolean;
}
export interface WebSession { user: AuthUser; session: AuthSession; csrfToken: string; }

export interface DeviceInstallation {
  id: string; externalDeviceId: string; deviceGroupId?: string | null;
  deviceName: string; appId: string; platform: string; status: string;
  clientVersion?: string | null; firstSeenAt: string; lastSeenAt: string;
  lastLoginAt?: string | null; lastSyncAt?: string | null;
  revokedAt?: string | null; current: boolean;
}

export interface ManagedSession {
  id: string; appId: string; deviceId: string; sessionType: string;
  status: string; scopes: string[]; publicDevice: boolean; createdAt: string;
  lastSeenAt: string; idleExpiresAt: string; absoluteExpiresAt: string;
  revokedAt?: string | null; current: boolean;
}

export interface CloudConflict { entityType: EntityType; entityId: string; reason: string; occurredAt: string; }
export interface CloudState {
  cursor: string | null;
  entities: Partial<Record<EntityType, Record<string, JsonEntity>>>;
  conflicts: CloudConflict[];
  lastLoadedAt: string | null;
}

export interface SyncChange {
  changeId: string; entityType: EntityType; entityId: string;
  operation: "upsert" | "delete"; baseServerVersion: string;
  entitySchemaVersion: number; clientModifiedAt: string;
  payload: JsonEntity | null; atomicGroupId: null;
  dependencies: Array<{ entityType: string; entityId: string }>;
}

export interface SearchHit {
  id: string; entityType: EntityType; title: string; subtitle: string;
  updatedAt: string; route: string;
}

export interface SnapshotResponse {
  snapshotId: string;
  snapshotCursor: string;
  items: Array<{ entityType: EntityType; entityId: string; serverVersion: string; payload: JsonEntity }>;
  nextPageToken: string | null;
  completed: boolean;
}

export interface PullResponse {
  changes: Array<{
    entityType: EntityType; entityId: string; operation: "upsert" | "delete";
    serverVersion: string; payload: JsonEntity | null;
  }>;
  nextCursor: string;
  hasMore: boolean;
}

export interface PushResult {
  status: string; changeId?: string; entityType?: string; entityId?: string;
  serverVersion?: string; currentServerVersion?: string; serverEntity?: unknown;
  serverDeleted?: boolean; reason?: string; message?: string;
}

export const REQUESTED_SCOPES = [
  "account:read", "account:write", "devices:read", "devices:write",
  "sessions:read", "sessions:write", "sync:read", "sync:write",
  "finance:read", "finance:write", "notes:read", "notes:write",
  "english:read", "english:write", "files:read", "files:write",
] as const;

export const EMPTY_CLOUD_STATE: CloudState = { cursor: null, entities: {}, conflicts: [], lastLoadedAt: null };

export function clone<T>(value: T): T {
  if (typeof structuredClone === "function") return structuredClone(value);
  return JSON.parse(JSON.stringify(value)) as T;
}

export function uuid(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") return crypto.randomUUID();
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (character) => {
    const random = Math.floor(Math.random() * 16);
    return (character === "x" ? random : (random & 0x3) | 0x8).toString(16);
  });
}

export function baseMeta(userId: string, deviceId: string, id = uuid(), now = new Date().toISOString()): EntityMeta {
  return { id, userId, createdAt: now, updatedAt: now, localVersion: 1, serverVersion: null, modifiedByDevice: deviceId, deletedAt: null };
}

export function localDate(date = new Date()): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

export function amountToCents(value: string): number {
  const normalized = value.trim().replace(/,/g, "");
  if (!/^-?\d+(\.\d{0,2})?$/.test(normalized)) throw new Error("金额格式无效，最多保留两位小数");
  const negative = normalized.startsWith("-");
  const [whole, fraction = ""] = (negative ? normalized.slice(1) : normalized).split(".");
  const cents = Number(whole) * 100 + Number(fraction.padEnd(2, "0"));
  if (!Number.isSafeInteger(cents)) throw new Error("金额超出安全范围");
  return negative ? -cents : cents;
}

export function formatMoney(cents: number, currency = "CNY", masked = false): string {
  if (masked) return "••••";
  return new Intl.NumberFormat("zh-CN", { style: "currency", currency, maximumFractionDigits: 2 }).format(cents / 100);
}

export function entityText(entity: JsonEntity, key: string): string {
  return typeof entity[key] === "string" ? String(entity[key]) : "";
}
