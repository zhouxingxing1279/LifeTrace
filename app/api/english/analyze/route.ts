import { analyzeSummary, LEVELS } from "@/src/server/englishRepository";
import type { CEFRLevel } from "@/src/types/english";

// POST /api/english/analyze：当前调用 Mock 服务，未来可在服务层替换为 DeepSeek。
export async function POST(request: Request) {
  try {
    const body = await request.json() as { recordId?: string; userLevel?: CEFRLevel };
    if (!body.recordId) return Response.json({ error: "缺少学习记录" }, { status: 400 });
    const level = LEVELS.includes(body.userLevel as CEFRLevel) ? body.userLevel as CEFRLevel : "B1";
    return Response.json(await analyzeSummary(body.recordId, level));
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "AI 分析失败" }, { status: 500 });
  }
}
