// 每日英语领域模型。API、Mock AI 和前端共用这些类型，后续替换 DeepSeek 时无需修改页面。
export type CEFRLevel = "A1" | "A2" | "B1" | "B2" | "C1";
export type EnglishCategory = "Technology" | "Science" | "Life" | "Business" | "Culture";

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
  source?: "local" | "voa";
  sourceName?: string;
  sourceUrl?: string;
  externalId?: string;
  publishedAt?: string;
  imageUrl?: string;
  audioUrl?: string;
  author?: string;
  summary?: string;
  wordCount?: number;
  fetchedAt?: string;
  rightsNote?: string;
}

export interface EnglishSourceSyncResult {
  source: "voa";
  engine: "python";
  imported: number;
  skipped: number;
  failed: number;
  syncedAt: string;
  cached: boolean;
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
