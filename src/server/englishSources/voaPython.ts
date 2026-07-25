import type {
  EnglishArticle,
  EnglishCategory,
  EnglishSourceSyncResult,
} from "@/src/types/english";

type PythonArticle = {
  source_url?: unknown;
  title?: unknown;
  category?: unknown;
  author?: unknown;
  published_at?: unknown;
  summary?: unknown;
  content?: unknown;
  word_count?: unknown;
  audio_url?: unknown;
  image_url?: unknown;
  fetched_at?: unknown;
  rights_note?: unknown;
};

type PythonFetchResponse = {
  engine?: unknown;
  articles?: unknown;
  skipped?: unknown;
  failed?: unknown;
  detail?: unknown;
  error?: unknown;
};

const SERVICE_URL = process.env.VOA_SERVICE_URL
  ?? process.env.XUNJI_SERVICE_URL
  ?? "http://127.0.0.1:8001";
const SERVICE_TIMEOUT_MS = 50_000;
const VOA_HOST = "learningenglish.voanews.com";
const CATEGORY_MAP: Record<string, EnglishCategory> = {
  science: "Science",
  health: "Life",
  words: "Culture",
};

const stringValue = (value: unknown) => typeof value === "string" ? value.trim() : "";

const safeSourceUrl = (value: unknown) => {
  try {
    const url = new URL(stringValue(value));
    return url.protocol === "https:" && url.hostname === VOA_HOST ? url.toString() : "";
  } catch {
    return "";
  }
};

const safeAssetUrl = (value: unknown) => {
  try {
    const url = new URL(stringValue(value));
    const voaHost = url.hostname === "voanews.com" || url.hostname.endsWith(".voanews.com");
    return url.protocol === "https:" && voaHost ? url.toString() : undefined;
  } catch {
    return undefined;
  }
};

const readingMinutes = (wordCount: number) =>
  Math.max(5, Math.min(25, Math.ceil(wordCount / 130)));

const questions = (title: string) => [
  `What is the main idea of "${title}"?`,
  "Which detail in the article best supports its main idea?",
  "What is one new thing you learned from this article?",
];

const toEnglishArticle = (raw: PythonArticle): EnglishArticle | null => {
  const sourceUrl = safeSourceUrl(raw.source_url);
  const externalId = sourceUrl.match(/\/(\d+)\.html(?:$|\?)/)?.[1] ?? "";
  const title = stringValue(raw.title);
  const content = stringValue(raw.content);
  const measuredWords = content.match(/\b[\w'-]+\b/g)?.length ?? 0;
  const reportedWords = Number(raw.word_count);
  const wordCount = Number.isFinite(reportedWords) && reportedWords > 0 ? Math.round(reportedWords) : measuredWords;
  if (!sourceUrl || !externalId || !title || wordCount < 50) return null;

  const now = new Date().toISOString();
  const published = new Date(stringValue(raw.published_at));
  const publishedAt = Number.isNaN(published.getTime()) ? now : published.toISOString();
  const category = CATEGORY_MAP[stringValue(raw.category).toLowerCase()] ?? "Life";
  return {
    id: `voa-${externalId}`,
    title,
    level: "B1",
    category,
    difficulty: 3,
    estimatedMinutes: readingMinutes(wordCount),
    content,
    vocabulary: [],
    questions: questions(title),
    source: "voa",
    sourceName: "VOA Learning English",
    sourceUrl,
    externalId,
    publishedAt,
    imageUrl: safeAssetUrl(raw.image_url),
    audioUrl: safeAssetUrl(raw.audio_url),
    author: stringValue(raw.author) || undefined,
    summary: stringValue(raw.summary) || undefined,
    wordCount,
    fetchedAt: stringValue(raw.fetched_at) || now,
    rightsNote: stringValue(raw.rights_note) || undefined,
    createdTime: publishedAt,
    updatedAt: now,
  };
};

export async function fetchVoaArticlesFromPython(limitPerFeed = 2) {
  let response: Response;
  try {
    response = await fetch(`${SERVICE_URL}/api/voa/articles`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ limitPerFeed }),
      signal: AbortSignal.timeout(SERVICE_TIMEOUT_MS),
    });
  } catch (error) {
    const timeout = error instanceof Error && error.name === "TimeoutError";
    throw new Error(timeout
      ? "Python VOA 抓取超过 50 秒，已停止本次同步"
      : "无法连接本机 Python 抓取服务，请重启 Life trace 后再试");
  }

  const payload = await response.json().catch(() => ({})) as PythonFetchResponse;
  if (!response.ok) {
    if (response.status === 404) {
      throw new Error("Python 服务版本过旧，请重启 Life trace 后再同步");
    }
    const message = stringValue(payload.detail) || stringValue(payload.error) || "Python VOA 抓取失败";
    throw new Error(message);
  }
  if (payload.engine !== "python" || !Array.isArray(payload.articles)) {
    throw new Error("Python VOA 抓取结果格式无效");
  }
  const articles = payload.articles
    .map((article) => toEnglishArticle(article as PythonArticle))
    .filter((article): article is EnglishArticle => Boolean(article));
  return {
    articles,
    skipped: Math.max(0, Number(payload.skipped) || 0) + (payload.articles.length - articles.length),
    failed: Math.max(0, Number(payload.failed) || 0),
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
