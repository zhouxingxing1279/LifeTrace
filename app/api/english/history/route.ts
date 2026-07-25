import { getEnglishHistory } from "@/src/server/englishRepository";

// GET /api/english/history：返回学习记录及最近 30 天聚合指标。
export async function GET() {
  try {
    return Response.json(await getEnglishHistory());
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "学习历史读取失败" }, { status: 500 });
  }
}
