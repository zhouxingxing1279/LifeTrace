// 每日英语领域模型。API、Mock AI 和前端共用这些类型，后续替换 DeepSeek 时无需修改页面。
export type CEFRLevel = "A1" | "A2" | "B1" | "B2" | "C1";
export type EnglishCategory = "Technology" | "Science" | "Life" | "Business" | "Culture";
export type EnglishProcessingStatus = "FETCHED" | "CLEANED" | "ANALYZED" | "READY" | "REJECTED" | "FAILED";
export type EnglishFetchStatus = "PENDING" | "SUCCESS" | "FAILED" | "SKIPPED";
export type EnglishSourceStatus = "active" | "stale" | "error" | "disabled" | "rate_limited";
export type EnglishSyncTaskStatus = "PENDING" | "RUNNING" | "COMPLETED" | "PARTIAL_SUCCESS" | "FAILED" | "CANCELLED";

export interface ArticleVocabularyItem {
  word: string;
  phonetic: string;
  meaning: string;
  example: string;
}

export interface EnglishArticle {
  id: string;
  title: string;
  level: CEFRLevel;
  category: EnglishCategory;
  content: string;
  vocabulary: ArticleVocabularyItem[];
  questions: string[];
  difficulty: number;
  estimatedMinutes: number;
  createdTime: string;
  updatedAt: string;
  source?: "local" | "voa" | string;
  sourceKey?: string;
  sourceName?: string;
  sourceCategory?: string;
  sourceUrl?: string;
  normalizedSourceUrl?: string;
  externalId?: string;
  publishedAt?: string;
  sourceUpdatedAt?: string;
  imageUrl?: string;
  audioUrl?: string;
  author?: string;
  summary?: string;
  wordCount?: number;
  fetchedAt?: string;
  rightsNote?: string;
  contentHash?: string;
  language?: string;
  qualityScore?: number;
  hasAudio?: boolean;
  licenseType?: string;
  attribution?: string;
  processingStatus?: EnglishProcessingStatus;
  fetchStatus?: EnglishFetchStatus;
  retryCount?: number;
  lastError?: string;
}

export interface EnglishSourceSyncResult {
  source: "voa" | "all" | string;
  engine: "python";
  imported: number;
  inserted?: number;
  updated?: number;
  skipped: number;
  failed: number;
  syncedAt: string;
  cached: boolean;
  taskId?: string;
  status?: EnglishSyncTaskStatus;
}

export interface EnglishContentSourceState {
  id: string;
  sourceKey: string;
  sourceName: string;
  sourceType: string;
  sourceUrl: string;
  category: string;
  enabled: boolean;
  syncInterval: number;
  initialFetchLimit: number;
  recentScanLimit: number;
  overlapDays: number;
  requestIntervalMs: number;
  lastSyncAt?: string;
  lastSuccessAt?: string;
  lastNewArticleAt?: string;
  latestExternalPublishedAt?: string;
  syncCursor?: string;
  consecutiveFailures: number;
  status: EnglishSourceStatus;
  lastError?: string;
  articleCount: number;
  createdAt: string;
  updatedAt: string;
}

export interface EnglishSyncTask {
  taskId: string;
  taskType: "incremental" | "backfill" | "retry_failed" | "weekly_repair" | "monthly_health";
  sourceKey?: string;
  requestedLimit?: number;
  status: EnglishSyncTaskStatus;
  startedAt?: string;
  finishedAt?: string;
  totalCount: number;
  successCount: number;
  insertedCount: number;
  updatedCount: number;
  skippedCount: number;
  failedCount: number;
  currentArticle?: string;
  progress: number;
  lastError?: string;
  createdAt: string;
  updatedAt: string;
}

export interface EnglishSyncLog {
  id: string;
  taskId: string;
  sourceKey?: string;
  level: "info" | "warning" | "error";
  event: string;
  requestUrl?: string;
  message: string;
  retryCount: number;
  durationMs?: number;
  details?: Record<string, unknown>;
  createdAt: string;
}

export interface EnglishLibraryStats {
  total: number;
  ready: number;
  pending: number;
  failed: number;
  rejected: number;
  withAudio: number;
  byCefr: Record<string, number>;
  byCategory: Record<string, number>;
  lastSyncAt?: string;
  lastNewArticleAt?: string;
  initialization: {
    status: "not_started" | "running" | "completed" | "failed";
    initializedAt?: string;
    initialArticleCount: number;
    targetArticleCount: number;
    currentSourceKey?: string;
    lastError?: string;
  };
}

export type EnglishCompletionStatus = "reading" | "summarized" | "analyzed" | "completed";

export interface EnglishLearningRecord {
  id: string;
  userId: string;
  date: string;
  articleId: string;
  readingTimeSeconds: number;
  summary: string;
  score?: number;
  analysisId?: string;
  newWords: string[];
  completionStatus: EnglishCompletionStatus;
  startedAt: string;
  completedAt?: string;
  createdAt: string;
  updatedAt: string;
}

export interface EnglishVocabulary {
  id: string;
  userId: string;
  word: string;
  phonetic: string;
  meaning: string;
  example: string;
  sourceArticleId: string;
  reviewCount: number;
  masterLevel: number;
  nextReviewTime: string;
  createdAt: string;
  updatedAt: string;
}

export type VocabularyStatus = "LEARNING" | "REVIEWING" | "MASTERED" | "ARCHIVED";
export type VocabularyReviewResult = "FORGOT" | "HARD" | "GOOD" | "EASY";

export interface DictionaryLookup {
  queryWord: string;
  normalizedWord: string;
  lemma?: string;
  found: boolean;
  reason?: "INVALID_WORD" | "NOT_FOUND";
  dictionaryWordId?: number;
  phonetic?: string;
  partsOfSpeech?: Array<{ type: string; translation: string[]; definition: string[] }>;
  collins?: number;
  oxford?: boolean;
  tags?: string[];
  bncRank?: number;
  frequencyRank?: number;
  exchange?: Record<string, string>;
  sourceSentence?: string;
  articleId?: string;
}

export interface UserVocabulary {
  id: string;
  word: string;
  normalizedWord: string;
  lemma: string;
  dictionaryWordId?: number;
  phonetic: string;
  selectedMeanings: string[];
  partOfSpeech: string;
  sourceArticleId?: string;
  sourceArticleTitle?: string;
  sourceSentence?: string;
  notes: string;
  masteryLevel: number;
  reviewStage: number;
  reviewCount: number;
  correctCount: number;
  incorrectCount: number;
  encounterCount: number;
  lastReviewedAt?: string;
  nextReviewAt?: string;
  status: VocabularyStatus;
  frequencyRank?: number;
  tags: string[];
  createdAt: string;
  updatedAt: string;
  occurrences?: VocabularyOccurrence[];
  reviewLogs?: VocabularyReviewLog[];
}

export interface VocabularyOccurrence {
  id: string;
  vocabularyId: string;
  articleId?: string;
  articleTitle?: string;
  sourceSentence: string;
  createdAt: string;
}

export interface VocabularyReviewLog {
  id: string;
  vocabularyId: string;
  result: VocabularyReviewResult;
  stageBefore: number;
  stageAfter: number;
  reviewedAt: string;
  nextReviewAt?: string;
  responseTimeMs?: number;
}

export interface VocabularySettings {
  preferredAccent: "en-US" | "en-GB";
  wordSpeechRate: number;
  sentenceSpeechRate: number;
  autoPronounce: boolean;
  defaultFirstMeaning: boolean;
  dailyReviewLimit: number;
  showSourceSentence: boolean;
  includeMasteredInRecommendations: boolean;
}

export interface EnglishHighlight {
  id: string;
  userId: string;
  articleId: string;
  text: string;
  color: "yellow" | "green" | "blue";
  createdAt: string;
  updatedAt: string;
}

export interface EnglishNote {
  id: string;
  userId: string;
  articleId: string;
  quote?: string;
  content: string;
  createdAt: string;
  updatedAt: string;
}

export interface EnglishMistake {
  original: string;
  correction: string;
  reason: string;
}

export interface EnglishAIAnalysis {
  id: string;
  userId: string;
  recordId: string;
  articleId: string;
  provider: "mock" | "deepseek";
  score: number;
  contentScore: number;
  grammarScore: number;
  vocabularyScore: number;
  structureScore: number;
  mistakes: EnglishMistake[];
  suggestions: string[];
  improvedSummary: string;
  weakPoints: string[];
  createdAt: string;
  updatedAt: string;
}

export interface EnglishTodayResponse {
  article: EnglishArticle;
  record?: EnglishLearningRecord;
  currentLevel: CEFRLevel;
  streak: number;
  weekCompleted: string[];
  recentRecords: Array<EnglishLearningRecord & { article?: EnglishArticle }>;
}

export interface EnglishHistoryResponse {
  records: Array<EnglishLearningRecord & { article?: EnglishArticle; analysis?: EnglishAIAnalysis }>;
  stats: {
    readingCount30: number;
    averageScore30: number;
    vocabularyGrowth30: number;
    streak: number;
  };
}
