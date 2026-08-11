export type ActivityType = "duration" | "count" | "completion" | "weekly" | "control";
export type ActivityColorKey = "emerald" | "blue" | "cyan" | "violet" | "rose" | "orange" | "amber" | "slate";
export type ActivityScheduleType = "daily" | "weekly" | "custom";
export type ActivityCheckinMethod = "manual" | "automatic";
export type ActivitySyncSource = "fitness" | "english";

export interface Activity {
  id: string;
  userId: string;
  name: string;
  type: ActivityType;
  unit: string;
  minimumTarget?: number;
  normalTarget?: number;
  targetPeriod: "daily" | "weekly";
  targetDays?: number[];
  icon?: string;
  color?: ActivityColorKey;
  scheduleType?: ActivityScheduleType;
  startDate?: string;
  checkinMethod?: ActivityCheckinMethod;
  syncSource?: ActivitySyncSource;
  description?: string;
  isArchived: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface ActivityLog {
  id: string;
  userId: string;
  activityId: string;
  value?: number;
  status?: "completed" | "partial" | "skipped";
  note?: string;
  metadata?: { state?: "stable" | "urge" | "relapse"; urgeLevel?: number; triggers?: string[]; actions?: string[] };
  createdAt: string;
  updatedAt: string;
}

export interface Transaction {
  id: string;
  userId: string;
  type: "expense" | "income" | "transfer";
  amount: number;
  category: string;
  categoryId?: string;
  account: string;
  accountId?: string;
  toAccount?: string;
  toAccountId?: string;
  counterparty?: string;
  item?: string;
  note?: string;
  occurredAt: string;
  createdAt: string;
  updatedAt: string;
}

export interface FinanceCategory {
  id: string;
  userId: string;
  name: string;
  type: "expense" | "income" | "transfer" | "refund" | "fee";
  parentId?: string;
  icon?: string;
  color?: string;
  isSystem: boolean;
  isArchived: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface FinanceAccount {
  id: string;
  userId: string;
  name: string;
  type: "cash" | "bank" | "wechat" | "alipay" | "investment" | "other";
  /** Balance at balanceAt; transactions after this point determine the current balance. */
  balance: number | null;
  balanceAt?: string;
  last4?: string;
  color: string;
  icon: string;
  isArchived: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface WorkoutHistorySet {
  weight: number;
  reps: number;
  completed: boolean;
}
export interface WorkoutHistoryExercise {
  name: string;
  plannedSets: number;
  completedSets: number;
  sets: WorkoutHistorySet[];
}
export interface WorkoutHistory {
  id: string; userId: string; templateId: string; name: string; occurredAt: string; durationSeconds: number;
  exerciseCount: number; setCount: number; plannedSetCount?: number; status?: "completed" | "partial";
  source: "manual" | "xunji"; sourceId?: string; caloriesKcal?: number; volumeKg?: number;
  exercises?: WorkoutHistoryExercise[]; createdAt: string; updatedAt: string;
}

export interface XunjiWorkoutSet {
  weightKg: number;
  reps: number;
  setNumber: number;
}

export interface XunjiWorkoutExercise {
  name: string;
  sets: XunjiWorkoutSet[];
}

export interface XunjiWorkout {
  source: "xunji";
  date: string;
  title: string;
  durationMinutes: number;
  caloriesKcal: number;
  volumeKg: number;
  exercises: XunjiWorkoutExercise[];
}

export interface WorkoutImportRecord {
  id: string;
  userId: string;
  source: "xunji";
  shareUrl?: string;
  rawData: unknown;
  workout?: XunjiWorkout;
  status: "pending" | "success" | "failed";
  error?: string;
  workoutRecordId?: string;
  createdAt: string;
  updatedAt: string;
}

export interface TrainingNote {
  id: string;
  userId: string;
  title: string;
  content: string;
  workoutRecordId: string;
  source: "xunji";
  noteDate: string;
  createdAt: string;
  updatedAt: string;
}

export interface DailyReview {
  id: string;
  userId: string;
  reviewDate: string;
  energy: number;
  mood: number;
  completionScore?: number;
  bestThing?: string;
  problem?: string;
  tomorrowPriority?: string;
  note?: string;
  createdAt: string;
  updatedAt: string;
}

export type ViewId = "today" | "calendar" | "activities" | "finance" | "statistics" | "review" | "settings";

export type NoteType =
  | "quick"
  | "document"
  | "daily"
  | "habit_log"
  | "workout_review"
  | "expense_note"
  | "weekly_review"
  | "monthly_review";

export interface NoteFolder {
  id: string;
  name: string;
  icon: string;
  color: string;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
}

export interface NoteTag {
  id: string;
  name: string;
  color: string;
  createdAt: string;
  updatedAt: string;
}

export interface NoteRelation {
  id: string;
  noteId: string;
  entityType: "habit" | "habit_checkin" | "workout" | "exercise" | "transaction" | "account" | "project";
  entityId: string;
  relationType: "reference" | "created_from" | "summary" | "attachment";
  createdAt: string;
}

export interface NoteAttachment {
  id: string;
  noteId: string;
  fileName: string;
  originalName: string;
  mimeType: string;
  fileSize: number;
  storagePath?: string;
  createdAt: string;
}

export interface NoteRevision {
  id: string;
  noteId: string;
  version: number;
  title: string | null;
  contentJson: Record<string, unknown>;
  contentHtml: string;
  contentMarkdown: string;
  createdAt: string;
}

export interface Note {
  id: string;
  title: string | null;
  noteType: NoteType;
  folderId: string | null;
  contentJson: Record<string, unknown>;
  contentHtml: string;
  contentText: string;
  contentMarkdown: string;
  summary: string;
  isPinned: boolean;
  isFavorite: boolean;
  isArchived: boolean;
  createdAt: string;
  updatedAt: string;
  deletedAt: string | null;
  version: number;
  aiSummary?: string | null;
  aiTags?: string | null;
  embeddingStatus?: string | null;
  lastAiProcessedAt?: string | null;
  tags: NoteTag[];
  relations: NoteRelation[];
  attachments?: NoteAttachment[];
}
