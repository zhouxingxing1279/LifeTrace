"use client";

import { create } from "zustand";
import { dayKey, loadSQLiteState, mutateSQLite, now, uid } from "@/src/db/sqliteClient";
import type { Activity, ActivityLog, DailyReview, FinanceAccount, FinanceCategory, Transaction, ViewId, WorkoutHistory } from "@/src/types";

interface LifeState {
  ready: boolean;
  storageError: string | null;
  view: ViewId;
  dark: boolean;
  activities: Activity[];
  logs: ActivityLog[];
  transactions: Transaction[];
  categories: FinanceCategory[];
  reviews: DailyReview[];
  accounts: FinanceAccount[];
  workoutHistory: WorkoutHistory[];
  syncState: "synced" | "syncing" | "offline";
  timer: { activityId: string; startedAt: number | null; accumulatedSeconds: number } | null;
  setView: (view: ViewId) => void;
  toggleDark: () => void;
  initialize: () => Promise<void>;
  addLog: (activityId: string, value?: number, status?: ActivityLog["status"], metadata?: ActivityLog["metadata"], note?: string) => Promise<void>;
  addTransaction: (data: Pick<Transaction, "type" | "amount" | "category" | "account"> & Partial<Pick<Transaction, "note" | "occurredAt" | "accountId" | "toAccount" | "toAccountId" | "counterparty" | "item">>) => Promise<void>;
  saveReview: (data: Pick<DailyReview, "energy" | "mood" | "bestThing" | "problem" | "tomorrowPriority" | "note">) => Promise<void>;
  addActivity: (data: Pick<Activity, "name" | "type" | "unit" | "normalTarget" | "targetPeriod"> & Partial<Pick<Activity, "minimumTarget" | "targetDays" | "icon" | "color" | "scheduleType" | "startDate" | "checkinMethod" | "syncSource" | "description">>) => Promise<void>;
  updateActivity: (id: string, data: Partial<Pick<Activity, "name" | "type" | "unit" | "minimumTarget" | "normalTarget" | "targetPeriod" | "targetDays" | "icon" | "color" | "scheduleType" | "startDate" | "checkinMethod" | "syncSource" | "description">>) => Promise<void>;
  archiveActivity: (id: string) => Promise<void>;
  saveAccount: (data: Partial<FinanceAccount> & Pick<FinanceAccount, "name" | "type" | "color" | "icon">) => Promise<void>;
  deleteAccount: (id: string) => Promise<void>;
  deleteWorkoutHistory: (id: string) => Promise<void>;
  updateTransaction: (id: string, data: Partial<Transaction>) => Promise<void>;
  deleteTransaction: (id: string) => Promise<void>;
  saveCategory: (data: Pick<FinanceCategory, "name" | "type"> & Partial<FinanceCategory>) => Promise<void>;
  archiveCategory: (id: string) => Promise<void>;
  startTimer: (activityId: string) => void;
  pauseTimer: () => void;
  finishTimer: () => Promise<void>;
  restoreBackup: (payload: unknown) => Promise<void>;
}

export const useLifeStore = create<LifeState>((set, get) => ({
  ready: false,
  storageError: null,
  view: "today",
  dark: false,
  activities: [],
  logs: [],
  transactions: [],
  categories: [],
  reviews: [],
  accounts: [],
  workoutHistory: [],
  syncState: "synced",
  timer: null,
  setView: (view) => set({ view }),
  toggleDark: () => set((s) => {
    const dark = !s.dark;
    void mutateSQLite({ operation: "put", table: "settings", value: { id: "preferences", dark, timer: s.timer, updatedAt: now() } }).catch((error) => set({ storageError: error instanceof Error ? error.message : "SQLite 写入失败" }));
    return { dark };
  }),
  initialize: async () => {
    try {
      const { activities, logs, transactions, categories, reviews, settings, accounts, workoutHistory } = await loadSQLiteState();
      set({ activities: activities.filter((item) => !item.isArchived), logs, transactions, categories: categories ?? [], reviews, accounts: accounts.filter(item=>!item.isArchived), workoutHistory, ready: true, storageError: null, dark: settings.dark, timer: settings.timer, syncState: "synced" });
    } catch (error) {
      set({ ready: true, storageError: error instanceof Error ? error.message : "无法连接 SQLite 数据库", syncState: "offline" });
    }
  },
  addLog: async (activityId, value, status = "completed", metadata, note) => {
    const stamp = now();
    const log: ActivityLog = { id: uid(), userId: "local-user", activityId, value, status, metadata, note: note?.trim() || undefined, createdAt: stamp, updatedAt: stamp };
    await mutateSQLite({ operation: "put", table: "logs", value: log });
    set({ logs: [...get().logs, log] });
  },
  addTransaction: async (data) => {
    const stamp = now();
    const transaction: Transaction = { id: uid(), userId: "local-user", occurredAt: data.occurredAt ?? stamp, createdAt: stamp, updatedAt: stamp, ...data };
    await mutateSQLite({ operation: "put", table: "transactions", value: transaction });
    set({ transactions: [transaction, ...get().transactions] });
  },
  saveReview: async (data) => {
    const date = dayKey();
    const old = get().reviews.find((item) => item.reviewDate === date);
    const stamp = now();
    const review: DailyReview = { id: old?.id ?? uid(), userId: "local-user", reviewDate: date, createdAt: old?.createdAt ?? stamp, updatedAt: stamp, ...data };
    await mutateSQLite({ operation: "put", table: "reviews", value: review });
    set({ reviews: [...get().reviews.filter((item) => item.reviewDate !== date), review] });
  },
  addActivity: async (data) => {
    const stamp = now();
    const activity: Activity = { id: uid(), userId: "local-user", isArchived: false, createdAt: stamp, updatedAt: stamp, ...data };
    await mutateSQLite({ operation: "put", table: "activities", value: activity });
    set({ activities: [...get().activities, activity] });
  },
  updateActivity: async (id, data) => {
    const updatedAt = now();
    await mutateSQLite({ operation: "patch", table: "activities", id, patch: { ...data, updatedAt } });
    set({ activities: get().activities.map((item) => item.id === id ? { ...item, ...data, updatedAt } : item) });
  },
  archiveActivity: async (id) => {
    const updatedAt = now();
    await mutateSQLite({ operation: "patch", table: "activities", id, patch: { isArchived: true, updatedAt } });
    set({ activities: get().activities.filter((item) => item.id !== id) });
  },
  saveAccount: async (data) => {
    const stamp=now(); const existing=data.id?get().accounts.find(item=>item.id===data.id):undefined;
    const account:FinanceAccount={id:existing?.id??uid(),userId:"local-user",name:data.name,type:data.type,balance:data.balance??existing?.balance??0,balanceAt:data.balanceAt??existing?.balanceAt,last4:data.last4??existing?.last4,color:data.color,icon:data.icon,isArchived:false,createdAt:existing?.createdAt??stamp,updatedAt:stamp};
    await mutateSQLite({operation:"put",table:"accounts",value:account});
    set({accounts:existing?get().accounts.map(item=>item.id===account.id?account:item):[...get().accounts,account]});
  },
  deleteAccount: async (id) => { await mutateSQLite({operation:"delete",table:"accounts",id}); set({accounts:get().accounts.filter(item=>item.id!==id)}); },
  deleteWorkoutHistory: async (id) => { await mutateSQLite({operation:"delete",table:"workoutHistory",id}); set({workoutHistory:get().workoutHistory.filter(item=>item.id!==id)}); },
  updateTransaction: async (id,data) => { const old=get().transactions.find(item=>item.id===id); if(!old)return; const value={...old,...data,id,updatedAt:now()}; await mutateSQLite({operation:"put",table:"transactions",value}); set({transactions:get().transactions.map(item=>item.id===id?value:item)}); },
  deleteTransaction: async (id) => { await mutateSQLite({operation:"delete",table:"transactions",id}); set({transactions:get().transactions.filter(item=>item.id!==id)}); },
  saveCategory: async (data) => {
    const stamp = now();
    const existing = data.id ? get().categories.find((item) => item.id === data.id) : undefined;
    const category: FinanceCategory = {
      id: existing?.id ?? uid(),
      userId: "local-user",
      name: data.name.trim(),
      type: data.type,
      parentId: data.parentId,
      icon: data.icon,
      color: data.color,
      isSystem: existing?.isSystem ?? false,
      isArchived: false,
      createdAt: existing?.createdAt ?? stamp,
      updatedAt: stamp,
    };
    await mutateSQLite({ operation: "put", table: "categories", value: category });
    set({ categories: existing
      ? get().categories.map((item) => item.id === category.id ? category : item)
      : [...get().categories, category] });
  },
  archiveCategory: async (id) => {
    await mutateSQLite({ operation: "delete", table: "categories", id });
    set({ categories: get().categories.filter((item) => item.id !== id) });
  },
  startTimer: (activityId) => {
    const current = get().timer;
    const timer = current?.activityId === activityId
      ? { ...current, startedAt: current.startedAt ?? Date.now() }
      : { activityId, startedAt: Date.now(), accumulatedSeconds: 0 };
    void mutateSQLite({ operation: "put", table: "settings", value: { id: "preferences", dark: get().dark, timer, updatedAt: now() } }).catch((error) => set({ storageError: error instanceof Error ? error.message : "SQLite 写入失败" }));
    set({ timer });
  },
  pauseTimer: () => {
    const current = get().timer;
    if (!current?.startedAt) return;
    const timer = { ...current, accumulatedSeconds: current.accumulatedSeconds + Math.floor((Date.now() - current.startedAt) / 1000), startedAt: null };
    void mutateSQLite({ operation: "put", table: "settings", value: { id: "preferences", dark: get().dark, timer, updatedAt: now() } }).catch((error) => set({ storageError: error instanceof Error ? error.message : "SQLite 写入失败" }));
    set({ timer });
  },
  finishTimer: async () => {
    const current = get().timer;
    if (!current) return;
    const seconds = current.accumulatedSeconds + (current.startedAt ? Math.floor((Date.now() - current.startedAt) / 1000) : 0);
    const minutes = Math.max(1, Math.round(seconds / 60));
    await get().addLog(current.activityId, minutes);
    await mutateSQLite({ operation: "put", table: "settings", value: { id: "preferences", dark: get().dark, timer: null, updatedAt: now() } });
    set({ timer: null });
  },
  restoreBackup: async (payload) => {
    if (!payload || typeof payload !== "object") throw new Error("备份文件格式无效");
    const data = payload as Partial<{ activities: Activity[]; logs: ActivityLog[]; transactions: Transaction[]; reviews: DailyReview[]; accounts: FinanceAccount[]; workoutHistory: WorkoutHistory[] }>;
    if (![data.activities, data.logs, data.transactions, data.reviews].every(Array.isArray)) throw new Error("备份文件缺少必要数据");
    const accounts=data.accounts??get().accounts; const workoutHistory=data.workoutHistory??get().workoutHistory;
    await mutateSQLite({ operation: "restore", data: { activities: data.activities!, logs: data.logs!, transactions: data.transactions!, reviews: data.reviews!, accounts, workoutHistory } });
    set({ activities: data.activities!.filter((item) => !item.isArchived), logs: data.logs!, transactions: data.transactions!, reviews: data.reviews!, accounts, workoutHistory });
  },
}));
