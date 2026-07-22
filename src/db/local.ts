import Dexie, { type EntityTable } from "dexie";
import type { Activity, ActivityLog, DailyReview, Transaction } from "@/src/types";

export class LifeTraceDB extends Dexie {
  activities!: EntityTable<Activity, "id">;
  activityLogs!: EntityTable<ActivityLog, "id">;
  transactions!: EntityTable<Transaction, "id">;
  dailyReviews!: EntityTable<DailyReview, "id">;

  constructor() {
    super("lifetrace-local");
    this.version(1).stores({
      activities: "id, userId, type, updatedAt, isArchived",
      activityLogs: "id, userId, activityId, createdAt, updatedAt",
      transactions: "id, userId, type, occurredAt, updatedAt",
      dailyReviews: "id, userId, &reviewDate, updatedAt",
    });
  }
}

export const db = new LifeTraceDB();
export const now = () => new Date().toISOString();
export const uid = () => crypto.randomUUID();
export const dayKey = (date = new Date()) => date.toISOString().slice(0, 10);

export const seedActivities: Activity[] = [
  { id: "piano", userId: "local-user", name: "钢琴练习", type: "duration", unit: "分钟", minimumTarget: 10, normalTarget: 30, targetPeriod: "daily", icon: "music", isArchived: false, createdAt: now(), updatedAt: now() },
  { id: "fitness", userId: "local-user", name: "健身", type: "weekly", unit: "次", normalTarget: 3, targetPeriod: "weekly", icon: "fitness", isArchived: false, createdAt: now(), updatedAt: now() },
  { id: "english", userId: "local-user", name: "英语学习", type: "duration", unit: "分钟", minimumTarget: 10, normalTarget: 30, targetPeriod: "daily", icon: "book", isArchived: false, createdAt: now(), updatedAt: now() },
  { id: "control", userId: "local-user", name: "行为管理", type: "control", unit: "状态", targetPeriod: "daily", icon: "shield", isArchived: false, createdAt: now(), updatedAt: now() },
];

