import type { EnglishArticle, EnglishCategory, EnglishSourceSyncResult } from "@/src/types/english";
import type {
  EnglishContentSource,
  NormalizedEnglishArticle,
  SourceFetchOptions,
  SourceFetchResult,
  SourceHealth,
  SourceMode,
} from "@/src/server/englishSync/source";

type PythonFetchResponse = {
  engine?: unknown;
  articles?: unknown;
  skipped?: unknown;
  failed?: unknown;
  errors?: unknown;
  discovered_count?: unknown;
  next_cursor?: unknown;
  request_count?: unknown;
  ok?: unknown;
  rate_limited?: unknown;
  detail?: unknown;
  error?: unknown;
};

const SERVICE_URL = process.env.VOA_SERVICE_URL
  ?? process.env.XUNJI_SERVICE_URL
  ?? "http://127.0.0.1:8001";
const SERVICE_TIMEOUT_MS = Number(process.env.VOA_SERVICE_TIMEOUT_MS || 180_000);

const stringValue = (value: unknown) => typeof value === "string" ? value.trim() : "";

const validateVoaUrl = (value: unknown) => {
  const url = new URL(stringValue(value));
  const allowedHosts = ["voanews.com", "voanews.eu"];
  const allowed = allowedHosts.some((host) => url.hostname === host || url.hostname.endsWith(`.${host}`));
  if (url.protocol !== "https:" || !allowed) throw new Error("VOA 返回了不受信任的 URL");
  return url.toString();
};

export const normalizePythonArticle = (raw: unknown, fallbackSourceKey = "voa_science"): NormalizedEnglishArticle => {
  if (!raw || typeof raw !== "object") throw new Error("文章数据格式无效");
  const value = raw as Record<string, unknown>;
  const sourceUrl = validateVoaUrl(value.source_url);
  const title = stringValue(value.title);
  const content = stringValue(value.content);
  if (!title || !content) throw new Error("文章缺少标题或正文");
  return {
    source_key: stringValue(value.source_key) || fallbackSourceKey,
    external_id: stringValue(value.external_id) || undefined,
    source_url: sourceUrl,
    title,
    summary: stringValue(value.summary) || undefined,
    content,
    author: stringValue(value.author) || undefined,
    category: stringValue(value.category) || "Life",
    published_at: stringValue(value.published_at) || undefined,
    source_updated_at: stringValue(value.source_updated_at) || undefined,
    audio_url: value.audio_url ? validateVoaUrl(value.audio_url) : undefined,
    image_url: value.image_url ? validateVoaUrl(value.image_url) : undefined,
    license_type: stringValue(value.license_type) || "VOA terms apply",
    attribution: stringValue(value.attribution) || "VOA Learning English",
    metadata: typeof value.metadata === "object" && value.metadata ? value.metadata as Record<string, unknown> : {},
  };
};

async function requestPython(path: string, body: Record<string, unknown>): Promise<PythonFetchResponse> {
  let response: Response;
  try {
    response = await fetch(`${SERVICE_URL}${path}`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(body),
      signal: AbortSignal.timeout(SERVICE_TIMEOUT_MS),
    });
  } catch (error) {
    const timeout = error instanceof Error && error.name === "TimeoutError";
    throw new Error(timeout ? "Python VOA 抓取超时" : "无法连接本机 Python 抓取服务");
  }
  const payload = await response.json().catch(() => ({})) as PythonFetchResponse;
  if (!response.ok) {
    const message = stringValue(payload.detail) || stringValue(payload.error) || `VOA 抓取失败（HTTP ${response.status}）`;
    const wrapped = new Error(message) as Error & { rateLimited?: boolean };
    wrapped.rateLimited = response.status === 429;
    throw wrapped;
  }
  return payload;
}

export class VoaPythonSource implements EnglishContentSource {
  constructor(
    public readonly sourceKey: string,
    private readonly feedKey: string,
  ) {}

  private async fetch(mode: SourceMode, options: SourceFetchOptions): Promise<SourceFetchResult> {
    const payload = await requestPython("/api/voa/articles", {
      sourceKey: this.sourceKey,
      category: this.feedKey,
      mode,
      limit: options.limit,
      overlapDays: options.overlapDays,
      cursor: options.cursor,
      requestIntervalMs: options.requestIntervalMs,
    });
    if (payload.engine !== "python" || !Array.isArray(payload.articles)) {
      throw new Error("Python VOA 抓取结果格式无效");
    }
    const failed = Array.isArray(payload.errors)
      ? payload.errors.map((error) => typeof error === "string" ? { error } : error as { error: string })
      : [];
    const articles: NormalizedEnglishArticle[] = [];
    for (const raw of payload.articles) {
      try {
        articles.push(this.normalizeArticle(raw));
      } catch (error) {
        failed.push({ error: error instanceof Error ? error.message : "文章标准化失败" });
      }
    }
    return {
      articles,
      failed,
      discoveredCount: Number(payload.discovered_count) || payload.articles.length,
      nextCursor: stringValue(payload.next_cursor) || undefined,
      requestCount: Number(payload.request_count) || undefined,
    };
  }

  fetchLatest(options: SourceFetchOptions) {
    return this.fetch("latest", options);
  }

  fetchHistory(options: SourceFetchOptions) {
    return this.fetch("history", options);
  }

  async fetchArticleDetail(url: string) {
    const payload = await requestPython("/api/voa/articles", {
      sourceKey: this.sourceKey,
      category: this.feedKey,
      mode: "detail",
      articleUrl: url,
      limit: 1,
    });
    if (!Array.isArray(payload.articles) || !payload.articles[0]) throw new Error("文章详情抓取失败");
    return this.normalizeArticle(payload.articles[0]);
  }

  normalizeArticle(raw: unknown) {
    return normalizePythonArticle(raw, this.sourceKey);
  }

  async healthCheck(): Promise<SourceHealth> {
    const payload = await requestPython("/api/voa/health", { category: this.feedKey });
    return {
      ok: Boolean(payload.ok),
      rateLimited: Boolean(payload.rate_limited),
      detail: stringValue(payload.detail) || undefined,
    };
  }
}

const CATEGORY_MAP: Record<string, EnglishCategory> = {
  science: "Science",
  health: "Life",
  words: "Culture",
  grammar: "Culture",
  education: "Life",
  Science: "Science",
  Technology: "Technology",
  Life: "Life",
  Health: "Life",
  Culture: "Culture",
  Grammar: "Culture",
  Education: "Life",
};

export const toLegacyEnglishArticle = (
  raw: NormalizedEnglishArticle,
  id: string,
  contentHash: string,
  qualityScore: number,
  status: EnglishArticle["processingStatus"],
  level: EnglishArticle["level"] = "B1",
): EnglishArticle => {
  const wordCount = raw.content.match(/\b[A-Za-z]+(?:['’-][A-Za-z]+)*\b/g)?.length ?? 0;
  const now = new Date().toISOString();
  const publishedAt = raw.published_at && Number.isFinite(new Date(raw.published_at).getTime())
    ? new Date(raw.published_at).toISOString()
    : now;
  return {
    id,
    title: raw.title,
    level,
    category: CATEGORY_MAP[raw.category] ?? "Life",
    difficulty: 3,
    estimatedMinutes: Math.max(5, Math.min(25, Math.ceil(wordCount / 130))),
    content: raw.content,
    vocabulary: [],
    questions: [
      `What is the main idea of "${raw.title}"?`,
      "Which detail best supports the main idea?",
      "What is one new thing you learned?",
    ],
    source: "voa",
    sourceKey: raw.source_key,
    sourceName: "VOA Learning English",
    sourceCategory: raw.category,
    sourceUrl: raw.source_url,
    externalId: raw.external_id,
    publishedAt,
    sourceUpdatedAt: raw.source_updated_at,
    imageUrl: raw.image_url,
    audioUrl: raw.audio_url,
    author: raw.author,
    summary: raw.summary,
    wordCount,
    fetchedAt: now,
    rightsNote: raw.attribution,
    contentHash,
    language: "en",
    qualityScore,
    hasAudio: Boolean(raw.audio_url),
    licenseType: raw.license_type,
    attribution: raw.attribution,
    processingStatus: status,
    fetchStatus: "SUCCESS",
    retryCount: 0,
    createdTime: publishedAt,
    updatedAt: now,
  };
};

// Compatibility entry point retained for older callers and tests.
export async function fetchVoaArticlesFromPython(limitPerFeed = 30) {
  const sources = [
    new VoaPythonSource("voa_science", "science"),
    new VoaPythonSource("voa_health", "health"),
    new VoaPythonSource("voa_words", "words"),
  ];
  const results = await Promise.allSettled(sources.map((source) =>
    source.fetchLatest({ limit: limitPerFeed, overlapDays: 14 }),
  ));
  return {
    articles: results.flatMap((result) => result.status === "fulfilled" ? result.value.articles : []),
    skipped: 0,
    failed: results.reduce((count, result) => count + (result.status === "rejected" ? 1 : result.value.failed.length), 0),
  };
}

export const voaSyncResult = (
  imported: number,
  skipped: number,
  failed: number,
  cached: boolean,
): EnglishSourceSyncResult => ({
  source: "voa",
  engine: "python",
  imported,
  skipped,
  failed,
  syncedAt: new Date().toISOString(),
  cached,
});
