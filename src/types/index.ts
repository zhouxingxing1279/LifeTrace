export type ActivityType = "duration" | "count" | "completion" | "weekly" | "control";

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
  type: "expense" | "income";
  amount: number;
  category: string;
  account: string;
  note?: string;
  occurredAt: string;
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

