"use client";

import { create } from "zustand";
import { db, dayKey, now, seedActivities, uid } from "@/src/db/local";
import type { Activity, ActivityLog, DailyReview, Transaction, ViewId } from "@/src/types";

interface LifeState {
  ready: boolean;
  view: ViewId;
  dark: boolean;
  activities: Activity[];
  logs: ActivityLog[];
  transactions: Transaction[];
  reviews: DailyReview[];
  syncState: "synced" | "syncing" | "offline";
  timer: { activityId: string; startedAt: number | null; accumulatedSeconds: number } | null;
  setView: (view: ViewId) => void;
  toggleDark: () => void;
  initialize: () => Promise<void>;
  addLog: (activityId: string, value?: number, status?: ActivityLog["status"], metadata?: ActivityLog["metadata"]) => Promise<void>;
  addTransaction: (data: Pick<Transaction, "type" | "amount" | "category" | "account" | "note">) => Promise<void>;
  saveReview: (data: Pick<DailyReview, "energy" | "mood" | "bestThing" | "problem" | "tomorrowPriority" | "note">) => Promise<void>;
  addActivity: (data: Pick<Activity, "name" | "type" | "unit" | "normalTarget" | "targetPeriod">) => Promise<void>;
  updateActivity: (id: string, data: Partial<Pick<Activity, "name" | "type" | "unit" | "minimumTarget" | "normalTarget" | "targetPeriod">>) => Promise<void>;
  archiveActivity: (id: string) => Promise<void>;
  startTimer: (activityId: string) => void;
  pauseTimer: () => void;
  finishTimer: () => Promise<void>;
}

const TIMER_KEY = "lifetrace-timer";
const THEME_KEY = "lifetrace-theme";

export const useLifeStore = create<LifeState>((set, get) => ({
  ready: false,
  view: "today",
  dark: false,
  activities: [],
  logs: [],
  transactions: [],
  reviews: [],
  syncState: "synced",
  timer: null,
  setView: (view) => set({ view }),
  toggleDark: () => set((s) => {
    const dark = !s.dark;
    localStorage.setItem(THEME_KEY, dark ? "dark" : "light");
    return { dark };
  }),
  initialize: async () => {
    if ((await db.activities.count()) === 0) await db.activities.bulkAdd(seedActivities);
    const [activities, logs, transactions, reviews] = await Promise.all([
      db.activities.toArray().then((items) => items.filter((item) => !item.isArchived)),
      db.activityLogs.toArray(),
      db.transactions.toArray(),
      db.dailyReviews.toArray(),
    ]);
    const savedTimer = localStorage.getItem(TIMER_KEY);
    set({ activities, logs, transactions, reviews, ready: true, dark: localStorage.getItem(THEME_KEY) === "dark", timer: savedTimer ? JSON.parse(savedTimer) : null, syncState: navigator.onLine ? "synced" : "offline" });
    window.addEventListener("online", () => set({ syncState: "syncing" }) || setTimeout(() => set({ syncState: "synced" }), 900));
    window.addEventListener("offline", () => set({ syncState: "offline" }));
  },
  addLog: async (activityId, value, status = "completed", metadata) => {
    const stamp = now();
    const log: ActivityLog = { id: uid(), userId: "local-user", activityId, value, status, metadata, createdAt: stamp, updatedAt: stamp };
    await db.activityLogs.add(log);
    set({ logs: [...get().logs, log] });
  },
  addTransaction: async (data) => {
    const stamp = now();
    const transaction: Transaction = { id: uid(), userId: "local-user", occurredAt: stamp, createdAt: stamp, updatedAt: stamp, ...data };
    await db.transactions.add(transaction);
    set({ transactions: [transaction, ...get().transactions] });
  },
  saveReview: async (data) => {
    const date = dayKey();
    const old = get().reviews.find((item) => item.reviewDate === date);
    const stamp = now();
    const review: DailyReview = { id: old?.id ?? uid(), userId: "local-user", reviewDate: date, createdAt: old?.createdAt ?? stamp, updatedAt: stamp, ...data };
    await db.dailyReviews.put(review);
    set({ reviews: [...get().reviews.filter((item) => item.reviewDate !== date), review] });
  },
  addActivity: async (data) => {
    const stamp = now();
    const activity: Activity = { id: uid(), userId: "local-user", isArchived: false, createdAt: stamp, updatedAt: stamp, ...data };
    await db.activities.add(activity);
    set({ activities: [...get().activities, activity] });
  },
  updateActivity: async (id, data) => {
    const updatedAt = now();
    await db.activities.update(id, { ...data, updatedAt });
    set({ activities: get().activities.map((item) => item.id === id ? { ...item, ...data, updatedAt } : item) });
  },
  archiveActivity: async (id) => {
    const updatedAt = now();
    await db.activities.update(id, { isArchived: true, updatedAt });
    set({ activities: get().activities.filter((item) => item.id !== id) });
  },
  startTimer: (activityId) => {
    const current = get().timer;
    const timer = current?.activityId === activityId
      ? { ...current, startedAt: current.startedAt ?? Date.now() }
      : { activityId, startedAt: Date.now(), accumulatedSeconds: 0 };
    localStorage.setItem(TIMER_KEY, JSON.stringify(timer));
    set({ timer });
  },
  pauseTimer: () => {
    const current = get().timer;
    if (!current?.startedAt) return;
    const timer = { ...current, accumulatedSeconds: current.accumulatedSeconds + Math.floor((Date.now() - current.startedAt) / 1000), startedAt: null };
    localStorage.setItem(TIMER_KEY, JSON.stringify(timer));
    set({ timer });
  },
  finishTimer: async () => {
    const current = get().timer;
    if (!current) return;
    const seconds = current.accumulatedSeconds + (current.startedAt ? Math.floor((Date.now() - current.startedAt) / 1000) : 0);
    const minutes = Math.max(1, Math.round(seconds / 60));
    await get().addLog(current.activityId, minutes);
    localStorage.removeItem(TIMER_KEY);
    set({ timer: null });
  },
}));
