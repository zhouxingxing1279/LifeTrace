export type ExecutionProject = {
  id: string;
  userId: string;
  name: string;
  description?: string | null;
  status: "active" | "completed" | "archived" | "cancelled";
  color?: string | null;
  icon?: string | null;
  sortOrder: number;
  version: number;
  createdAt: string;
  updatedAt: string;
};

export type ExecutionTaskStatus = "todo" | "in_progress" | "waiting" | "done" | "cancelled";
export type ExecutionTaskPriority = "low" | "normal" | "high" | "urgent";

export type ExecutionTask = {
  id: string;
  userId: string;
  projectId?: string | null;
  parentTaskId?: string | null;
  title: string;
  description?: string | null;
  status: ExecutionTaskStatus;
  priority: ExecutionTaskPriority;
  estimatedMinutes?: number | null;
  actualMinutes?: number | null;
  dueAt?: string | null;
  scheduledStartAt?: string | null;
  scheduledEndAt?: string | null;
  timezone?: string | null;
  context?: string | null;
  completedAt?: string | null;
  cancelledAt?: string | null;
  version: number;
  createdAt: string;
  updatedAt: string;
};

export type CalendarEvent = {
  id: string;
  userId: string;
  title: string;
  description?: string | null;
  isAllDay: boolean;
  startAt?: string | null;
  endAt?: string | null;
  startLocalDate?: string | null;
  endLocalDate?: string | null;
  timezone?: string | null;
  status: string;
  recurrenceRuleId?: string | null;
  sourceTaskId?: string | null;
  version: number;
  createdAt: string;
  updatedAt: string;
};

export type WaitingItem = {
  id: string;
  userId: string;
  title: string;
  description?: string | null;
  status: "open" | "resolved" | "cancelled";
  waitingFor: string;
  expectedAt?: string | null;
  followUpAt?: string | null;
  resolvedAt?: string | null;
  resolutionSummary?: string | null;
  sourceTaskId?: string | null;
  version: number;
  createdAt: string;
  updatedAt: string;
};

export type Memo = {
  id: string;
  userId: string;
  content: string;
  plainText: string;
  isPinned: boolean;
  status: "active" | "archived";
  archivedAt?: string | null;
  context?: string | null;
  version: number;
  createdAt: string;
  updatedAt: string;
  tags: string[];
};

export type Reminder = {
  id: string;
  userId: string;
  subjectType: "task" | "calendar_event" | "waiting_item" | "memo";
  subjectId: string;
  triggerAt: string;
  timezone?: string | null;
  status: "scheduled" | "fired" | "dismissed" | "cancelled";
  snoozedUntil?: string | null;
  lastFiredAt?: string | null;
  fireKey: string;
  version: number;
  createdAt: string;
  updatedAt: string;
};

export type RecurrenceRule = {
  id: string;
  userId: string;
  frequency: string;
  intervalValue: number;
  weekdaysJson?: string | null;
  monthDay?: number | null;
  timezone?: string | null;
  untilAt?: string | null;
  maxOccurrences?: number | null;
  version: number;
  createdAt: string;
  updatedAt: string;
};

export type TaskBlocker = { taskId: string; title: string; status: string };

export type TaskInput = {
  projectId?: string | null;
  title: string;
  description?: string | null;
  priority?: ExecutionTaskPriority;
  estimatedMinutes?: number | null;
  actualMinutes?: number | null;
  dueAt?: string | null;
  scheduledStartAt?: string | null;
  scheduledEndAt?: string | null;
  timezone?: string | null;
  context?: string | null;
};

export type CalendarTimingInput = {
  isAllDay: boolean;
  startAt?: string | null;
  endAt?: string | null;
  startLocalDate?: string | null;
  endLocalDate?: string | null;
  timezone?: string | null;
};

export type CalendarInput = CalendarTimingInput & {
  title: string;
  description?: string | null;
  sourceTaskId?: string | null;
};

export type WaitingInput = {
  title: string;
  description?: string | null;
  waitingFor: string;
  expectedAt?: string | null;
  followUpAt?: string | null;
  sourceTaskId?: string | null;
};

export type MemoInput = { content: string; context?: string | null; tags: string[] };

export type RecurrenceInput = {
  frequency: string;
  intervalValue?: number | null;
  weekdays?: number[];
  monthDay?: number | null;
  timezone?: string | null;
  untilAt?: string | null;
  maxOccurrences?: number | null;
};

export type ApiErrorPayload = { error?: string; code?: string };

async function request<T>(url: string, init?: RequestInit): Promise<T> {
  const response = await fetch(url, init);
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
    const error = payload as ApiErrorPayload | string | null;
    const message = typeof error === "string" ? error : error?.error;
    throw new Error(message || `执行服务请求失败（${response.status}）`);
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

export function localDateTimeToRfc3339(value: string): string | null {
  if (!value.trim()) return null;
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return null;
  return date.toISOString();
}

export function rfc3339ToLocalDateTime(value?: string | null): string {
  if (!value) return "";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "";
  const offset = date.getTimezoneOffset() * 60_000;
  return new Date(date.getTime() - offset).toISOString().slice(0, 16);
}

export function browserTimezone(): string {
  return Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC";
}

export const executionApi = {
  projects: {
    list: () => request<ExecutionProject[]>("/api/execution/projects"),
    create: (input: Omit<Partial<ExecutionProject>, "id"> & { name: string }) =>
      request<ExecutionProject>("/api/execution/projects", json("POST", input)),
    update: (id: string, input: Omit<Partial<ExecutionProject>, "id"> & { name: string }) =>
      request<ExecutionProject>(`/api/execution/projects/${encodeURIComponent(id)}`, json("PUT", input)),
    remove: (id: string) => request<{ ok: true }>(`/api/execution/projects/${encodeURIComponent(id)}`, { method: "DELETE" }),
  },
  tasks: {
    list: (options: { status?: string; projectId?: string } = {}) =>
      request<ExecutionTask[]>(query("/api/execution/tasks", options)),
    get: (id: string) => request<ExecutionTask>(`/api/execution/tasks/${encodeURIComponent(id)}`),
    create: (input: TaskInput) => request<ExecutionTask>("/api/execution/tasks", json("POST", input)),
    update: (id: string, input: TaskInput) =>
      request<ExecutionTask>(`/api/execution/tasks/${encodeURIComponent(id)}`, json("PUT", input)),
    setStatus: (id: string, status: ExecutionTaskStatus) =>
      request<ExecutionTask>(`/api/execution/tasks/${encodeURIComponent(id)}/status`, json("PUT", { status })),
    remove: (id: string) => request<{ ok: true }>(`/api/execution/tasks/${encodeURIComponent(id)}`, { method: "DELETE" }),
    subtasks: (id: string) => request<ExecutionTask[]>(`/api/execution/tasks/${encodeURIComponent(id)}/subtasks`),
    addSubtask: (id: string, input: TaskInput) =>
      request<ExecutionTask>(`/api/execution/tasks/${encodeURIComponent(id)}/subtasks`, json("POST", input)),
    dependencies: (id: string) => request<Array<{ id: string; taskId: string; dependsOnTaskId: string }>>(`/api/execution/tasks/${encodeURIComponent(id)}/dependencies`),
    addDependency: (id: string, dependsOnTaskId: string) =>
      request<unknown>(`/api/execution/tasks/${encodeURIComponent(id)}/dependencies`, json("POST", { dependsOnTaskId })),
    removeDependency: (id: string, dependsOnTaskId: string) =>
      request<{ ok: true }>(`/api/execution/tasks/${encodeURIComponent(id)}/dependencies/${encodeURIComponent(dependsOnTaskId)}`, { method: "DELETE" }),
    blockers: (id: string) => request<TaskBlocker[]>(`/api/execution/tasks/${encodeURIComponent(id)}/blockers`),
    recurrence: (id: string) => request<RecurrenceRule | null>(`/api/execution/tasks/${encodeURIComponent(id)}/recurrence`),
    setRecurrence: (id: string, input: RecurrenceInput) =>
      request<RecurrenceRule>(`/api/execution/tasks/${encodeURIComponent(id)}/recurrence`, json("PUT", input)),
    clearRecurrence: (id: string) => request<{ ok: true }>(`/api/execution/tasks/${encodeURIComponent(id)}/recurrence`, { method: "DELETE" }),
    schedule: (id: string, timing: CalendarTimingInput) =>
      request<CalendarEvent>(`/api/execution/tasks/${encodeURIComponent(id)}/schedule`, json("POST", { timing })),
  },
  calendar: {
    list: (options: Record<string, string | boolean | undefined> = {}) =>
      request<CalendarEvent[]>(query("/api/execution/calendar-events", options)),
    create: (input: CalendarInput) => request<CalendarEvent>("/api/execution/calendar-events", json("POST", input)),
    update: (id: string, input: CalendarInput) => request<CalendarEvent>(`/api/execution/calendar-events/${encodeURIComponent(id)}`, json("PUT", input)),
    remove: (id: string) => request<{ ok: true }>(`/api/execution/calendar-events/${encodeURIComponent(id)}`, { method: "DELETE" }),
    cancel: (id: string) => request<CalendarEvent>(`/api/execution/calendar-events/${encodeURIComponent(id)}/cancel`, json("POST", {})),
    move: (id: string, timing: CalendarTimingInput) => request<CalendarEvent>(`/api/execution/calendar-events/${encodeURIComponent(id)}/move`, json("PUT", timing)),
    conflicts: (timing: CalendarTimingInput, excludeEventId?: string) =>
      request<Array<{ eventId: string; occurrenceId?: string | null; title: string; isAllDay: boolean }>>("/api/execution/calendar-conflicts", json("POST", { timing, excludeEventId })),
    recurrence: (id: string) => request<RecurrenceRule | null>(`/api/execution/calendar-events/${encodeURIComponent(id)}/recurrence`),
    setRecurrence: (id: string, input: RecurrenceInput) =>
      request<RecurrenceRule>(`/api/execution/calendar-events/${encodeURIComponent(id)}/recurrence`, json("PUT", input)),
    clearRecurrence: (id: string) =>
      request<{ ok: true }>(`/api/execution/calendar-events/${encodeURIComponent(id)}/recurrence`, { method: "DELETE" }),
  },
  waiting: {
    list: (options: { view?: string; now?: string } = {}) => request<WaitingItem[]>(query("/api/execution/waiting-items", options)),
    create: (input: WaitingInput) => request<WaitingItem>("/api/execution/waiting-items", json("POST", input)),
    update: (id: string, input: WaitingInput) => request<WaitingItem>(`/api/execution/waiting-items/${encodeURIComponent(id)}`, json("PUT", input)),
    resolve: (id: string, resolutionSummary?: string) => request<WaitingItem>(`/api/execution/waiting-items/${encodeURIComponent(id)}/resolve`, json("POST", { resolutionSummary })),
    cancel: (id: string) => request<WaitingItem>(`/api/execution/waiting-items/${encodeURIComponent(id)}/cancel`, json("POST", {})),
    remove: (id: string) => request<{ ok: true }>(`/api/execution/waiting-items/${encodeURIComponent(id)}`, { method: "DELETE" }),
    convertToTask: (id: string, input: Partial<TaskInput> & { title?: string; resolveSource?: boolean }) =>
      request<ExecutionTask>(`/api/execution/waiting-items/${encodeURIComponent(id)}/convert-to-task`, json("POST", input)),
  },
  memos: {
    list: (options: { q?: string; status?: string; pinned?: boolean; tag?: string } = {}) => request<Memo[]>(query("/api/execution/memos", options)),
    create: (input: MemoInput) => request<Memo>("/api/execution/memos", json("POST", input)),
    update: (id: string, input: MemoInput) => request<Memo>(`/api/execution/memos/${encodeURIComponent(id)}`, json("PUT", input)),
    pin: (id: string, pinned: boolean) => request<Memo>(`/api/execution/memos/${encodeURIComponent(id)}/pin`, json("PUT", { pinned })),
    archive: (id: string) => request<Memo>(`/api/execution/memos/${encodeURIComponent(id)}/archive`, json("POST", {})),
    restore: (id: string) => request<Memo>(`/api/execution/memos/${encodeURIComponent(id)}/restore`, json("POST", {})),
    remove: (id: string) => request<{ ok: true }>(`/api/execution/memos/${encodeURIComponent(id)}`, { method: "DELETE" }),
    convertToTask: (id: string, input: Partial<TaskInput> & { title?: string }) => request<ExecutionTask>(`/api/execution/memos/${encodeURIComponent(id)}/convert-to-task`, json("POST", input)),
    convertToCalendar: (id: string, input: { title?: string; description?: string; timing: CalendarTimingInput }) => request<CalendarEvent>(`/api/execution/memos/${encodeURIComponent(id)}/convert-to-calendar`, json("POST", input)),
    convertToWaiting: (id: string, input: { title?: string; description?: string; waitingFor: string; expectedAt?: string | null; followUpAt?: string | null }) => request<WaitingItem>(`/api/execution/memos/${encodeURIComponent(id)}/convert-to-waiting`, json("POST", input)),
  },
  reminders: {
    list: (subjectType: Reminder["subjectType"], subjectId: string) => request<Reminder[]>(query("/api/execution/reminders", { subjectType, subjectId })),
    due: (now = new Date().toISOString(), limit = 100) => request<Reminder[]>(query("/api/execution/reminders/due", { now, limit })),
    create: (input: { subjectType: Reminder["subjectType"]; subjectId: string; triggerAt: string; timezone?: string }) => request<Reminder>("/api/execution/reminders", json("POST", input)),
    update: (id: string, input: { triggerAt: string; timezone?: string }) => request<Reminder>(`/api/execution/reminders/${encodeURIComponent(id)}`, json("PUT", input)),
    snooze: (id: string, untilAt: string) => request<Reminder>(`/api/execution/reminders/${encodeURIComponent(id)}/snooze`, json("POST", { untilAt })),
    dismiss: (id: string) => request<Reminder>(`/api/execution/reminders/${encodeURIComponent(id)}/dismiss`, json("POST", {})),
    cancel: (id: string) => request<Reminder>(`/api/execution/reminders/${encodeURIComponent(id)}/cancel`, json("POST", {})),
    remove: (id: string) => request<{ ok: true }>(`/api/execution/reminders/${encodeURIComponent(id)}`, { method: "DELETE" }),
  },
};
