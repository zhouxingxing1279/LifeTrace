import {
  browserTimezone,
  createExecutionEntityLink,
  createExecutionTask,
  normalizeWeekdays,
  recurrenceWeekdays,
  type ExecutionTaskInput,
} from "./execution";
import { baseMeta, localDate, type JsonEntity } from "./types";

export interface ExecutionWaitingInput {
  title: string;
  description?: string;
  waitingFor: string;
  expectedAt?: string | null;
  followUpAt?: string | null;
  sourceTaskId?: string | null;
}

export type ExecutionReminderSubject = "task" | "calendar_event" | "waiting_item" | "memo";

export function createExecutionWaitingItem(
  userId: string,
  deviceId: string,
  input: ExecutionWaitingInput,
): JsonEntity {
  const title = input.title.trim();
  const waitingFor = input.waitingFor.trim();
  if (!title) throw new Error("请输入等待事项标题");
  if (!waitingFor) throw new Error("请输入等待对象");
  return {
    meta: baseMeta(userId, deviceId),
    title,
    description: input.description?.trim() || null,
    status: "open",
    waitingFor,
    expectedAt: input.expectedAt ?? null,
    followUpAt: input.followUpAt ?? null,
    resolvedAt: null,
    resolutionSummary: null,
    sourceTaskId: input.sourceTaskId ?? null,
  };
}

export function resolveExecutionWaitingItem(item: JsonEntity, summary = ""): JsonEntity {
  return {
    ...item,
    status: "resolved",
    resolvedAt: new Date().toISOString(),
    resolutionSummary: summary.trim() || null,
  };
}

export function createWaitingConversionLinks(
  userId: string,
  deviceId: string,
  waitingItemId: string,
  taskId: string,
): [JsonEntity, JsonEntity] {
  return [
    createExecutionEntityLink(userId, deviceId, "waiting_item", waitingItemId, "converted_to", "task", taskId),
    createExecutionEntityLink(userId, deviceId, "task", taskId, "derived_from", "waiting_item", waitingItemId),
  ];
}

export function createExecutionReminder(
  userId: string,
  deviceId: string,
  subjectType: ExecutionReminderSubject,
  subjectId: string,
  triggerAt: string,
): JsonEntity {
  const parsed = new Date(triggerAt);
  if (Number.isNaN(parsed.getTime())) throw new Error("请选择有效提醒时间");
  const normalized = parsed.toISOString();
  return {
    meta: baseMeta(userId, deviceId),
    subjectType,
    subjectId,
    triggerAt: normalized,
    timezone: browserTimezone(),
    status: "scheduled",
    snoozedUntil: null,
    lastFiredAt: null,
    fireKey: `${subjectType}:${subjectId}:${normalized}`,
  };
}

export function reminderEffectiveAt(reminder: JsonEntity): string {
  return typeof reminder.snoozedUntil === "string" && reminder.snoozedUntil
    ? reminder.snoozedUntil
    : typeof reminder.triggerAt === "string"
      ? reminder.triggerAt
      : "";
}

export function reminderIsDue(reminder: JsonEntity, now = new Date()): boolean {
  if (reminder.status !== "scheduled") return false;
  const effective = new Date(reminderEffectiveAt(reminder));
  return !Number.isNaN(effective.getTime()) && effective.getTime() <= now.getTime();
}

export function snoozeExecutionReminder(reminder: JsonEntity, untilAt: string): JsonEntity {
  const parsed = new Date(untilAt);
  if (Number.isNaN(parsed.getTime())) throw new Error("请选择有效稍后提醒时间");
  return { ...reminder, status: "scheduled", snoozedUntil: parsed.toISOString() };
}

export function dismissExecutionReminder(reminder: JsonEntity): JsonEntity {
  return { ...reminder, status: "dismissed", snoozedUntil: null };
}

export function createExecutionSubtask(
  userId: string,
  deviceId: string,
  parent: JsonEntity,
  input: Omit<ExecutionTaskInput, "projectId">,
): JsonEntity {
  const task = createExecutionTask(userId, deviceId, {
    ...input,
    projectId: typeof parent.projectId === "string" ? parent.projectId : null,
  });
  return { ...task, parentTaskId: parent.meta.id };
}

export function createExecutionTaskDependency(
  userId: string,
  deviceId: string,
  taskId: string,
  dependsOnTaskId: string,
): JsonEntity {
  if (!taskId || !dependsOnTaskId || taskId === dependsOnTaskId) throw new Error("任务不能依赖自身");
  return {
    meta: baseMeta(userId, deviceId),
    taskId,
    dependsOnTaskId,
    dependencyType: "finish_before_start",
  };
}

export function dependencyCreatesCycle(
  taskId: string,
  dependsOnTaskId: string,
  dependencies: JsonEntity[],
): boolean {
  if (taskId === dependsOnTaskId) return true;
  const graph = new Map<string, string[]>();
  for (const dependency of dependencies) {
    const source = typeof dependency.taskId === "string" ? dependency.taskId : "";
    const target = typeof dependency.dependsOnTaskId === "string" ? dependency.dependsOnTaskId : "";
    if (!source || !target) continue;
    graph.set(source, [...(graph.get(source) ?? []), target]);
  }
  graph.set(taskId, [...(graph.get(taskId) ?? []), dependsOnTaskId]);
  const seen = new Set<string>();
  const stack = [dependsOnTaskId];
  while (stack.length) {
    const current = stack.pop()!;
    if (current === taskId) return true;
    if (seen.has(current)) continue;
    seen.add(current);
    stack.push(...(graph.get(current) ?? []));
  }
  return false;
}

export function taskBlockers(taskId: string, tasks: JsonEntity[], dependencies: JsonEntity[]): JsonEntity[] {
  const taskById = new Map(tasks.map((task) => [task.meta.id, task]));
  return dependencies
    .filter((dependency) => dependency.taskId === taskId)
    .map((dependency) => taskById.get(String(dependency.dependsOnTaskId ?? "")))
    .filter((task): task is JsonEntity => Boolean(task && task.status !== "done"));
}

export function materializeCalendarOccurrences(
  userId: string,
  deviceId: string,
  event: JsonEntity,
  rule: JsonEntity,
  existing: JsonEntity[],
  startDate = localDate(),
  horizonDays = 60,
): JsonEntity[] {
  const anchor = eventAnchorDate(event);
  const rangeStart = parseLocalDay(startDate);
  if (!anchor || !rangeStart) return [];
  const until = typeof rule.untilAt === "string" && rule.untilAt ? parseLocalDay(rule.untilAt.slice(0, 10)) : null;
  const existingKeys = new Set(
    existing
      .filter((item) => item.eventId === event.meta.id)
      .map((item) => String(item.occurrenceKey ?? "")),
  );
  const maxOccurrences = typeof rule.maxOccurrences === "number" ? Math.max(1, Math.trunc(rule.maxOccurrences)) : null;
  let remaining = maxOccurrences === null ? Number.POSITIVE_INFINITY : Math.max(0, maxOccurrences - existingKeys.size);
  const values: JsonEntity[] = [];
  for (let offset = 0; offset < Math.max(1, horizonDays) && remaining > 0; offset += 1) {
    const candidate = new Date(rangeStart.getFullYear(), rangeStart.getMonth(), rangeStart.getDate() + offset);
    if (candidate < anchor || (until && candidate > until)) continue;
    if (!dateMatchesRecurrence(candidate, anchor, rule)) continue;
    const key = localDate(candidate);
    if (existingKeys.has(key)) continue;
    values.push(createExecutionCalendarOccurrence(userId, deviceId, event, key));
    existingKeys.add(key);
    remaining -= 1;
  }
  return values;
}

export function createExecutionCalendarOccurrence(
  userId: string,
  deviceId: string,
  event: JsonEntity,
  occurrenceKey: string,
): JsonEntity {
  const timing = calendarOccurrenceTiming(event, occurrenceKey);
  return {
    meta: baseMeta(userId, deviceId),
    eventId: event.meta.id,
    occurrenceKey,
    isAllDay: event.isAllDay === true,
    startAt: timing.startAt,
    endAt: timing.endAt,
    startLocalDate: timing.startLocalDate,
    endLocalDate: timing.endLocalDate,
    status: "scheduled",
    titleOverride: null,
    descriptionOverride: null,
  };
}

export function moveCalendarOccurrence(occurrence: JsonEntity, date: string, time: string, durationMinutes: number): JsonEntity {
  if (occurrence.isAllDay === true) {
    const span = localDaySpan(String(occurrence.startLocalDate ?? occurrence.occurrenceKey ?? date), String(occurrence.endLocalDate ?? occurrence.startLocalDate ?? date));
    return {
      ...occurrence,
      startLocalDate: date,
      endLocalDate: addLocalDays(date, span),
      startAt: null,
      endAt: null,
      status: "scheduled",
    };
  }
  const start = new Date(`${date}T${time}:00`);
  if (Number.isNaN(start.getTime())) throw new Error("请选择有效的实例时间");
  const end = new Date(start.getTime() + Math.max(5, durationMinutes || 30) * 60_000);
  return { ...occurrence, startAt: start.toISOString(), endAt: end.toISOString(), status: "scheduled" };
}

function eventAnchorDate(event: JsonEntity): Date | null {
  if (event.isAllDay === true && typeof event.startLocalDate === "string") return parseLocalDay(event.startLocalDate);
  if (typeof event.startAt === "string") {
    const parsed = new Date(event.startAt);
    if (!Number.isNaN(parsed.getTime())) return parseLocalDay(localDate(parsed));
  }
  return parseLocalDay(localDate(new Date(event.meta.createdAt)));
}

function calendarOccurrenceTiming(event: JsonEntity, occurrenceKey: string) {
  if (event.isAllDay === true) {
    const start = typeof event.startLocalDate === "string" ? event.startLocalDate : occurrenceKey;
    const end = typeof event.endLocalDate === "string" ? event.endLocalDate : start;
    const span = localDaySpan(start, end);
    return { startAt: null, endAt: null, startLocalDate: occurrenceKey, endLocalDate: addLocalDays(occurrenceKey, span) };
  }
  const start = typeof event.startAt === "string" ? new Date(event.startAt) : null;
  const end = typeof event.endAt === "string" ? new Date(event.endAt) : null;
  const nextStart = start && !Number.isNaN(start.getTime()) ? combineLocalDateAndTime(occurrenceKey, start) : null;
  const duration = start && end && !Number.isNaN(end.getTime()) ? Math.max(0, end.getTime() - start.getTime()) : 3_600_000;
  return {
    startAt: nextStart?.toISOString() ?? null,
    endAt: nextStart ? new Date(nextStart.getTime() + duration).toISOString() : null,
    startLocalDate: null,
    endLocalDate: null,
  };
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
    const months = (candidate.getFullYear() - anchor.getFullYear()) * 12 + candidate.getMonth() - anchor.getMonth();
    return months >= 0 && months % interval === 0 && candidate.getDate() === Math.max(1, Math.min(31, Number(rule.monthDay ?? anchor.getDate())));
  }
  return false;
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

function localDaySpan(start: string, end: string): number {
  const left = parseLocalDay(start);
  const right = parseLocalDay(end);
  return left && right ? Math.max(0, dayDistance(left, right)) : 0;
}

function addLocalDays(day: string, offset: number): string {
  const date = parseLocalDay(day);
  return date ? localDate(new Date(date.getFullYear(), date.getMonth(), date.getDate() + offset)) : day;
}

export function normalizedRecurrenceWeekdays(values: number[]): number[] {
  return normalizeWeekdays(values);
}
