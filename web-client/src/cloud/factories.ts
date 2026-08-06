import { amountToCents, baseMeta, localDate, type JsonEntity } from "./types";

export function createFinanceAccount(userId: string, deviceId: string, name: string): JsonEntity {
  return { meta: baseMeta(userId, deviceId), name: name.trim() || "默认账户", accountType: "cash", color: "#49715d", icon: "wallet", isArchived: false, currency: "CNY", openingBalanceCents: 0, balanceAt: new Date().toISOString(), last4: null };
}

export function createFinanceCategory(userId: string, deviceId: string, name: string, categoryType: "expense" | "income"): JsonEntity {
  return { meta: baseMeta(userId, deviceId), name: name.trim(), categoryType, parentId: null, icon: categoryType === "expense" ? "receipt" : "coins", color: categoryType === "expense" ? "#b86b55" : "#49715d", isSystem: false, isArchived: false };
}

export interface TransactionInput {
  accountId?: string | null; toAccountId?: string | null; categoryId?: string | null;
  amount: string; type: "expense" | "income" | "refund" | "fee";
  occurredAt?: string; localDate?: string;
  status?: "candidate" | "confirmed" | "ignored" | "duplicate";
  sourceType?: string; merchant?: string | null; item?: string | null;
  counterparty?: string | null; note?: string | null; externalTransactionId?: string | null;
}

export function createTransaction(userId: string, deviceId: string, input: TransactionInput): JsonEntity {
  const amountCents = Math.abs(amountToCents(input.amount));
  if (!amountCents) throw new Error("金额必须大于 0");
  const occurredAt = input.occurredAt ?? new Date().toISOString();
  return {
    meta: baseMeta(userId, deviceId), transactionType: input.type, amountCents,
    currency: "CNY", occurredAt, localDate: input.localDate ?? localDate(new Date(occurredAt)),
    status: input.status ?? "confirmed", sourceType: input.sourceType ?? "web_manual",
    accountId: input.accountId ?? null, toAccountId: input.toAccountId ?? null,
    categoryId: input.categoryId ?? null, merchant: input.merchant?.trim() || null,
    item: input.item?.trim() || null, counterparty: input.counterparty?.trim() || null,
    note: input.note?.trim() || null, externalTransactionId: input.externalTransactionId?.trim() || null,
  };
}

export function createBudgetPreference(userId: string, deviceId: string, month: string, amount: string, categoryId: string | null = null): JsonEntity {
  if (!/^\d{4}-\d{2}$/.test(month)) throw new Error("预算月份格式必须为 YYYY-MM");
  return { meta: baseMeta(userId, deviceId), preferenceKey: `finance.budget.${month}.${categoryId ?? "all"}`, value: { month, categoryId, amountCents: Math.abs(amountToCents(amount)), warningThreshold: 0.8 } };
}

export interface HabitInput {
  name: string;
  activityType?: string;
  unit?: string;
  minimumTarget?: number | null;
  normalTarget?: number | null;
  targetPeriod?: "daily" | "weekly" | string;
  targetDays?: number[];
  icon?: string | null;
  color?: string | null;
  description?: string | null;
}

export function createHabitActivity(userId: string, deviceId: string, input: HabitInput): JsonEntity {
  const name = input.name.trim();
  if (!name) throw new Error("请输入项目名称");
  return {
    meta: baseMeta(userId, deviceId), name,
    activityType: input.activityType ?? "habit", unit: input.unit?.trim() || "次",
    minimumTarget: input.minimumTarget ?? null, normalTarget: input.normalTarget ?? 1,
    targetPeriod: input.targetPeriod ?? "daily", targetDays: input.targetDays ?? [],
    icon: input.icon ?? name.slice(0, 1), color: input.color ?? "#0f766e",
    scheduleType: "flexible", startDate: localDate(), checkinMethod: "manual",
    syncSource: "web", description: input.description?.trim() || null, isArchived: false,
  };
}

export function createHabitLog(userId: string, deviceId: string, activityId: string, value = 1, note = "", date = localDate()): JsonEntity {
  if (!activityId) throw new Error("请选择坚持项目");
  return {
    meta: baseMeta(userId, deviceId), activityId, logDate: date,
    value: Number.isFinite(value) ? value : 1, status: "completed",
    note: note.trim() || null, metadata: { source: "web" },
  };
}

export interface DailyReviewInput {
  reviewDate?: string;
  energy?: number | null;
  mood?: number | null;
  completionScore?: number | null;
  bestThing?: string;
  problem?: string;
  tomorrowPriority?: string;
  note?: string;
}

export function createDailyReview(userId: string, deviceId: string, input: DailyReviewInput, id?: string): JsonEntity {
  return {
    meta: baseMeta(userId, deviceId, id), reviewDate: input.reviewDate ?? localDate(),
    energy: input.energy ?? null, mood: input.mood ?? null,
    completionScore: input.completionScore ?? null,
    bestThing: input.bestThing?.trim() || null, problem: input.problem?.trim() || null,
    tomorrowPriority: input.tomorrowPriority?.trim() || null, note: input.note?.trim() || null,
  };
}

export interface WorkoutInput {
  name: string;
  occurredAt?: string;
  durationMinutes?: number;
  exerciseCount?: number;
  setCount?: number;
  volumeKg?: number | null;
  caloriesKcal?: number | null;
  source?: string;
}

export function createWorkout(userId: string, deviceId: string, input: WorkoutInput): JsonEntity {
  const occurredAt = input.occurredAt ?? new Date().toISOString();
  return {
    meta: baseMeta(userId, deviceId), source: input.source ?? "manual", sourceId: null,
    name: input.name.trim() || "训练", occurredAt, localDate: localDate(new Date(occurredAt)),
    durationSeconds: Math.max(0, Math.round((input.durationMinutes ?? 0) * 60)),
    exerciseCount: Math.max(0, Math.round(input.exerciseCount ?? 0)),
    setCount: Math.max(0, Math.round(input.setCount ?? 0)), plannedSetCount: null,
    volumeKg: input.volumeKg ?? null, caloriesKcal: input.caloriesKcal ?? null, status: "completed",
  };
}

export function createWorkoutExercise(userId: string, deviceId: string, workoutId: string, name: string, sortOrder = 0, plannedSets = 0, completedSets = 0): JsonEntity {
  return { meta: baseMeta(userId, deviceId), workoutId, name: name.trim() || "训练动作", sortOrder, plannedSets, completedSets };
}

export function createWorkoutSet(userId: string, deviceId: string, exerciseId: string, setNumber: number, weightKg: number | null, reps: number | null): JsonEntity {
  return { meta: baseMeta(userId, deviceId), exerciseId, setNumber, weightKg, reps, completed: true };
}

export function createWorkoutImport(userId: string, deviceId: string, shareUrl: string): JsonEntity {
  return { meta: baseMeta(userId, deviceId), source: "xunji", shareUrl: shareUrl.trim() || null, status: "pending", parser: null, parserVersion: null, error: null, workoutId: null };
}

export function createTrainingNote(userId: string, deviceId: string, title: string, content: string, workoutId: string | null = null): JsonEntity {
  return { meta: baseMeta(userId, deviceId), title: title.trim() || "训练笔记", content: content.trim(), workoutId, source: "manual", noteDate: localDate() };
}

export function createNoteFolder(userId: string, deviceId: string, name: string, sortOrder = 0): JsonEntity {
  return { meta: baseMeta(userId, deviceId), name: name.trim(), icon: "folder", color: "#8a765b", sortOrder };
}

export function createNoteTag(userId: string, deviceId: string, name: string): JsonEntity {
  return { meta: baseMeta(userId, deviceId), name: name.trim(), color: "#49715d" };
}

export function createNoteTagRelation(userId: string, deviceId: string, noteId: string, tagId: string): JsonEntity {
  return { meta: baseMeta(userId, deviceId, `${noteId}:${tagId}`), noteId, tagId };
}

export interface NoteContent { html: string; text: string; json: unknown; markdown?: string; }

function escapeHtml(value: string): string {
  return value.replace(/[&<>'"]/g, (character) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", "'": "&#39;", '"': "&quot;" })[character] ?? character);
}

export function createNote(userId: string, deviceId: string, title: string, content: string | NoteContent, folderId: string | null = null): JsonEntity {
  const value: NoteContent = typeof content === "string"
    ? { html: content.trim() ? `<p>${escapeHtml(content.trim()).replace(/\n/g, "<br>")}</p>` : "", text: content.trim(), json: { type: "doc", content: content.trim() }, markdown: content.trim() }
    : content;
  return {
    meta: baseMeta(userId, deviceId), noteType: "quick", title: title.trim() || null,
    contentJson: value.json, contentHtml: value.html, contentText: value.text,
    contentMarkdown: value.markdown ?? value.text, summary: value.text.slice(0, 160),
    isPinned: false, isFavorite: false, isArchived: false, folderId,
    aiSummary: null, aiTags: null, embeddingStatus: null, lastAiProcessedAt: null,
  };
}

export function createVocabulary(userId: string, deviceId: string, word: string, definition: string): JsonEntity {
  const displayWord = word.trim();
  if (!displayWord) throw new Error("请输入单词");
  const cleanDefinition = definition.trim();
  return {
    meta: baseMeta(userId, deviceId), normalizedWord: displayWord.toLocaleLowerCase("en-US"), displayWord,
    definition: cleanDefinition, phonetic: "", partOfSpeech: "", selectedMeanings: cleanDefinition ? [cleanDefinition] : [],
    lemma: displayWord.toLocaleLowerCase("en-US"), notes: "", masteryLevel: 0, reviewStage: 0,
    reviewCount: 0, correctCount: 0, incorrectCount: 0, encounterCount: 1, status: "LEARNING",
    tags: [], sourceArticleId: null, sourceArticleTitle: null, sourceSentence: null,
    frequencyRank: null, lastReviewedAt: null, nextReviewAt: null, metadata: null,
  };
}

export function createEnglishHighlight(userId: string, deviceId: string, articleId: string, selectedText: string, note = ""): JsonEntity {
  return { meta: baseMeta(userId, deviceId), articleId, blockId: null, selectedText: selectedText.trim(), startOffset: null, endOffset: null, prefix: null, suffix: null, color: "yellow", note: note.trim() || null };
}

export function createEnglishNote(userId: string, deviceId: string, articleId: string, content: string, quote = ""): JsonEntity {
  return { meta: baseMeta(userId, deviceId), articleId, quote: quote.trim() || null, content: content.trim(), blockId: null, startOffset: null, endOffset: null, selectedText: quote.trim() || null, prefix: null, suffix: null, highlightId: null };
}

export function createEnglishLearningRecord(userId: string, deviceId: string, articleId: string, summary: string, readingTimeSeconds: number, newWords: string[] = []): JsonEntity {
  return { meta: baseMeta(userId, deviceId), articleId, analysisId: null, recordDate: localDate(), readingTimeSeconds: Math.max(0, Math.round(readingTimeSeconds)), summary: summary.trim(), newWords, completionStatus: "completed", readingStatus: "completed", startedAt: null, completedAt: new Date().toISOString(), score: null };
}

export function createPreference(userId: string, deviceId: string, preferenceKey: string, value: unknown): JsonEntity {
  return { meta: baseMeta(userId, deviceId), preferenceKey, value };
}

export function createFileMetadata(userId: string, deviceId: string, file: { name: string; type: string; size: number; sha256: string }): JsonEntity {
  return { meta: baseMeta(userId, deviceId), originalName: file.name, mimeType: file.type || "application/octet-stream", sizeBytes: file.size, sha256: file.sha256, storageState: "pending_upload", createdByDevice: deviceId };
}
