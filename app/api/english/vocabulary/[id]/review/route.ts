import { reviewUserVocabulary } from "@/src/server/vocabulary/repository";
import type { VocabularyReviewResult } from "@/src/types/english";

const RESULTS: VocabularyReviewResult[] = ["FORGOT", "HARD", "GOOD", "EASY"];
export async function POST(request: Request, context: { params: Promise<{ id: string }> }) {
  try {
    const { id } = await context.params;
    const body = await request.json() as { result?: VocabularyReviewResult; responseTimeMs?: number };
    if (!body.result || !RESULTS.includes(body.result)) return Response.json({ error: "无效的复习结果" }, { status: 400 });
    return Response.json(await reviewUserVocabulary(id, body.result, body.responseTimeMs));
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "复习记录保存失败" }, { status: 500 });
  }
}
