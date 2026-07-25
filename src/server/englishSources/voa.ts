import type {
  CEFRLevel,
  EnglishArticle,
  EnglishCategory,
  EnglishSourceSyncResult,
} from "@/src/types/english";

type FeedDefinition = {
  url: string;
  category: EnglishCategory;
  level: CEFRLevel;
};

type FeedItem = {
  title: string;
  link: string;
  guid?: string;
  publishedAt: string;
  summary?: string;
  audioUrl?: string;
  imageUrl?: string;
};

type JsonLdRecord = Record<string, unknown>;

const VOA_HOST = "learningenglish.voanews.com";
const VOA_SOURCE_NAME = "VOA Learning English";
const FETCH_ATTEMPTS = 3;
const RETRYABLE_STATUS = new Set([429, 500, 502, 503, 504]);
const REQUEST_HEADERS = {
  accept: "application/rss+xml, application/xml, text/xml, text/html;q=0.9",
  "accept-language": "en-US,en;q=0.9",
  "user-agent": "LifeTrace/1.0 (+personal English learning reader)",
};

// Worker 兼容版实现移植自用户提供的 fetch_voa_articles.py。
// 官方 RSS 入口均来自 VOA Learning English 的 RSS 订阅页。
const VOA_FEEDS: FeedDefinition[] = [
  { url: "https://learningenglish.voanews.com/api/zbmroml-vomx-tpeqboo_", category: "Culture", level: "B1" },
  { url: "https://learningenglish.voanews.com/api/zkm-ql-vomx-tpej-rqi", category: "Life", level: "B1" },
  { url: "https://learningenglish.voanews.com/api/zmmpql-vomx-tpey-_q", category: "Life", level: "B1" },
  { url: "https://learningenglish.voanews.com/api/zmg_pl-vomx-tpeymtm", category: "Science", level: "B1" },
  { url: "https://learningenglish.voanews.com/api/zmypyl-vomx-tpeyry_", category: "Culture", level: "B1" },
];

const decodeEntities = (value: string) => value
  .replace(/<!\[CDATA\[([\s\S]*?)\]\]>/g, "$1")
  .replace(/&nbsp;/gi, " ")
  .replace(/&amp;/gi, "&")
  .replace(/&quot;/gi, "\"")
  .replace(/&#39;|&apos;/gi, "'")
  .replace(/&lt;/gi, "<")
  .replace(/&gt;/gi, ">")
  .replace(/&#(\d+);/g, (_, code: string) => String.fromCodePoint(Number(code)))
  .replace(/&#x([\da-f]+);/gi, (_, code: string) => String.fromCodePoint(Number.parseInt(code, 16)));

const plainText = (value: string) => decodeEntities(value)
  .replace(/<br\s*\/?>/gi, "\n")
  .replace(/<[^>]+>/g, " ")
  .replace(/\s+/g, " ")
  .trim();

const tagValue = (xml: string, tag: string) => {
  const match = xml.match(new RegExp(`<(?:[\\w-]+:)?${tag}(?:\\s[^>]*)?>([\\s\\S]*?)<\\/(?:[\\w-]+:)?${tag}>`, "i"));
  return match ? plainText(match[1]) : "";
};

const attributeValue = (tag: string, attribute: string) =>
  decodeEntities(tag.match(new RegExp(`\\b${attribute}\\s*=\\s*["']([^"']+)["']`, "i"))?.[1] ?? "");

const safeVoaUrl = (value: string) => {
  try {
    const url = new URL(value);
    return url.protocol === "https:" && url.hostname === VOA_HOST ? url.toString() : "";
  } catch {
    return "";
  }
};

const safeVoaAssetUrl = (value: string | undefined, baseUrl: string) => {
  if (!value) return undefined;
  try {
    const url = new URL(decodeEntities(value), baseUrl);
    const voaHost = url.hostname === "voanews.com" || url.hostname.endsWith(".voanews.com");
    return url.protocol === "https:" && voaHost ? url.toString() : undefined;
  } catch {
    return undefined;
  }
};

const parseFeed = (xml: string): FeedItem[] => [...xml.matchAll(/<item(?:\s[^>]*)?>([\s\S]*?)<\/item>/gi)]
  .map((match) => {
    const item = match[1];
    let audioUrl: string | undefined;
    let imageUrl: string | undefined;
    for (const mediaTag of item.match(/<(?:enclosure|media:content)\b[^>]*>/gi) ?? []) {
      const url = attributeValue(mediaTag, "url");
      const type = attributeValue(mediaTag, "type").toLowerCase();
      const cleanUrl = url.toLowerCase().split("?", 1)[0];
      if (type.startsWith("audio/") || /\.(?:mp3|m4a|ogg)$/.test(cleanUrl)) {
        audioUrl ??= safeVoaAssetUrl(url, `https://${VOA_HOST}`);
      } else if (type.startsWith("image/") || /\.(?:jpe?g|png|webp)$/.test(cleanUrl)) {
        imageUrl ??= safeVoaAssetUrl(url, `https://${VOA_HOST}`);
      }
    }
    return {
      title: tagValue(item, "title"),
      link: safeVoaUrl(tagValue(item, "link")),
      guid: tagValue(item, "guid") || undefined,
      publishedAt: tagValue(item, "pubDate"),
      summary: tagValue(item, "description") || undefined,
      audioUrl,
      imageUrl,
    };
  })
  .filter((item) => Boolean(item.title && item.link));

const jsonLdObjects = (value: unknown): JsonLdRecord[] => {
  if (Array.isArray(value)) return value.flatMap(jsonLdObjects);
  if (!value || typeof value !== "object") return [];
  const record = value as JsonLdRecord;
  return [record, ...jsonLdObjects(record["@graph"])];
};

const parseArticleJsonLd = (html: string): JsonLdRecord => {
  let fallback: JsonLdRecord = {};
  const scripts = html.matchAll(/<script[^>]+type=["']application\/ld\+json["'][^>]*>([\s\S]*?)<\/script>/gi);
  for (const match of scripts) {
    try {
      const parsed = JSON.parse(match[1].replace(/^\s*<!--|-->\s*$/g, "").trim()) as unknown;
      for (const record of jsonLdObjects(parsed)) {
        const rawType = record["@type"];
        const types = (Array.isArray(rawType) ? rawType : [rawType])
          .filter((item): item is string => typeof item === "string")
          .map((item) => item.toLowerCase());
        if (types.some((item) => ["article", "newsarticle", "reportagenewsarticle"].includes(item))) return record;
        if (!Object.keys(fallback).length && ["headline", "articleBody", "datePublished"].some((key) => key in record)) {
          fallback = record;
        }
      }
    } catch {
      // A malformed JSON-LD block must not prevent parsing the rest of the page.
    }
  }
  return fallback;
};

const stringValue = (value: unknown) => typeof value === "string" ? value.trim() : "";

const metaContent = (html: string, names: string[]) => {
  for (const tag of html.match(/<meta\b[^>]*>/gi) ?? []) {
    const name = attributeValue(tag, "name") || attributeValue(tag, "property");
    if (names.includes(name.toLowerCase())) {
      const content = attributeValue(tag, "content");
      if (content) return plainText(content);
    }
  }
  return "";
};

const authorFromJsonLd = (value: unknown): string | undefined => {
  const authors = Array.isArray(value) ? value : [value];
  const names = authors.flatMap((author) => {
    if (typeof author === "string") return [author.trim()];
    if (author && typeof author === "object") return [stringValue((author as JsonLdRecord).name)];
    return [];
  }).filter(Boolean);
  return names.length ? names.join(", ") : undefined;
};

const mediaFromJsonLd = (value: unknown): string | undefined => {
  const values = Array.isArray(value) ? value : [value];
  for (const item of values) {
    if (typeof item === "string" && item.trim()) return item.trim();
    if (item && typeof item === "object") {
      const record = item as JsonLdRecord;
      const candidate = stringValue(record.contentUrl) || stringValue(record.embedUrl) || stringValue(record.url);
      if (candidate) return candidate;
    }
  }
  return undefined;
};

const isVoaOwnedArticle = (html: string) => {
  const copied = html.match(/copied:\s*"([^"]+)"/i)?.[1]?.toLowerCase();
  const agencyCredit = /\b(?:Associated Press|Agence France-Presse|Reuters|AFP)\b/i.test(
    html.match(/(?:byline|copied_title):"[^"]*"/gi)?.join(" ") ?? "",
  );
  return copied === "no" && !agencyCredit;
};

const isBoilerplate = (paragraph: string) =>
  /^(?:This story was|Learn English|Subscribe|Download|Click here|Embed|Share|Follow us|Breaking News)/i.test(paragraph)
  || /^(?:(?:0:00|\d{1,2}:\d{2}(?::\d{2})?)\s*)+$/.test(paragraph)
  || /^(?:128|64) kbps(?:\s*\|\s*MP3)?$/i.test(paragraph);

const htmlArticleBody = (html: string) => {
  const start = html.search(/<div class=["'][^"']*\bwsw\b[^"']*["'][^>]*>/i);
  if (start < 0) return "";
  const endMarker = html.indexOf('<div class="article-categories"', start);
  let fragment = html.slice(start, endMarker > start ? endMarker : undefined);
  fragment = fragment.split(/<h2[^>]*>\s*Words in This Story\s*<\/h2>/i)[0];
  fragment = fragment
    .replace(/<(?:script|style|figure|nav|aside|footer)\b[\s\S]*?<\/(?:script|style|figure|nav|aside|footer)>/gi, "")
    .replace(/<div class=["'][^"']*(?:embed|share|social|related|caption|media)[^"']*["'][\s\S]*?<\/div>/gi, "");
  const paragraphs = [...fragment.matchAll(/<(?:p|h2|h3|blockquote)(?:\s[^>]*)?>([\s\S]*?)<\/(?:p|h2|h3|blockquote)>/gi)]
    .map((match) => plainText(match[1]))
    .filter((paragraph) => paragraph.length >= 20 && !isBoilerplate(paragraph));
  return [...new Set(paragraphs)].join("\n\n");
};

const articleBody = (html: string, jsonLd: JsonLdRecord) => {
  const structuredBody = stringValue(jsonLd.articleBody)
    .replace(/\r\n?/g, "\n")
    .replace(/\n{3,}/g, "\n\n");
  if (structuredBody.split(/\s+/).filter(Boolean).length >= 80) return decodeEntities(structuredBody);
  return htmlArticleBody(html);
};

const audioFromHtml = (html: string, jsonLd: JsonLdRecord, pageUrl: string) => {
  const structured = mediaFromJsonLd(jsonLd.audio) || mediaFromJsonLd(jsonLd.associatedMedia);
  const meta = metaContent(html, ["og:audio", "og:audio:url", "twitter:player:stream"]);
  const tagged = (html.match(/<(?:audio|source|a)\b[^>]*(?:src|href)=["'][^"']+["'][^>]*>/gi) ?? [])
    .map((tag) => attributeValue(tag, "src") || attributeValue(tag, "href"))
    .find((value) => /\.(?:mp3|m4a|ogg)(?:$|\?)/i.test(value));
  return safeVoaAssetUrl(structured || meta || tagged, pageUrl);
};

const externalIdFromUrl = (value: string) => {
  const id = value.match(/\/(\d+)\.html(?:$|\?)/)?.[1];
  return id ?? "";
};

const readingMinutes = (content: string) =>
  Math.max(5, Math.min(25, Math.ceil(content.split(/\s+/).filter(Boolean).length / 130)));

const comprehensionQuestions = (title: string) => [
  `What is the main idea of “${title}”?`,
  "Which detail in the article best supports its main idea?",
  "What is one new thing you learned from this article?",
];

const sleep = (milliseconds: number) => new Promise((resolve) => setTimeout(resolve, milliseconds));

class VoaResponseError extends Error {
  constructor(readonly status: number) {
    super(`VOA 请求失败（${status}）`);
  }
}

async function fetchText(url: string) {
  let lastError: unknown;
  for (let attempt = 0; attempt < FETCH_ATTEMPTS; attempt += 1) {
    try {
      const response = await fetch(url, { headers: REQUEST_HEADERS, signal: AbortSignal.timeout(20_000) });
      if (response.ok) return response.text();
      if (!RETRYABLE_STATUS.has(response.status)) throw new VoaResponseError(response.status);
      lastError = new VoaResponseError(response.status);
      const retryAfter = Number(response.headers.get("retry-after"));
      if (attempt < FETCH_ATTEMPTS - 1) {
        await sleep(Number.isFinite(retryAfter) ? Math.min(retryAfter * 1_000, 8_000) : 800 * (2 ** attempt));
      }
    } catch (error) {
      if (error instanceof VoaResponseError && !RETRYABLE_STATUS.has(error.status)) throw error;
      lastError = error;
      if (attempt < FETCH_ATTEMPTS - 1) await sleep(800 * (2 ** attempt));
    }
  }
  throw lastError instanceof Error ? lastError : new Error("VOA 请求失败");
}

async function fetchArticle(item: FeedItem, feed: FeedDefinition): Promise<EnglishArticle | null> {
  const html = await fetchText(item.link);
  if (!isVoaOwnedArticle(html)) return null;
  const jsonLd = parseArticleJsonLd(html);
  const content = articleBody(html, jsonLd);
  const wordCount = content.match(/\b[\w'-]+\b/g)?.length ?? 0;
  const externalId = externalIdFromUrl(item.link);
  if (!externalId || wordCount < 50) return null;

  const syncedAt = new Date().toISOString();
  const rawPublishedAt = stringValue(jsonLd.datePublished)
    || metaContent(html, ["article:published_time", "date"])
    || item.publishedAt;
  const published = new Date(rawPublishedAt);
  const publishedAt = Number.isNaN(published.getTime()) ? syncedAt : published.toISOString();
  const title = plainText(
    stringValue(jsonLd.headline)
    || metaContent(html, ["og:title", "twitter:title"])
    || item.title,
  );
  const summary = plainText(
    stringValue(jsonLd.description)
    || metaContent(html, ["description", "og:description"])
    || item.summary
    || "",
  ) || undefined;
  const imageUrl = safeVoaAssetUrl(
    mediaFromJsonLd(jsonLd.image) || metaContent(html, ["og:image"]) || item.imageUrl,
    item.link,
  );
  const audioUrl = audioFromHtml(html, jsonLd, item.link) ?? item.audioUrl;

  return {
    id: `voa-${externalId}`,
    title,
    level: feed.level,
    category: feed.category,
    difficulty: 3,
    estimatedMinutes: readingMinutes(content),
    content,
    vocabulary: [],
    questions: comprehensionQuestions(title),
    source: "voa",
    sourceName: VOA_SOURCE_NAME,
    sourceUrl: item.link,
    externalId,
    publishedAt,
    imageUrl,
    audioUrl,
    author: authorFromJsonLd(jsonLd.author) || metaContent(html, ["author"]) || undefined,
    summary,
    wordCount,
    fetchedAt: syncedAt,
    rightsNote: "VOA 自有内容可按官方说明署名使用；第三方文字、图片、音频或视频需另行确认授权。",
    createdTime: publishedAt,
    updatedAt: syncedAt,
  };
}

export async function fetchVoaArticles(limitPerFeed = 2) {
  const feedResults = await Promise.allSettled(VOA_FEEDS.map(async (feed) => ({
    feed,
    items: parseFeed(await fetchText(feed.url)).slice(0, limitPerFeed),
  })));
  const candidates = feedResults.flatMap((result) => result.status === "fulfilled"
    ? result.value.items.map((item) => ({ item, feed: result.value.feed }))
    : []);
  const unique = [...new Map(candidates.map((candidate) => [candidate.item.link, candidate])).values()];
  const articleResults: PromiseSettledResult<EnglishArticle | null>[] = [];
  // VOA 会限制短时间内的大量页面请求，按两个一组抓取更稳定。
  for (let index = 0; index < unique.length; index += 2) {
    articleResults.push(...await Promise.allSettled(
      unique.slice(index, index + 2).map(({ item, feed }) => fetchArticle(item, feed)),
    ));
  }
  const articles = articleResults.flatMap((result) =>
    result.status === "fulfilled" && result.value ? [result.value] : []);
  return {
    articles,
    skipped: articleResults.filter((result) => result.status === "fulfilled" && !result.value).length,
    failed: feedResults.filter((result) => result.status === "rejected").length
      + articleResults.filter((result) => result.status === "rejected").length,
  };
}

export const voaSyncResult = (
  imported: number,
  skipped: number,
  failed: number,
  cached: boolean,
): EnglishSourceSyncResult => ({
  source: "voa",
  imported,
  skipped,
  failed,
  syncedAt: new Date().toISOString(),
  cached,
});
