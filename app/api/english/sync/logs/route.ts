import { listLogs } from "@/src/server/englishSync/storage";

export async function GET(request: Request) {
  try {
    const url = new URL(request.url);
    const taskId = url.searchParams.get("taskId") ?? undefined;
    const limit = Number(url.searchParams.get("limit") || 100);
    return Response.json({ logs: await listLogs(taskId, Math.max(1, Math.min(limit, 200))) });
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "同步日志读取失败" }, { status: 500 });
  }
}
