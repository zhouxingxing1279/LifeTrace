import { baseMeta, localDate, type JsonEntity } from "./types";

export type ExecutionTaskStatus = "todo" | "in_progress" | "waiting" | "done" | "cancelled";
export type ExecutionTaskPriority = "low" | "normal" | "high" | "urgent";
export type ExecutionRecurrenceFrequency = "daily" | "weekly" | "monthly";

export interface ExecutionProjectInput {
  name: string;
  description?: string;
  color?: string | null;
  icon?: string | null;
}

export interface ExecutionTaskInput {
  title: string;
  description?: string;
  projectId?: string | null;
  status?: ExecutionTaskStatus;
  priority?: ExecutionTaskPriority;
  estimatedMinutes?: number | null;
  dueAt?: string | null;
  scheduledStartAt?: string | null;
  scheduledEndAt?: string | null;
  timezone?: string | null;
  context?: string | null;
}

export interface ExecutionCalendarInput {
  title: string;
  description?: string;
  isAllDay?: boolean;
  startAt?: string | null;
  endAt?: string | null;
  startLocalDate?: string | null;
  endLocalDate?: string | null;
  sourceTaskId?: string | null;
  recurrenceRuleId?: string | null;
}

export interface ExecutionRecurrenceInput {
  frequency: ExecutionRecurrenceFrequency;
  intervalValue?: number;
  weekdays?: number[];
  monthDay?: number | null;
  timezone?: string | null;
  untilAt?: string | null;
  maxOccurrences?: number | null;
}

export function browserTimezone(): string {
  return Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC";
}

export function createExecutionProject(userId: string, deviceId: string, input: ExecutionProjectInput): JsonEntity {
  const name = input.name.trim();
  if (!name) throw new Error("请输入计划名称");
  return {
    meta: baseMeta(userId, deviceId), name,
    description: input.description?.trim() || null,
    status: "active", color: input.color ?? "#49715d", icon: input.icon ?? "target", sortOrder: 0,
  };
}

export function createExecutionTask(userId: string, deviceId: string, input: ExecutionTaskInput): JsonEntity {
  const title = input.title.trim();
  if (!title) throw new Error("请输入任务内容");
  return {
    meta: baseMeta(userId, deviceId), projectId: input.projectId ?? null, parentTaskId: null,
    title, description: input.description?.trim() || null,
    status: input.status ?? "todo", priority: input.priority ?? "normal",
    estimatedMinutes: input.estimatedMinutes ?? null, actualMinutes: null,
    dueAt: input.dueAt ?? null, scheduledStartAt: input.scheduledStartAt ?? null,
    scheduledEndAt: input.scheduledEndAt ?? null, timezone: input.timezone ?? browserTimezone(),
    context: input.context?.trim() || null, completedAt: null, cancelledAt: null,
    recurrenceRuleId: null,
  };
}

export function createExecutionMemo(userId: string, deviceId: string, content: string, context: string | null = null): JsonEntity {
  const value = content.trim();
  if (!value) throw new Error("请输入备忘内容");
  return {
    meta: baseMeta(userId, deviceId), content: value, plainText: value,
    isPinned: false, status: "active", archivedAt: null, context, tags: [],
  };
}

export function createExecutionCalendarEvent(userId: string, deviceId: string, input: ExecutionCalendarInput): JsonEntity {
  const title = input.title.trim();
  if (!title) throw new Error("请输入日程名称");
  return {
    meta: baseMeta(userId, deviceId), title, description: input.description?.trim() || null,
    isAllDay: input.isAllDay ?? false, startAt: input.startAt ?? null, endAt: input.endAt ?? null,
    startLocalDate: input.startLocalDate ?? null, endLocalDate: input.endLocalDate ?? null,
    timezone: browserTimezone(), status: "scheduled", recurrenceRuleId: input.recurrenceRuleId ?? null,
    sourceTaskId: input.sourceTaskId ?? null,
  };
}

export function createExecutionRecurrenceRule(userId: string, deviceId: string, input: ExecutionRecurrenceInput, id?: string): JsonEntity {
  const intervalValue = Math.max(1, Math.trunc(input.intervalValue ?? 1));
  const weekdays = normalizeWeekdays(input.weekdays ?? []);
  if (input.frequency === "weekly" && !weekdays.length) throw new Error("每周重复至少选择一个星期");
  const monthDay = input.frequency === "monthly" ? Math.max(1, Math.min(31, Math.trunc(input.monthDay ?? new Date().getDate()))) : null;
  return {
    meta: baseMeta(userId, deviceId, id),
    frequency: input.frequency,
    intervalValue,
    weekdays: input.frequency === "weekly" ? weekdays : [],
    weekdaysJson: input.frequency === "weekly" ? JSON.stringify(weekdays) : null,
    monthDay,
    timezone: input.timezone ?? browserTimezone(),
    untilAt: input.untilAt ?? null,
    maxOccurrences: input.maxOccurrences ? Math.max(1, Math.trunc(input.maxOccurrences)) : null,
  };
}

export function createExecutionTaskOccurrence(
  userId: string,
  deviceId: string,
  task: JsonEntity,
  occurrenceKey: string,
): JsonEntity {
  const timing = occurrenceTiming(task, occurrenceKey);
  return {
    meta: baseMeta(userId, deviceId),
    taskId: task.meta.id,
    occurrenceKey,
    scheduledStartAt: timing.scheduledStartAt,
    scheduledEndAt: timing.scheduledEndAt,
    dueAt: timing.dueAt,
    status: "pending",
    titleOverride: null,
    descriptionOverride: null,
    completedAt: null,
    skippedAt: null,
  };
}

export function createExecutionEntityLink(
  userId: string,
  deviceId: string,
  sourceType: string,
  sourceId: string,
  relationType: "related_to" | "derived_from" | "converted_to" | "attachment" | "reference",
  targetType: string,
  targetId: string,
): JsonEntity {
  return {
    meta: baseMeta(userId, deviceId), sourceType, sourceId, relationType, targetType, targetId,
  };
}

export function createMemoConversionLinks(
  userId: string,
  deviceId: string,
  memoId: string,
  targetType: "task" | "calendar_event" | "waiting_item",
  targetId: string,
): [JsonEntity, JsonEntity] {
  return [
    createExecutionEntityLink(userId, deviceId, "memo", memoId, "converted_to", targetType, targetId),
    createExecutionEntityLink(userId, deviceId, targetType, targetId, "derived_from", "memo", memoId),
  ];
}

export function createExecutionCompletionResult(userId: string, deviceId: string, taskId: string, actualMinutes: number | null = null, summary = ""): JsonEntity {
  return {
    meta: baseMeta(userId, deviceId), taskId, summary: summary.trim() || null,
    completedAt: new Date().toISOString(), actualMinutes,
  };
}

export function normalizeWeekdays(values: number[]): number[] {
  return [...new Set(values)]
    .filter((value) => Number.isInteger(value) && value >= 1 && value <= 7)
    .sort((left, right) => left - right);
}

export function isOpenExecutionTask(task: JsonEntity): boolean {
  return task.status !== "done" && task.status !== "cancelled";
}

export function executionTaskDate(task: JsonEntity): string | null {
  const scheduled = typeof task.scheduledStartAt === "string" ? task.scheduledStartAt : "";
  const due = typeof task.dueAt === "string" ? task.dueAt : "";
  const value = scheduled || due;
  if (!value) return null;
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? value.slice(0, 10) || null : localDate(parsed);
}

export function taskMatchesToday(task: JsonEntity, today = localDate()): boolean {
  return executionTaskDate(task) === today;
}

export function taskIsInbox(task: JsonEntity): boolean {
  if (!isOpenExecutionTask(task)) return false;
  return task.context === "inbox" || (!task.projectId && !task.dueAt && !task.scheduledStartAt);
}

export function taskPriorityLabel(task: JsonEntity): string {
  return ({ low: "低", normal: "普通", high: "高", urgent: "紧急" } as Record<string, string>)[String(task.priority ?? "normal")] ?? "普通";
}

export function recurrenceLabel(rule?: JsonEntity | null): string {
  if (!rule) return "不重复";
  const interval = Math.max(1, Number(rule.intervalValue ?? 1));
  if (rule.frequency === "daily") return interval === 1 ? "每天" : `每 ${interval} 天`;
  if (rule.frequency === "weekly") {
    const weekdays = recurrenceWeekdays(rule).map((day) => "一二三四五六日"[day - 1]).join("、");
    return `${interval === 1 ? "每周" : `每 ${interval} 周`} ${weekdays ? `周${weekdays}` : ""}`.trim();
  }
  if (rule.frequency === "monthly") return `${interval === 1 ? "每月" : `每 ${interval} 月`} ${Number(rule.monthDay ?? 1)} 日`;
  return "重复";
}

export function recurrenceWeekdays(rule: JsonEntity): number[] {
  if (Array.isArray(rule.weekdays)) return normalizeWeekdays(rule.weekdays.filter((value): value is number => typeof value === "number"));
  if (typeof rule.weekdaysJson === "string" && rule.weekdaysJson) {
    try {
      const parsed = JSON.parse(rule.weekdaysJson) as unknown;
      if (Array.isArray(parsed)) return normalizeWeekdays(parsed.filter((value): value is number => typeof value === "number"));
    } catch {
      return [];
    }
  }
  return [];
}

export function materializeTaskOccurrences(
  userId: string,
  deviceId: string,
  task: JsonEntity,
  rule: JsonEntity,
  existing: JsonEntity[],
  startDate = localDate(),
  horizonDays = 30,
): JsonEntity[] {
  const anchor = executionTaskDate(task) ?? localDate(new Date(task.meta.createdAt));
  const rangeStart = parseLocalDay(startDate);
  const anchorDate = parseLocalDay(anchor);
  if (!rangeStart || !anchorDate) return [];
  const until = typeof rule.untilAt === "string" && rule.untilAt ? parseLocalDay(rule.untilAt.slice(0, 10)) : null;
  const existingKeys = new Set(existing.filter((item) => item.taskId === task.meta.id).map((item) => String(item.occurrenceKey ?? "")));
  const maxOccurrences = typeof rule.maxOccurrences === "number" ? Math.max(1, Math.trunc(rule.maxOccurrences)) : null;
  let remaining = maxOccurrences === null ? Number.POSITIVE_INFINITY : Math.max(0, maxOccurrences - existingKeys.size);
  const values: JsonEntity[] = [];
  for (let offset = 0; offset < Math.max(1, horizonDays) && remaining > 0; offset += 1) {
    const candidate = new Date(rangeStart.getFullYear(), rangeStart.getMonth(), rangeStart.getDate() + offset);
    if (candidate < anchorDate || (until && candidate > until)) continue;
    if (!dateMatchesRecurrence(candidate, anchorDate, rule)) continue;
    const key = localDate(candidate);
    if (existingKeys.has(key)) continue;
    values.push(createExecutionTaskOccurrence(userId, deviceId, task, key));
    existingKeys.add(key);
    remaining -= 1;
  }
  return values;
}

function dateMatchesRecurrence(candidate: Date, anchor: Date, rule: JsonEntity): boolean {
  const interval = Math.max(1, Math.trunc(Number(rule.intervalValue ?? 1)));
  if (rule.frequency === "daily") return dayDistance(anchor, candidate) % interval === 0;
  if (rule.frequency === "weekly") {
    const weekdays = recurrenceWeekdays(rule);
    const weekday = candidate.getDay() === 0 ? 7 : candidate.getDay();
    return weekdays.includes(weekday) && Math.floor(dayDistance(startOfWeek(anchor), startOfWeek(candidate)) / 7) % interval === 0;
  }
  if (rule.frequency === "monthly") {
    const monthDistance = (candidate.getFullYear() - anchor.getFullYear()) * 12 + candidate.getMonth() - anchor.getMonth();
    return monthDistance >= 0 && monthDistance % interval === 0 && candidate.getDate() === Math.max(1, Math.min(31, Number(rule.monthDay ?? anchor.getDate())));
  }
  return false;
}

function occurrenceTiming(task: JsonEntity, occurrenceKey: string) {
  const start = typeof task.scheduledStartAt === "string" ? new Date(task.scheduledStartAt) : null;
  const end = typeof task.scheduledEndAt === "string" ? new Date(task.scheduledEndAt) : null;
  const due = typeof task.dueAt === "string" ? new Date(task.dueAt) : null;
  if (start && !Number.isNaN(start.getTime())) {
    const nextStart = combineLocalDateAndTime(occurrenceKey, start);
    const duration = end && !Number.isNaN(end.getTime()) ? Math.max(0, end.getTime() - start.getTime()) : null;
    return {
      scheduledStartAt: nextStart?.toISOString() ?? null,
      scheduledEndAt: nextStart && duration !== null ? new Date(nextStart.getTime() + duration).toISOString() : null,
      dueAt: null,
    };
  }
  if (due && !Number.isNaN(due.getTime())) {
    const nextDue = combineLocalDateAndTime(occurrenceKey, due);
    return { scheduledStartAt: null, scheduledEndAt: null, dueAt: nextDue?.toISOString() ?? null };
  }
  const endOfDay = new Date(`${occurrenceKey}T23:59:00`);
  return { scheduledStartAt: null, scheduledEndAt: null, dueAt: Number.isNaN(endOfDay.getTime()) ? null : endOfDay.toISOString() };
}

function parseLocalDay(value: string): Date | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})/.exec(value);
  if (!match) return null;
  const date = new Date(Number(match[1]), Number(match[2]) - 1, Number(match[3]));
  return Number.isNaN(date.getTime()) ? null : date;
}

function combineLocalDateAndTime(day: string, time: Date): Date | null {
  const date = parseLocalDay(day);
  if (!date) return null;
  date.setHours(time.getHours(), time.getMinutes(), time.getSeconds(), time.getMilliseconds());
  return date;
}

function startOfWeek(value: Date): Date {
  const day = value.getDay() === 0 ? 7 : value.getDay();
  return new Date(value.getFullYear(), value.getMonth(), value.getDate() - day + 1);
}

function dayDistance(left: Date, right: Date): number {
  const start = Date.UTC(left.getFullYear(), left.getMonth(), left.getDate());
  const end = Date.UTC(right.getFullYear(), right.getMonth(), right.getDate());
  return Math.floor((end - start) / 86_400_000);
}
