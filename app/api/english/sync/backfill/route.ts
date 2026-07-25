import { after } from "next/server";
import { runEnglishSyncTask, scheduleEnglishSync } from "@/src/server/englishSync/service";

export async function POST(request: Request) {
  try {
    const body = await request.json().catch(() => ({})) as { sourceKey?: string; force?: boolean; limit?: number };
    const result = await scheduleEnglishSync("backfill", body.sourceKey, Boolean(body.force), body.limit);
    if (result.created && result.task) after(() => runEnglishSyncTask(result.task!.taskId));
    return Response.json({ ...result, taskId: result.task?.taskId }, { status: result.created ? 202 : 200 });
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "文章库初始化失败" }, { status: 500 });
  }
}
