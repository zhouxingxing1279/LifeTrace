import { browserTimezone, type ExecutionTask, type TaskInput, type WaitingItem } from "@/src/services/executionApi";

export function preserveTaskUpdateFields(task: ExecutionTask, input: TaskInput): TaskInput {
  return {
    ...input,
    actualMinutes: task.actualMinutes ?? null,
    scheduledStartAt: task.scheduledStartAt ?? null,
    scheduledEndAt: task.scheduledEndAt ?? null,
    timezone: input.timezone || task.timezone || browserTimezone(),
  };
}

export function waitingToTaskInput(item: WaitingItem): TaskInput & { resolveSource: true } {
  return {
    title: item.title,
    description: item.description || null,
    priority: "normal",
    dueAt: item.expectedAt || null,
    timezone: browserTimezone(),
    context: item.waitingFor ? `等待：${item.waitingFor}` : null,
    resolveSource: true,
  };
}

export function normalizeWeekdays(values: number[]): number[] {
  return [...new Set(values)]
    .filter((value) => Number.isInteger(value) && value >= 1 && value <= 7)
    .sort((a, b) => a - b);
}
