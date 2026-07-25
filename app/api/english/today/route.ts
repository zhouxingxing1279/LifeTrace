import { getTodayEnglish, LEVELS } from "@/src/server/englishRepository";
import type { CEFRLevel } from "@/src/types/english";

// GET /api/english/today：按用户等级返回今日推荐、连续天数和本周进度。
export async function GET(request: Request) {
  try {
    const url = new URL(request.url);
    const levelValue = url.searchParams.get("level");
    const level = LEVELS.includes(levelValue as CEFRLevel) ? levelValue as CEFRLevel : undefined;
    return Response.json(await getTodayEnglish(level, url.searchParams.get("articleId") ?? undefined));
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "今日英语任务读取失败" }, { status: 500 });
  }
}
