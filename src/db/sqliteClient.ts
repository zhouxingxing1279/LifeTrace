import type { Activity, ActivityLog, DailyReview, FinanceAccount, Transaction, WorkoutHistory } from "@/src/types";
import { createId } from "@/src/utils/id";

export interface LifeData {
  activities: Activity[];
  logs: ActivityLog[];
  transactions: Transaction[];
  reviews: DailyReview[];
  settings: LifeSettings;
  accounts: FinanceAccount[];
  workoutHistory: WorkoutHistory[];
}

export interface LifeSettings {
  id: "preferences";
  dark: boolean;
  timer: { activityId: string; startedAt: number | null; accumulatedSeconds: number } | null;
  updatedAt: string;
}

export type SQLiteMutation =
  | { operation: "put"; table: "activities" | "logs" | "transactions" | "reviews" | "settings" | "accounts" | "workoutHistory"; value: Activity | ActivityLog | Transaction | DailyReview | LifeSettings | FinanceAccount | WorkoutHistory }
  | { operation: "patch"; table: "activities" | "accounts"; id: string; patch: Record<string, unknown> }
  | { operation: "delete"; table: "transactions" | "accounts" | "workoutHistory"; id: string }
  | { operation: "restore"; data: Omit<LifeData, "settings"> };

const request = async <T>(input: RequestInfo, init?: RequestInit): Promise<T> => {
  const response = await fetch(input, init);
  const payload = await response.json() as T & { error?: string };
  if (!response.ok) throw new Error(payload.error || "SQLite 数据服务暂时不可用");
  return payload;
};

const mutateServerSQLite = (mutation: SQLiteMutation) => request<{ ok: true }>("/api/state", {
  method: "POST",
  headers: { "content-type": "application/json" },
  body: JSON.stringify(mutation),
});

export const loadSQLiteState = () => request<LifeData>("/api/state");

export const mutateSQLite = (mutation: SQLiteMutation) => mutateServerSQLite(mutation);

export const now = () => new Date().toISOString();
export const uid = createId;
export const dayKey = (date = new Date()) => date.toISOString().slice(0, 10);
