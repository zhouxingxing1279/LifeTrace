export const ENGLISH_SYNC_CONFIG = {
  syncIntervalSeconds: Number(process.env.ENGLISH_SYNC_INTERVAL_SECONDS || 86_400),
  overlapDays: Number(process.env.ENGLISH_SYNC_OVERLAP_DAYS || 14),
  recentScanLimit: Number(process.env.ENGLISH_SYNC_RECENT_SCAN_LIMIT || 30),
  weeklyScanLimit: Number(process.env.ENGLISH_SYNC_WEEKLY_SCAN_LIMIT || 50),
  staleAfterDays: Number(process.env.ENGLISH_SYNC_STALE_AFTER_DAYS || 30),
  errorFailureThreshold: Number(process.env.ENGLISH_SYNC_ERROR_THRESHOLD || 3),
  minimumWords: Number(process.env.ENGLISH_ARTICLE_MIN_WORDS || 200),
  maximumWords: Number(process.env.ENGLISH_ARTICLE_MAX_WORDS || 3000),
  minimumEnglishRatio: Number(process.env.ENGLISH_ARTICLE_MIN_ENGLISH_RATIO || 0.75),
  minimumQualityScore: Number(process.env.ENGLISH_ARTICLE_MIN_QUALITY_SCORE || 60),
  defaultInitialFetchLimit: Number(process.env.ENGLISH_INITIAL_FETCH_LIMIT || 100),
} as const;

export const DEFAULT_VOA_SOURCES = [
  { sourceKey: "voa_science", sourceName: "VOA Science & Technology", category: "Science", sourceUrl: "https://learningenglish.voanews.com/z/1579", feedKey: "science" },
  { sourceKey: "voa_health", sourceName: "VOA Health & Lifestyle", category: "Life", sourceUrl: "https://learningenglish.voanews.com/z/955", feedKey: "health" },
  { sourceKey: "voa_words", sourceName: "VOA Words and Their Stories", category: "Culture", sourceUrl: "https://learningenglish.voanews.com/z/987/episodes", feedKey: "words" },
  { sourceKey: "voa_grammar", sourceName: "VOA Everyday Grammar", category: "Culture", sourceUrl: "https://learningenglish.voanews.com/z/4456/episodes", feedKey: "grammar" },
  { sourceKey: "voa_education", sourceName: "VOA Education", category: "Life", sourceUrl: "https://learningenglish.voanews.com/z/959", feedKey: "education" },
] as const;
