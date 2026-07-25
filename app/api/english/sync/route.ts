import { after } from "next/server";
import { runEnglishSyncTask, scheduleEnglishSync, shouldRunStartupSync } from "@/src/server/englishSync/service";

export async function POST(request: Request) {
  try {
    const body = await request.json().catch(() => ({})) as { force?: boolean; startupCheck?: boolean };
    if (body.startupCheck && !(await shouldRunStartupSync())) {
      return Response.json({ created: false, cached: true, message: "尚未达到同步间隔" });
    }
    const result = await scheduleEnglishSync("incremental", undefined, Boolean(body.force));
    if (result.created && result.task) after(() => runEnglishSyncTask(result.task!.taskId));
    return Response.json({ ...result, taskId: result.task?.taskId }, { status: result.created ? 202 : 200 });
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "英语文章同步失败" }, { status: 500 });
  }
}
