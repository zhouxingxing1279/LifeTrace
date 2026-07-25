import { assistantInsight } from "@/src/server/englishRepository";

// GET /api/english/assistant：聚合最近十次 AI 分析，生成阶段性学习建议。
export async function GET() {
  try {
    return Response.json(await assistantInsight());
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "学习建议生成失败" }, { status: 500 });
  }
}
