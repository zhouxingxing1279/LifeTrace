import { sqliteTable, text } from "drizzle-orm/sqlite-core";

const entity = (name: string) => sqliteTable(name, {
  id: text("id").primaryKey(),
  dataJson: text("data_json").notNull(),
  updatedAt: text("updated_at").notNull(),
});

export const activities = entity("activities");
export const activityLogs = entity("activity_logs");
export const transactions = entity("transactions");
export const dailyReviews = entity("daily_reviews");
export const settings = entity("settings");
export const financeAccounts = entity("finance_accounts");
export const workoutTemplates = entity("workout_templates");
export const workoutHistory = entity("workout_history");
export const exerciseLibrary = entity("exercise_library");
export const englishArticles = entity("english_articles");
export const englishLearningRecords = entity("english_learning_records");
export const englishVocabulary = entity("english_vocabulary");
export const englishHighlights = entity("english_highlights");
export const englishNotes = entity("english_notes");
export const englishAiAnalysis = entity("english_ai_analysis");
export const workoutImportRecords = entity("workout_import_records");
export const trainingNotes = entity("training_notes");
export const importUploads = entity("import_uploads");
export const notes = entity("notes");
export const noteFolders = entity("note_folders");
export const noteTags = entity("note_tags");
export const noteTagRelations = entity("note_tag_relations");
export const noteRelations = entity("note_relations");
export const noteAttachments = entity("note_attachments");
export const noteRevisions = entity("note_revisions");
