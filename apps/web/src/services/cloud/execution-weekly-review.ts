import { baseMeta, type JsonEntity } from "./types";

export interface ExecutionWeeklyReviewInput {
  weekStart: string;
  weekEnd: string;
  plannedCount: number;
  completedCount: number;
  completionRate: number;
  plannedMinutes: number;
  actualMinutes: number;
  overdueTaskCount: number;
  overdueOccurrenceCount: number;
  note?: string;
}

export function createExecutionWeeklyReview(
  userId: string,
  deviceId: string,
  input: ExecutionWeeklyReviewInput,
  id?: string,
): JsonEntity {
  if (!input.weekStart || !input.weekEnd || input.weekStart > input.weekEnd) {
    throw new Error("周复盘日期范围无效");
  }
  return {
    meta: baseMeta(userId, deviceId, id),
    weekStart: input.weekStart,
    weekEnd: input.weekEnd,
    plannedCount: Math.max(0, Math.trunc(input.plannedCount)),
    completedCount: Math.max(0, Math.trunc(input.completedCount)),
    completionRate: Math.max(0, Math.min(100, input.completionRate)),
    plannedMinutes: Math.max(0, Math.trunc(input.plannedMinutes)),
    actualMinutes: Math.max(0, Math.trunc(input.actualMinutes)),
    overdueTaskCount: Math.max(0, Math.trunc(input.overdueTaskCount)),
    overdueOccurrenceCount: Math.max(0, Math.trunc(input.overdueOccurrenceCount)),
    note: input.note?.trim() || null,
  };
}
