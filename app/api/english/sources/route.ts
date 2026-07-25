import { listSourceStates } from "@/src/server/englishSync/storage";

export async function GET() {
  try {
    return Response.json({ sources: await listSourceStates() });
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "数据源状态读取失败" }, { status: 500 });
  }
}
