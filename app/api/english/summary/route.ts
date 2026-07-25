import { saveSummary } from "@/src/server/englishRepository";

// POST /api/english/summary：保存英文总结和本次阅读时长。
export async function POST(request: Request) {
  try {
    const body = await request.json() as { articleId?: string; summary?: string; readingTimeSeconds?: number; recordId?: string };
    const summary = body.summary?.trim() ?? "";
    if (!body.articleId || !summary) return Response.json({ error: "文章和英文总结不能为空" }, { status: 400 });
    if (/[\u3400-\u9fff]/.test(summary)) return Response.json({ error: "总结必须使用英文书写" }, { status: 400 });
    if (summary.split(/\s+/).filter(Boolean).length < 20) return Response.json({ error: "总结至少需要 20 个英文单词" }, { status: 400 });
    return Response.json(await saveSummary({ articleId: body.articleId, summary, readingTimeSeconds: body.readingTimeSeconds, recordId: body.recordId }));
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "总结保存失败" }, { status: 500 });
  }
}
