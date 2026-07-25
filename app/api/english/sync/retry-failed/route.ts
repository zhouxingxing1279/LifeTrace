import { after } from "next/server";
import { runEnglishSyncTask, scheduleEnglishSync } from "@/src/server/englishSync/service";

export async function POST(request: Request) {
  try {
    const body = await request.json().catch(() => ({})) as { sourceKey?: string };
    const result = await scheduleEnglishSync("retry_failed", body.sourceKey, true);
    if (result.created && result.task) after(() => runEnglishSyncTask(result.task!.taskId));
    return Response.json({ ...result, taskId: result.task?.taskId }, { status: result.created ? 202 : 200 });
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "失败文章重试任务创建失败" }, { status: 500 });
  }
}
