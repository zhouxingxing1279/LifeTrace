import { baseMeta, localDate, type JsonEntity } from "./types";

export type ExecutionTaskStatus = "todo" | "in_progress" | "waiting" | "done" | "cancelled";
export type ExecutionTaskPriority = "low" | "normal" | "high" | "urgent";

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
    timezone: browserTimezone(), status: "active", recurrenceRuleId: null,
    sourceTaskId: input.sourceTaskId ?? null,
  };
}

export function createExecutionCompletionResult(userId: string, deviceId: string, taskId: string, actualMinutes: number | null = null, summary = ""): JsonEntity {
  return {
    meta: baseMeta(userId, deviceId), taskId, summary: summary.trim() || null,
    completedAt: new Date().toISOString(), actualMinutes,
  };
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
