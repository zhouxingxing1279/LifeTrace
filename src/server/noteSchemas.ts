import { z } from "zod";

export const noteTypes = [
  "quick", "document", "daily", "habit_log", "workout_review",
  "expense_note", "weekly_review", "monthly_review",
] as const;

export const idSchema = z.string().trim().min(1).max(100).regex(/^[\w-]+$/);
export const noteTypeSchema = z.enum(noteTypes);
export const relationSchema = z.object({
  entityType: z.enum(["habit", "habit_checkin", "workout", "exercise", "transaction", "account", "project"]),
  entityId: idSchema,
  relationType: z.enum(["reference", "created_from", "summary", "attachment"]).default("reference"),
});
export const notePayloadSchema = z.object({
  id: idSchema.optional(),
  title: z.string().max(300).nullable().optional(),
  noteType: noteTypeSchema.default("document"),
  folderId: idSchema.nullable().optional(),
  contentJson: z.record(z.string(), z.unknown()).default({ type: "doc", content: [] }),
  contentHtml: z.string().max(5_000_000).default(""),
  contentText: z.string().max(5_000_000).default(""),
  contentMarkdown: z.string().max(5_000_000).default(""),
  summary: z.string().max(500).default(""),
  isPinned: z.boolean().default(false),
  isFavorite: z.boolean().default(false),
  isArchived: z.boolean().default(false),
  tagIds: z.array(idSchema).max(50).default([]),
  relations: z.array(relationSchema).max(100).default([]),
  createRevision: z.boolean().default(false),
});

export const folderSchema = z.object({
  id: idSchema.optional(),
  name: z.string().trim().min(1).max(80),
  icon: z.string().trim().max(20).default("folder"),
  color: z.string().regex(/^#[0-9a-fA-F]{6}$/).default("#2a7a5e"),
  sortOrder: z.number().int().min(0).max(10000).default(0),
});

export const tagSchema = z.object({
  id: idSchema.optional(),
  name: z.string().trim().min(1).max(50),
  color: z.string().regex(/^#[0-9a-fA-F]{6}$/).default("#5f7d70"),
});

export function safeJson<T>(value: string | null | undefined, fallback: T): T {
  if (!value) return fallback;
  try { return JSON.parse(value) as T; } catch { return fallback; }
}
