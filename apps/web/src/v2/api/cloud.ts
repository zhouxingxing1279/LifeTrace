import type {
  SnapshotResponseV1,
  PushRequestV1,
  PushResponseV1,
  SyncChangeV1
} from "../../../../../contracts/typescript/lifetrace-contracts.generated";
import type { WebSessionResponseV1 } from "../../../../../contracts/typescript/lifetrace-auth.generated";
import type {
  FinanceTransaction,
  HabitItem,
  LifeTraceSettings,
  LifeTraceState,
  NoteItem,
  ReadingItem,
  ReviewEntry,
  TaskItem,
  WorkoutItem
} from "../model";
import { initialState, isoDate, newId } from "../model";
import { apiRequest, isAuthenticationError } from "./client";

export interface CloudSession {
  authenticated: boolean;
  user?: { id: string; email: string; displayName?: string | null };
  csrfToken?: string;
  deviceId?: string;
  error?: string;
}

type JsonRecord = Record<string, unknown>;

type SnapshotItem = {
  entityType: string;
  entityId: string;
  serverVersion: string;
  payload: unknown;
};

const ENTITY_TYPES = [
  "execution.task",
  "habit.activity",
  "habit.log",
  "workout.workout",
  "finance.transaction",
  "note.note",
  "english.learning_record",
  "review.daily",
  "user.preference"
];

const SETTINGS_ENTITY_ID = "frontend-v2-settings";
const SETTINGS_PREFERENCE_KEY = "frontend.v2.settings";

function record(value: unknown): JsonRecord {
  return value && typeof value === "object" && !Array.isArray(value) ? value as JsonRecord : {};
}

function text(value: unknown, fallback = ""): string {
  return typeof value === "string" ? value : fallback;
}

function numberValue(value: unknown, fallback = 0): number {
  return typeof value === "number" && Number.isFinite(value) ? value : Number(value) || fallback;
}

function bool(value: unknown, fallback = false): boolean {
  return typeof value === "boolean" ? value : fallback;
}

function stringArray(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((item): item is string => typeof item === "string") : [];
}

function cloneState(state: LifeTraceState): LifeTraceState {
  return structuredClone(state);
}

function stable(value: unknown): string {
  return JSON.stringify(value, Object.keys(record(value)).sort());
}

function equal(a: unknown, b: unknown): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

function key(entityType: string, id: string): string {
  return `${entityType}:${id}`;
}

function mapTask(payload: JsonRecord, id: string): TaskItem {
  const priority = text(payload.priority, "normal");
  return {
    id,
    title: text(payload.title, text(payload.name, "Untitled task")),
    dueDate: text(payload.dueDate, isoDate()),
    project: text(payload.project, text(payload.projectName)),
    priority: priority === "high" || priority === "low" ? priority : "normal",
    completed: bool(payload.completed, text(payload.status) === "completed")
  };
}

function mapHabit(payload: JsonRecord, id: string, completedDates: string[]): HabitItem {
  return {
    id,
    name: text(payload.name, "Habit"),
    targetDays: numberValue(payload.targetDayCount, Array.isArray(payload.targetDays) ? payload.targetDays.length || 7 : 7),
    streak: numberValue(payload.streak, consecutiveStreak(completedDates)),
    completedDates: [...completedDates].sort()
  };
}

function consecutiveStreak(dates: string[]): number {
  const completed = new Set(dates);
  const cursor = new Date();
  let count = 0;
  for (;;) {
    if (!completed.has(isoDate(cursor))) break;
    count += 1;
    cursor.setDate(cursor.getDate() - 1);
  }
  return count;
}

function mapWorkout(payload: JsonRecord, id: string): WorkoutItem {
  return {
    id,
    date: text(payload.localDate, isoDate()),
    title: text(payload.name, "Workout"),
    durationMinutes: Math.round(numberValue(payload.durationSeconds) / 60),
    volume: numberValue(payload.volumeKg)
  };
}

function mapTransaction(payload: JsonRecord, id: string): FinanceTransaction {
  const direction = text(payload.transactionType) === "income" ? "income" : "expense";
  return {
    id,
    date: text(payload.localDate, isoDate()),
    title: text(payload.item, text(payload.merchant, text(payload.note, "Transaction"))),
    category: text(payload.categoryName, "未分类"),
    account: text(payload.accountName, "默认账户"),
    amountCents: Math.abs(numberValue(payload.amountCents)),
    direction
  };
}

function mapNote(payload: JsonRecord, id: string): NoteItem {
  const meta = record(payload.meta);
  return {
    id,
    title: text(payload.title, "未命名笔记"),
    content: text(payload.contentMarkdown, text(payload.contentText)),
    updatedAt: text(meta.updatedAt, new Date().toISOString()),
    pinned: bool(payload.isPinned)
  };
}

function mapReading(payload: JsonRecord, id: string): ReadingItem {
  const completed = text(payload.readingStatus) === "completed" || text(payload.completionStatus) === "completed" || bool(payload.completed);
  return {
    id,
    title: text(payload.title, "Reading item"),
    source: text(payload.source, "LifeTrace Cloud"),
    progress: completed ? 100 : Math.min(100, Math.max(0, numberValue(payload.progress))),
    completed,
    highlights: stringArray(payload.highlights),
    note: text(payload.note, text(payload.summary))
  };
}

function mapReview(payload: JsonRecord): ReviewEntry {
  return {
    date: text(payload.reviewDate, isoDate()),
    bestThing: text(payload.bestThing),
    problem: text(payload.problem),
    tomorrowPriority: text(payload.tomorrowPriority)
  };
}

function mapSettings(payload: JsonRecord): Partial<LifeTraceSettings> {
  if (text(payload.preferenceKey) !== SETTINGS_PREFERENCE_KEY) return {};
  const value = record(payload.value);
  const theme = text(value.theme);
  return {
    theme: theme === "light" || theme === "dark" ? theme : "system",
    reducedMotion: bool(value.reducedMotion),
    accent: "blue"
  };
}

export function stateFromSnapshot(items: SnapshotItem[]): LifeTraceState {
  const state = initialState();
  const habitLogs = new Map<string, string[]>();
  const settings: Partial<LifeTraceSettings> = {};

  for (const item of items) {
    if (item.entityType !== "habit.log") continue;
    const payload = record(item.payload);
    const activityId = text(payload.activityId);
    const date = text(payload.logDate);
    if (!activityId || !date || text(payload.status) === "skipped") continue;
    const values = habitLogs.get(activityId) || [];
    values.push(date);
    habitLogs.set(activityId, values);
  }

  for (const item of items) {
    const payload = record(item.payload);
    if (item.entityType === "execution.task") state.tasks.push(mapTask(payload, item.entityId));
    else if (item.entityType === "habit.activity" && !bool(payload.isArchived)) state.habits.push(mapHabit(payload, item.entityId, habitLogs.get(item.entityId) || []));
    else if (item.entityType === "workout.workout") state.workouts.push(mapWorkout(payload, item.entityId));
    else if (item.entityType === "finance.transaction") state.transactions.push(mapTransaction(payload, item.entityId));
    else if (item.entityType === "note.note" && !bool(payload.isArchived)) state.notes.push(mapNote(payload, item.entityId));
    else if (item.entityType === "english.learning_record") state.reading.push(mapReading(payload, item.entityId));
    else if (item.entityType === "review.daily") state.reviews.push(mapReview(payload));
    else if (item.entityType === "user.preference") Object.assign(settings, mapSettings(payload));
  }

  state.settings = { ...state.settings, ...settings };
  state.tasks.sort((a, b) => b.dueDate.localeCompare(a.dueDate));
  state.workouts.sort((a, b) => b.date.localeCompare(a.date));
  state.transactions.sort((a, b) => b.date.localeCompare(a.date));
  state.notes.sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
  return state;
}

function meta(existing: JsonRecord, id: string, userId: string, serverVersion: string | undefined): JsonRecord {
  const previous = record(existing.meta);
  const now = new Date().toISOString();
  return {
    ...previous,
    id,
    userId,
    createdAt: text(previous.createdAt, now),
    updatedAt: now,
    deletedAt: null,
    localVersion: numberValue(previous.localVersion) + 1,
    serverVersion: serverVersion || null,
    modifiedByDevice: previous.modifiedByDevice ?? null
  };
}

function taskPayload(item: TaskItem, existing: JsonRecord, userId: string, serverVersion?: string): JsonRecord {
  return { ...existing, meta: meta(existing, item.id, userId, serverVersion), title: item.title, dueDate: item.dueDate, project: item.project, priority: item.priority, completed: item.completed, status: item.completed ? "completed" : "active" };
}

function habitPayload(item: HabitItem, existing: JsonRecord, userId: string, serverVersion?: string): JsonRecord {
  return {
    ...existing,
    meta: meta(existing, item.id, userId, serverVersion),
    name: item.name,
    activityType: "completion",
    unit: "completion",
    minimumTarget: null,
    normalTarget: 1,
    targetPeriod: "daily",
    targetDays: [],
    targetDayCount: item.targetDays,
    streak: item.streak,
    icon: null,
    color: null,
    scheduleType: "daily",
    startDate: null,
    checkinMethod: "manual",
    syncSource: null,
    description: null,
    isArchived: false
  };
}

function habitLogPayload(habitId: string, date: string, id: string, existing: JsonRecord, userId: string, serverVersion?: string): JsonRecord {
  return { ...existing, meta: meta(existing, id, userId, serverVersion), activityId: habitId, logDate: date, value: 1, status: "completed", note: null, metadata: null };
}

function workoutPayload(item: WorkoutItem, existing: JsonRecord, userId: string, serverVersion?: string): JsonRecord {
  return {
    ...existing,
    meta: meta(existing, item.id, userId, serverVersion),
    source: "manual",
    sourceId: null,
    name: item.title,
    occurredAt: `${item.date}T12:00:00Z`,
    localDate: item.date,
    durationSeconds: item.durationMinutes * 60,
    exerciseCount: 0,
    setCount: 0,
    plannedSetCount: null,
    volumeKg: item.volume || null,
    caloriesKcal: null,
    status: "completed"
  };
}

function transactionPayload(item: FinanceTransaction, existing: JsonRecord, userId: string, serverVersion?: string): JsonRecord {
  return {
    ...existing,
    meta: meta(existing, item.id, userId, serverVersion),
    transactionType: item.direction,
    amountCents: Math.abs(item.amountCents),
    currency: "CNY",
    accountId: null,
    toAccountId: null,
    categoryId: null,
    counterparty: null,
    merchant: null,
    item: item.title,
    note: null,
    occurredAt: `${item.date}T12:00:00Z`,
    localDate: item.date,
    status: "confirmed",
    sourceType: "lifetrace-web-v2",
    externalTransactionId: null,
    categoryName: item.category,
    accountName: item.account
  };
}

function notePayload(item: NoteItem, existing: JsonRecord, userId: string, serverVersion?: string): JsonRecord {
  return {
    ...existing,
    meta: meta(existing, item.id, userId, serverVersion),
    title: item.title || null,
    noteType: "quick",
    folderId: null,
    contentJson: {},
    contentHtml: "",
    contentText: item.content,
    contentMarkdown: item.content,
    summary: "",
    isPinned: item.pinned,
    isFavorite: false,
    isArchived: false,
    aiSummary: null,
    aiTags: null,
    embeddingStatus: null,
    lastAiProcessedAt: null
  };
}

function readingPayload(item: ReadingItem, existing: JsonRecord, userId: string, serverVersion?: string): JsonRecord {
  const now = new Date().toISOString();
  return {
    ...existing,
    meta: meta(existing, item.id, userId, serverVersion),
    articleId: null,
    recordDate: isoDate(),
    readingTimeSeconds: 0,
    summary: item.note,
    score: null,
    analysisId: null,
    newWords: [],
    completionStatus: item.completed ? "completed" : "reading",
    readingStatus: item.completed ? "completed" : "reading",
    startedAt: existing.startedAt ?? now,
    completedAt: item.completed ? text(existing.completedAt, now) : null,
    title: item.title,
    source: item.source,
    progress: item.progress,
    completed: item.completed,
    highlights: item.highlights,
    note: item.note
  };
}

function reviewPayload(item: ReviewEntry, existing: JsonRecord, userId: string, serverVersion?: string): JsonRecord {
  return {
    ...existing,
    meta: meta(existing, item.date, userId, serverVersion),
    reviewDate: item.date,
    energy: null,
    mood: null,
    completionScore: null,
    bestThing: item.bestThing || null,
    problem: item.problem || null,
    tomorrowPriority: item.tomorrowPriority || null,
    note: null
  };
}

function settingsPayload(settings: LifeTraceSettings, existing: JsonRecord, userId: string, serverVersion?: string): JsonRecord {
  return { ...existing, meta: meta(existing, SETTINGS_ENTITY_ID, userId, serverVersion), preferenceKey: SETTINGS_PREFERENCE_KEY, value: settings };
}

export class CloudStateRepository {
  private sessionValue: WebSessionResponseV1 | null = null;
  private versions = new Map<string, string>();
  private payloads = new Map<string, JsonRecord>();
  private baseline = initialState();
  private saveFlight: Promise<void> = Promise.resolve();

  private client() {
    const session = this.sessionValue;
    if (!session) throw new Error("Web session is not initialized");
    return {
      appId: "web",
      clientVersion: "2.0.0",
      platform: "web",
      protocolVersion: 1,
      schemaVersion: 1,
      deviceId: session.session.deviceId
    };
  }

  async getSession(): Promise<CloudSession> {
    try {
      const session = await apiRequest<WebSessionResponseV1>("/api/v1/web/session");
      this.sessionValue = session;
      return { authenticated: true, user: session.user, csrfToken: session.csrfToken, deviceId: session.session.deviceId };
    } catch (error) {
      if (isAuthenticationError(error)) {
        this.sessionValue = null;
        return { authenticated: false };
      }
      return { authenticated: false, error: error instanceof Error ? error.message : "Cloud session unavailable" };
    }
  }

  async login(email: string, password: string): Promise<CloudSession> {
    const session = await apiRequest<WebSessionResponseV1>("/api/v1/web/session/login", {
      method: "POST",
      body: { email, password, requestedScopes: [], publicDevice: false }
    });
    this.sessionValue = session;
    return { authenticated: true, user: session.user, csrfToken: session.csrfToken, deviceId: session.session.deviceId };
  }

  async logout(): Promise<void> {
    const csrfToken = this.sessionValue?.csrfToken;
    if (this.sessionValue) await apiRequest<unknown>("/api/v1/web/session/logout", { method: "POST", csrfToken });
    this.sessionValue = null;
    this.versions.clear();
    this.payloads.clear();
    this.baseline = initialState();
  }

  async loadState(): Promise<LifeTraceState | null> {
    if (!this.sessionValue) return null;
    const items: SnapshotItem[] = [];
    let snapshotId: string | null = null;
    let pageToken: string | null = null;

    do {
      const response = await apiRequest<SnapshotResponseV1>("/api/v1/sync/snapshot", {
        method: "POST",
        csrfToken: this.sessionValue.csrfToken,
        body: {
          requestId: newId("snapshot"),
          client: this.client(),
          snapshotId,
          pageToken,
          entityTypes: ENTITY_TYPES,
          pageSize: 200
        }
      });
      snapshotId = response.snapshotId;
      pageToken = response.nextPageToken;
      for (const item of response.items as SnapshotItem[]) {
        items.push(item);
        this.versions.set(key(item.entityType, item.entityId), item.serverVersion);
        this.payloads.set(key(item.entityType, item.entityId), record(item.payload));
      }
    } while (pageToken);

    this.baseline = stateFromSnapshot(items);
    return cloneState(this.baseline);
  }

  saveState(next: LifeTraceState): Promise<void> {
    const snapshot = cloneState(next);
    this.saveFlight = this.saveFlight.then(() => this.persist(snapshot));
    return this.saveFlight;
  }

  private change(entityType: string, id: string, operation: "upsert" | "delete", payload: JsonRecord | null): SyncChangeV1 {
    return {
      changeId: newId("change"),
      entityType,
      entityId: id,
      operation,
      baseServerVersion: this.versions.get(key(entityType, id)) || "0",
      entitySchemaVersion: 1,
      clientModifiedAt: new Date().toISOString(),
      payload,
      atomicGroupId: null,
      dependencies: []
    };
  }

  private collectionChanges<T extends { id: string }>(entityType: string, previous: T[], next: T[], payloadFor: (value: T, existing: JsonRecord, userId: string, serverVersion?: string) => JsonRecord): SyncChangeV1[] {
    const changes: SyncChangeV1[] = [];
    const before = new Map(previous.map((value) => [value.id, value]));
    const after = new Map(next.map((value) => [value.id, value]));
    const userId = this.sessionValue!.user.id;

    for (const value of next) {
      if (before.has(value.id) && equal(before.get(value.id), value)) continue;
      const entityKey = key(entityType, value.id);
      changes.push(this.change(entityType, value.id, "upsert", payloadFor(value, this.payloads.get(entityKey) || {}, userId, this.versions.get(entityKey))));
    }
    for (const value of previous) if (!after.has(value.id)) changes.push(this.change(entityType, value.id, "delete", null));
    return changes;
  }

  private habitLogChanges(previous: HabitItem[], next: HabitItem[]): SyncChangeV1[] {
    const userId = this.sessionValue!.user.id;
    const oldLogs = new Set(previous.flatMap((habit) => habit.completedDates.map((date) => `${habit.id}:${date}`)));
    const newLogs = new Set(next.flatMap((habit) => habit.completedDates.map((date) => `${habit.id}:${date}`)));
    const changes: SyncChangeV1[] = [];
    for (const id of newLogs) {
      if (oldLogs.has(id)) continue;
      const separator = id.lastIndexOf(":");
      const habitId = id.slice(0, separator);
      const date = id.slice(separator + 1);
      const entityKey = key("habit.log", id);
      changes.push(this.change("habit.log", id, "upsert", habitLogPayload(habitId, date, id, this.payloads.get(entityKey) || {}, userId, this.versions.get(entityKey))));
    }
    for (const id of oldLogs) if (!newLogs.has(id)) changes.push(this.change("habit.log", id, "delete", null));
    return changes;
  }

  private async persist(next: LifeTraceState): Promise<void> {
    if (!this.sessionValue || equal(this.baseline, next)) return;
    const userId = this.sessionValue.user.id;
    const changes = [
      ...this.collectionChanges("execution.task", this.baseline.tasks, next.tasks, taskPayload),
      ...this.collectionChanges("habit.activity", this.baseline.habits, next.habits, habitPayload),
      ...this.habitLogChanges(this.baseline.habits, next.habits),
      ...this.collectionChanges("workout.workout", this.baseline.workouts, next.workouts, workoutPayload),
      ...this.collectionChanges("finance.transaction", this.baseline.transactions, next.transactions, transactionPayload),
      ...this.collectionChanges("note.note", this.baseline.notes, next.notes, notePayload),
      ...this.collectionChanges("english.learning_record", this.baseline.reading, next.reading, readingPayload)
    ];

    const oldReviews = new Map(this.baseline.reviews.map((item) => [item.date, item]));
    const nextReviews = new Map(next.reviews.map((item) => [item.date, item]));
    for (const item of next.reviews) {
      if (oldReviews.has(item.date) && equal(oldReviews.get(item.date), item)) continue;
      const entityKey = key("review.daily", item.date);
      changes.push(this.change("review.daily", item.date, "upsert", reviewPayload(item, this.payloads.get(entityKey) || {}, userId, this.versions.get(entityKey))));
    }
    for (const item of this.baseline.reviews) if (!nextReviews.has(item.date)) changes.push(this.change("review.daily", item.date, "delete", null));

    if (!equal(this.baseline.settings, next.settings)) {
      const entityKey = key("user.preference", SETTINGS_ENTITY_ID);
      changes.push(this.change("user.preference", SETTINGS_ENTITY_ID, "upsert", settingsPayload(next.settings, this.payloads.get(entityKey) || {}, userId, this.versions.get(entityKey))));
    }

    for (let offset = 0; offset < changes.length; offset += 100) {
      const batch = changes.slice(offset, offset + 100);
      const request: PushRequestV1 = { requestId: newId("push"), client: this.client(), changes: batch };
      const response = await apiRequest<PushResponseV1>("/api/v1/sync/push", { method: "POST", csrfToken: this.sessionValue.csrfToken, body: request });
      for (const result of response.results) {
        if (result.status === "accepted" || result.status === "duplicate") {
          this.versions.set(key(result.entityType, result.entityId), result.serverVersion);
          const sent = batch.find((change) => change.entityType === result.entityType && change.entityId === result.entityId);
          if (sent?.payload) this.payloads.set(key(result.entityType, result.entityId), record(sent.payload));
          else this.payloads.delete(key(result.entityType, result.entityId));
        } else if (result.status === "conflict") {
          throw new Error(`Sync conflict for ${result.entityType}:${result.entityId}`);
        } else {
          throw new Error(`Sync rejected for ${result.entityType}:${result.entityId}: ${result.message}`);
        }
      }
    }
    this.baseline = cloneState(next);
  }
}

export const cloudStateRepository = new CloudStateRepository();
