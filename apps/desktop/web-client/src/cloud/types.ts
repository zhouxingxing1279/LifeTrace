export const APP_VERSION = "0.2.1";
export const APP_ID = "lifetrace-web";
export const PROTOCOL_VERSION = 1;
export const SCHEMA_VERSION = 1;

/**
 * Every cloud-syncable LifeTrace domain used by the browser application.
 * Photos, the encrypted local album, credentials and local import uploads are
 * deliberately absent because the contract registry marks them device-local or
 * secret-local-only.
 */
export const ENTITY_TYPES = [
  "finance.account", "finance.category", "finance.transaction", "finance.transaction_evidence",
  "habit.activity", "habit.log", "review.daily",
  "note.folder", "note.note", "note.tag", "note.tag_relation", "note.relation", "note.revision",
  "english.article", "english.highlight", "english.note", "english.learning_record",
  "english.vocabulary", "english.vocabulary_occurrence", "english.vocabulary_review_state",
  "workout.import", "workout.workout", "workout.exercise", "workout.set", "workout.training_note",
  "execution.goal", "execution.project", "execution.recurrence_rule", "execution.task", "execution.task_dependency",
  "execution.task_occurrence", "execution.waiting_item", "execution.calendar_event",
  "execution.calendar_occurrence", "execution.memo", "execution.memo_tag",
  "execution.memo_tag_relation", "execution.reminder", "execution.completion_result",
  "execution.entity_link",
  "file.metadata", "entity.link", "user.preference",
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
  payload: JsonEntity | null; atomicGroupId: string | null;
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

export interface BeeCountIntegrationStatus {
  enabled: boolean;
  readOnly: true;
  source: "beecount-cloud";
  upstreamReachable: boolean;
  upstreamVersion?: unknown;
}

export interface BeeCountLedger {
  id: string;
  sourceId: string;
  name: string;
  currency: string;
  monthStartDay: number;
  transactionCount: number;
  incomeTotalCents: number;
  expenseTotalCents: number;
  balanceCents: number;
  updatedAt?: string | null;
  role?: string;
  isShared?: boolean;
  memberCount?: number;
  readOnly: true;
}

export interface BeeCountLedgerList {
  source: "beecount-cloud";
  readOnly: true;
  items: BeeCountLedger[];
  fetchedAt: string;
}

export interface BeeCountLedgerSnapshot {
  source: "beecount-cloud";
  readOnly: true;
  fetchedAt: string;
  ledger: BeeCountLedger;
  transactions: { items: BeeCountTransaction[]; total: number; limit: number; offset: number };
  accounts: BeeCountAccount[];
  categories: BeeCountCategory[];
  tags: BeeCountTag[];
  budgets: BeeCountBudget[];
}

export interface BeeCountTransaction {
  id: string;
  externalTransactionId: string;
  transactionType: string;
  amountCents: number;
  nativeAmountCents?: number | null;
  currency: string;
  occurredAt: string;
  localDate?: string | null;
  status: "confirmed";
  sourceType: "beecount-cloud";
  note?: string | null;
  ledgerId?: string | null;
  ledgerName?: string | null;
  accountId?: string | null;
  toAccountId?: string | null;
  categoryId?: string | null;
  accountName?: string | null;
  fromAccountName?: string | null;
  toAccountName?: string | null;
  categoryName?: string | null;
  tags: string[];
  tagIds: string[];
  attachments: Array<Record<string, unknown>>;
  excludeFromStats: boolean;
  excludeFromBudget: boolean;
  readOnly: true;
}

export interface BeeCountAccount {
  id: string;
  sourceId: string;
  name: string;
  accountType?: string | null;
  currency?: string | null;
  openingBalanceCents?: number | null;
  balanceCents?: number | null;
  incomeTotalCents?: number | null;
  expenseTotalCents?: number | null;
  transactionCount?: number | null;
  hidden?: boolean;
  note?: string | null;
  source: "beecount-cloud";
  readOnly: true;
}

export interface BeeCountCategory {
  id: string;
  sourceId: string;
  name: string;
  categoryType: string;
  level?: number | null;
  sortOrder?: number | null;
  icon?: string | null;
  parentName?: string | null;
  transactionCount?: number | null;
  source: "beecount-cloud";
  readOnly: true;
}

export interface BeeCountTag {
  id: string;
  sourceId: string;
  name: string;
  color?: string | null;
  transactionCount?: number | null;
  incomeTotalCents?: number | null;
  expenseTotalCents?: number | null;
  source: "beecount-cloud";
  readOnly: true;
}

export interface BeeCountBudget {
  id: string;
  sourceId: string;
  budgetType: string;
  categoryId?: string | null;
  categoryName?: string | null;
  amountCents: number;
  period: string;
  startDay: number;
  enabled: boolean;
  source: "beecount-cloud";
  readOnly: true;
}

export const REQUESTED_SCOPES = [
  "account:read", "account:write", "devices:read", "devices:write",
  "sessions:read", "sessions:write", "sync:read", "sync:write",
  "finance:read", "finance:write", "notes:read", "notes:write",
  "english:read", "english:write", "habits:read", "habits:write",
  "reviews:read", "reviews:write", "workouts:read", "workouts:write",
  "execution:read", "execution:write", "files:read", "files:write",
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

export function entityNumber(entity: JsonEntity, key: string): number {
  return typeof entity[key] === "number" ? Number(entity[key]) : 0;
}
