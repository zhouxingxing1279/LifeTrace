import { getActiveTask, getTask, listTasks } from "@/src/server/englishSync/storage";

export async function GET(request: Request) {
  try {
    const url = new URL(request.url);
    const taskId = url.searchParams.get("taskId");
    if (taskId) {
      const task = await getTask(taskId);
      return task ? Response.json({ task }) : Response.json({ error: "同步任务不存在" }, { status: 404 });
    }
    return Response.json({ activeTask: await getActiveTask(), tasks: await listTasks(20) });
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "同步状态读取失败" }, { status: 500 });
  }
}
