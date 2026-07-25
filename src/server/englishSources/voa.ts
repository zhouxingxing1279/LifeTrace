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
  publishedAt: string;
  imageUrl?: string;
};

const VOA_HOST = "learningenglish.voanews.com";
const VOA_SOURCE_NAME = "VOA Learning English";
const REQUEST_HEADERS = {
  accept: "application/rss+xml, application/xml, text/xml, text/html;q=0.9",
  "user-agent": "LifeTrace/1.0 (+personal English learning reader)",
};

// 官方 RSS 入口均来自 VOA Learning English 的 RSS 订阅页。
const VOA_FEEDS: FeedDefinition[] = [
  { url: "https://learningenglish.voanews.com/api/zbmroml-vomx-tpeqboo_", category: "Culture", level: "B1" },
  { url: "https://learningenglish.voanews.com/api/zkm-ql-vomx-tpej-rqi", category: "Life", level: "B1" },
  { url: "https://learningenglish.voanews.com/api/zmmpql-vomx-tpey-_q", category: "Life", level: "B1" },
  { url: "https://learningenglish.voanews.com/api/zmg_pl-vomx-tpeymtm", category: "Science", level: "B1" },
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
  const match = xml.match(new RegExp(`<${tag}(?:\\s[^>]*)?>([\\s\\S]*?)<\\/${tag}>`, "i"));
  return match ? plainText(match[1]) : "";
};

const safeVoaUrl = (value: string) => {
  try {
    const url = new URL(value);
    return url.protocol === "https:" && url.hostname === VOA_HOST ? url.toString() : "";
  } catch {
    return "";
  }
};

const parseFeed = (xml: string): FeedItem[] => [...xml.matchAll(/<item>([\s\S]*?)<\/item>/gi)]
  .map((match) => {
    const item = match[1];
    const enclosure = item.match(/<enclosure[^>]+url=["']([^"']+)["']/i)?.[1];
    return {
      title: tagValue(item, "title"),
      link: safeVoaUrl(tagValue(item, "link")),
      publishedAt: tagValue(item, "pubDate"),
      imageUrl: enclosure?.startsWith("https://") ? decodeEntities(enclosure) : undefined,
    };
  })
  .filter((item) => Boolean(item.title && item.link));

const isVoaOwnedArticle = (html: string) => {
  const copied = html.match(/copied:\s*"([^"]+)"/i)?.[1]?.toLowerCase();
  const agencyCredit = /\b(?:Associated Press|Agence France-Presse|Reuters|AFP)\b/i.test(
    html.match(/(?:byline|copied_title):"[^"]*"/gi)?.join(" ") ?? "",
  );
  return copied === "no" && !agencyCredit;
};

const articleBody = (html: string) => {
  const start = html.search(/<div class=["']wsw["']>/i);
  if (start < 0) return "";
  const endMarker = html.indexOf('<div class="article-categories"', start);
  let fragment = html.slice(start, endMarker > start ? endMarker : undefined);
  fragment = fragment.split(/<h2[^>]*>\s*Words in This Story\s*<\/h2>/i)[0];
  fragment = fragment
    .replace(/<script[\s\S]*?<\/script>/gi, "")
    .replace(/<style[\s\S]*?<\/style>/gi, "")
    .replace(/<figure[\s\S]*?<\/figure>/gi, "")
    .replace(/<div class=["']wsw__embed["'][\s\S]*?<\/div>/gi, "");
  const paragraphs = [...fragment.matchAll(/<p(?:\s[^>]*)?>([\s\S]*?)<\/p>/gi)]
    .map((match) => plainText(match[1]))
    .filter((paragraph) =>
      paragraph.length >= 45
      && !/^_{5,}$/.test(paragraph)
      && !/^(?:This story was|Learn English|Subscribe|Download|Click here)/i.test(paragraph),
    );
  return [...new Set(paragraphs)].join("\n\n");
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

async function fetchText(url: string) {
  const response = await fetch(url, { headers: REQUEST_HEADERS, signal: AbortSignal.timeout(20_000) });
  if (!response.ok) throw new Error(`VOA 请求失败（${response.status}）`);
  return response.text();
}

async function fetchArticle(item: FeedItem, feed: FeedDefinition): Promise<EnglishArticle | null> {
  const html = await fetchText(item.link);
  if (!isVoaOwnedArticle(html)) return null;
  const content = articleBody(html);
  const externalId = externalIdFromUrl(item.link);
  if (!externalId || content.length < 300) return null;
  const syncedAt = new Date().toISOString();
  const published = new Date(item.publishedAt);
  const publishedAt = Number.isNaN(published.getTime()) ? syncedAt : published.toISOString();
  return {
    id: `voa-${externalId}`,
    title: item.title.trim(),
    level: feed.level,
    category: feed.category,
    difficulty: 3,
    estimatedMinutes: readingMinutes(content),
    content,
    vocabulary: [],
    questions: comprehensionQuestions(item.title.trim()),
    source: "voa",
    sourceName: VOA_SOURCE_NAME,
    sourceUrl: item.link,
    externalId,
    publishedAt,
    imageUrl: item.imageUrl,
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
