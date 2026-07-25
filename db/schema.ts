import { integer, real, sqliteTable, text, uniqueIndex } from "drizzle-orm/sqlite-core";

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
export const englishArticles = sqliteTable("english_articles", {
  id: text("id").primaryKey(),
  dataJson: text("data_json").notNull(),
  updatedAt: text("updated_at").notNull(),
  sourceKey: text("source_key"),
  sourceName: text("source_name"),
  sourceCategory: text("source_category"),
  externalId: text("external_id"),
  sourceUrl: text("source_url"),
  normalizedSourceUrl: text("normalized_source_url"),
  title: text("title"),
  summary: text("summary"),
  content: text("content"),
  author: text("author"),
  publishedAt: text("published_at"),
  sourceUpdatedAt: text("source_updated_at"),
  fetchedAt: text("fetched_at"),
  createdAt: text("created_at"),
  contentHash: text("content_hash"),
  wordCount: integer("word_count"),
  language: text("language"),
  cefrLevel: text("cefr_level"),
  estimatedReadingMinutes: integer("estimated_reading_minutes"),
  qualityScore: real("quality_score"),
  audioUrl: text("audio_url"),
  imageUrl: text("image_url"),
  hasAudio: integer("has_audio", { mode: "boolean" }),
  licenseType: text("license_type"),
  attribution: text("attribution"),
  processingStatus: text("processing_status"),
  fetchStatus: text("fetch_status"),
  retryCount: integer("retry_count").default(0),
  lastError: text("last_error"),
}, (table) => [
  uniqueIndex("english_articles_source_external_unique")
    .on(table.sourceKey, table.externalId),
  uniqueIndex("english_articles_source_url_unique").on(table.normalizedSourceUrl),
]);
export const englishLearningRecords = entity("english_learning_records");
export const englishVocabulary = entity("english_vocabulary");
export const englishUserVocabulary = sqliteTable("english_user_vocabulary", {
  id: text("id").primaryKey(),
  word: text("word").notNull(),
  normalizedWord: text("normalized_word").notNull(),
  lemma: text("lemma").notNull(),
  dictionaryWordId: integer("dictionary_word_id"),
  phonetic: text("phonetic"),
  selectedMeaningsJson: text("selected_meanings_json").notNull(),
  partOfSpeech: text("part_of_speech"),
  sourceArticleId: text("source_article_id"),
  sourceArticleTitle: text("source_article_title"),
  sourceSentence: text("source_sentence"),
  notes: text("notes"),
  masteryLevel: integer("mastery_level").notNull().default(0),
  reviewStage: integer("review_stage").notNull().default(0),
  reviewCount: integer("review_count").notNull().default(0),
  correctCount: integer("correct_count").notNull().default(0),
  incorrectCount: integer("incorrect_count").notNull().default(0),
  encounterCount: integer("encounter_count").notNull().default(1),
  lastReviewedAt: text("last_reviewed_at"),
  nextReviewAt: text("next_review_at"),
  status: text("status").notNull().default("LEARNING"),
  frequencyRank: integer("frequency_rank"),
  tagsJson: text("tags_json").notNull().default("[]"),
  createdAt: text("created_at").notNull(),
  updatedAt: text("updated_at").notNull(),
}, (table) => [uniqueIndex("english_user_vocabulary_lemma_unique").on(table.lemma)]);
export const englishVocabularyOccurrences = sqliteTable("english_vocabulary_occurrences", {
  id: text("id").primaryKey(),
  vocabularyId: text("vocabulary_id").notNull(),
  articleId: text("article_id"),
  articleTitle: text("article_title"),
  sourceSentence: text("source_sentence").notNull(),
  createdAt: text("created_at").notNull(),
});
export const englishVocabularyReviewLogs = sqliteTable("english_vocabulary_review_logs", {
  id: text("id").primaryKey(),
  vocabularyId: text("vocabulary_id").notNull(),
  result: text("result").notNull(),
  stageBefore: integer("stage_before").notNull(),
  stageAfter: integer("stage_after").notNull(),
  reviewedAt: text("reviewed_at").notNull(),
  nextReviewAt: text("next_review_at"),
  responseTimeMs: integer("response_time_ms"),
});
export const englishVocabularySettings = sqliteTable("english_vocabulary_settings", {
  id: text("id").primaryKey(),
  dataJson: text("data_json").notNull(),
  updatedAt: text("updated_at").notNull(),
});
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

export const englishContentSources = sqliteTable("english_content_sources", {
  id: text("id").primaryKey(),
  sourceKey: text("source_key").notNull().unique(),
  sourceName: text("source_name").notNull(),
  sourceType: text("source_type").notNull(),
  sourceUrl: text("source_url").notNull(),
  category: text("category").notNull(),
  enabled: integer("enabled", { mode: "boolean" }).notNull().default(true),
  syncInterval: integer("sync_interval").notNull().default(86400),
  initialFetchLimit: integer("initial_fetch_limit").notNull().default(100),
  recentScanLimit: integer("recent_scan_limit").notNull().default(30),
  overlapDays: integer("overlap_days").notNull().default(14),
  requestIntervalMs: integer("request_interval_ms").notNull().default(1000),
  lastSyncAt: text("last_sync_at"),
  lastSuccessAt: text("last_success_at"),
  lastNewArticleAt: text("last_new_article_at"),
  latestExternalPublishedAt: text("latest_external_published_at"),
  syncCursor: text("sync_cursor"),
  consecutiveFailures: integer("consecutive_failures").notNull().default(0),
  status: text("status").notNull().default("active"),
  lastError: text("last_error"),
  createdAt: text("created_at").notNull(),
  updatedAt: text("updated_at").notNull(),
});

export const englishLibraryState = sqliteTable("english_library_state", {
  id: text("id").primaryKey(),
  initializationStatus: text("initialization_status").notNull().default("not_started"),
  initializedAt: text("initialized_at"),
  initialArticleCount: integer("initial_article_count").notNull().default(0),
  targetArticleCount: integer("target_article_count").notNull().default(500),
  currentSourceKey: text("current_source_key"),
  lastError: text("last_error"),
  createdAt: text("created_at").notNull(),
  updatedAt: text("updated_at").notNull(),
});

export const englishSyncTasks = sqliteTable("english_sync_tasks", {
  taskId: text("task_id").primaryKey(),
  taskType: text("task_type").notNull(),
  sourceKey: text("source_key"),
  requestedLimit: integer("requested_limit"),
  status: text("status").notNull(),
  startedAt: text("started_at"),
  finishedAt: text("finished_at"),
  totalCount: integer("total_count").notNull().default(0),
  successCount: integer("success_count").notNull().default(0),
  insertedCount: integer("inserted_count").notNull().default(0),
  updatedCount: integer("updated_count").notNull().default(0),
  skippedCount: integer("skipped_count").notNull().default(0),
  failedCount: integer("failed_count").notNull().default(0),
  currentArticle: text("current_article"),
  progress: real("progress").notNull().default(0),
  lastError: text("last_error"),
  createdAt: text("created_at").notNull(),
  updatedAt: text("updated_at").notNull(),
});

export const englishSyncLogs = sqliteTable("english_sync_logs", {
  id: text("id").primaryKey(),
  taskId: text("task_id").notNull(),
  sourceKey: text("source_key"),
  level: text("level").notNull(),
  event: text("event").notNull(),
  requestUrl: text("request_url"),
  message: text("message").notNull(),
  retryCount: integer("retry_count").notNull().default(0),
  durationMs: integer("duration_ms"),
  detailsJson: text("details_json"),
  createdAt: text("created_at").notNull(),
});

export const englishProcessingQueue = sqliteTable("english_processing_queue", {
  id: text("id").primaryKey(),
  articleId: text("article_id").notNull(),
  jobType: text("job_type").notNull(),
  status: text("status").notNull().default("PENDING"),
  retryCount: integer("retry_count").notNull().default(0),
  lastError: text("last_error"),
  availableAt: text("available_at").notNull(),
  createdAt: text("created_at").notNull(),
  updatedAt: text("updated_at").notNull(),
}, (table) => [
  uniqueIndex("english_processing_queue_article_job_unique").on(table.articleId, table.jobType),
]);
