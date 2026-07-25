import { after } from "next/server";
import { runEnglishSyncTask, scheduleEnglishSync } from "@/src/server/englishSync/service";

export async function POST(_request: Request, context: { params: Promise<{ sourceKey: string }> }) {
  try {
    const { sourceKey } = await context.params;
    const result = await scheduleEnglishSync("incremental", sourceKey, true);
    if (result.created && result.task) after(() => runEnglishSyncTask(result.task!.taskId));
    return Response.json({ ...result, taskId: result.task?.taskId }, { status: result.created ? 202 : 200 });
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "数据源同步失败" }, { status: 500 });
  }
}
