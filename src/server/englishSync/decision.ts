import { calculateContentHash, normalizeContent, normalizeUrl } from "./normalize";
import type { NormalizedEnglishArticle } from "./source";

export type ExistingArticleIdentity = {
  id: string;
  sourceKey?: string;
  externalId?: string;
  normalizedSourceUrl?: string;
  contentHash?: string;
  title?: string;
  publishedAt?: string;
};

export type ArticleDecision = {
  action: "insert" | "update" | "skip" | "duplicate_content";
  contentHash: string;
  normalizedSourceUrl: string;
  existingId?: string;
};

export async function decideArticle(
  incoming: NormalizedEnglishArticle,
  existing: ExistingArticleIdentity[],
): Promise<ArticleDecision> {
  const normalizedSourceUrl = normalizeUrl(incoming.source_url);
  const contentHash = await calculateContentHash(incoming.content);
  const sameIdentity = existing.find((article) =>
    Boolean(incoming.external_id)
      && article.sourceKey === incoming.source_key
      && article.externalId === incoming.external_id,
  ) ?? existing.find((article) => article.normalizedSourceUrl === normalizedSourceUrl);

  if (sameIdentity) {
    return {
      action: sameIdentity.contentHash === contentHash ? "skip" : "update",
      contentHash,
      normalizedSourceUrl,
      existingId: sameIdentity.id,
    };
  }
  const sameContent = existing.find((article) => article.contentHash === contentHash);
  if (sameContent) {
    return { action: "duplicate_content", contentHash, normalizedSourceUrl, existingId: sameContent.id };
  }
  const auxiliary = existing.find((article) =>
    article.title === incoming.title && article.publishedAt === incoming.published_at,
  );
  if (auxiliary && normalizeContent(incoming.content).length === 0) {
    return { action: "skip", contentHash, normalizedSourceUrl, existingId: auxiliary.id };
  }
  return { action: "insert", contentHash, normalizedSourceUrl };
}

export function sourceStatusAfterSync(input: {
  now: Date;
  lastNewArticleAt?: string;
  consecutiveFailures: number;
  requestSucceeded: boolean;
  rateLimited?: boolean;
  staleAfterDays: number;
  errorFailureThreshold: number;
}) {
  if (input.rateLimited) return "rate_limited" as const;
  if (!input.requestSucceeded && input.consecutiveFailures >= input.errorFailureThreshold) return "error" as const;
  if (!input.requestSucceeded) return "active" as const;
  if (input.lastNewArticleAt) {
    const age = input.now.getTime() - new Date(input.lastNewArticleAt).getTime();
    if (age >= input.staleAfterDays * 86_400_000) return "stale" as const;
  }
  return "active" as const;
}
