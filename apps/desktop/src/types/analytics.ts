export type AnalyticsDomain = "finance" | "habits" | "notes" | "english" | "fitness" | "execution";

export type ProjectionStatus = {
  dirty: boolean;
  eventCount: number;
  searchDocumentCount: number;
  lastRebuiltAt?: string | null;
  projectionVersion: number;
  lastError?: string | null;
};

export type TimelineEvent = {
  id: string;
  occurredAt: string;
  endedAt?: string | null;
  localDate: string;
  timezone?: string | null;
  domain: AnalyticsDomain | string;
  eventType: string;
  title: string;
  summary: string;
  entityType: string;
  entityId: string;
  metrics: Record<string, unknown>;
  tags: unknown[];
};

export type TimelinePage = {
  items: TimelineEvent[];
  nextCursor?: string | null;
};

export type SearchHit = {
  id: string;
  domain: AnalyticsDomain | string;
  entityType: string;
  entityId: string;
  title: string;
  snippet: string;
  occurredAt?: string | null;
  updatedAt: string;
  score: number;
};

export type ReportFacts = {
  period: { start: string; end: string; timezone: string };
  finance: { transactionCount: number; expenseCents: number; incomeCents: number; netCents: number };
  habits: { logCount: number; completedCount: number; completionRate: number };
  fitness: { workoutCount: number; durationSeconds: number; volumeKg: number; caloriesKcal: number };
  english: { sessionCount: number; readingTimeSeconds: number; completedCount: number; newVocabularyCount: number };
  notes: { createdCount: number };
  execution: { taskCount: number; completedTaskCount: number; calendarEventCount: number };
  reviews: { count: number; averageMood: number; averageEnergy: number };
};

export type ReportSnapshot = {
  id: string;
  reportType: "weekly" | "monthly" | "custom";
  periodStart: string;
  periodEnd: string;
  timezone: string;
  facts: ReportFacts;
  coverage: Record<string, boolean>;
  generatedAt: string;
  factsVersion: number;
};

export type InsightSnapshot = {
  id: string;
  insightType: string;
  periodStart: string;
  periodEnd: string;
  title: string;
  summary: string;
  evidence: Record<string, unknown>;
  sampleSize: number;
  confidence: { level?: string; causal?: boolean; [key: string]: unknown };
  algorithmVersion: string;
};
