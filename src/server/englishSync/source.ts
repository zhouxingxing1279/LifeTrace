export type SourceMode = "latest" | "history" | "repair";

export interface NormalizedEnglishArticle {
  source_key: string;
  external_id?: string;
  source_url: string;
  title: string;
  summary?: string;
  content: string;
  author?: string;
  category: string;
  published_at?: string;
  source_updated_at?: string;
  audio_url?: string;
  image_url?: string;
  license_type?: string;
  attribution?: string;
  metadata?: Record<string, unknown>;
}

export interface SourceFetchOptions {
  limit: number;
  overlapDays?: number;
  cursor?: string;
  requestIntervalMs?: number;
}

export interface SourceFetchResult {
  articles: NormalizedEnglishArticle[];
  failed: Array<{ sourceUrl?: string; title?: string; error: string; retryCount?: number }>;
  discoveredCount: number;
  nextCursor?: string;
  requestCount?: number;
}

export interface SourceHealth {
  ok: boolean;
  rateLimited?: boolean;
  detail?: string;
  parserSuccessRate?: number;
}

export interface EnglishContentSource {
  readonly sourceKey: string;
  fetchLatest(options: SourceFetchOptions): Promise<SourceFetchResult>;
  fetchHistory(options: SourceFetchOptions): Promise<SourceFetchResult>;
  fetchArticleDetail(url: string): Promise<NormalizedEnglishArticle>;
  normalizeArticle(raw: unknown): NormalizedEnglishArticle;
  healthCheck(): Promise<SourceHealth>;
}
