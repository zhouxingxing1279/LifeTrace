import { vocabularyStats } from "@/src/server/vocabulary/repository";
export async function GET() {
  try { return Response.json(await vocabularyStats()); }
  catch (error) { return Response.json({ error: error instanceof Error ? error.message : "生词统计读取失败" }, { status: 500 }); }
}
