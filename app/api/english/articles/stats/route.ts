import { getLibraryStats } from "@/src/server/englishSync/storage";

export async function GET() {
  try {
    return Response.json(await getLibraryStats());
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "文章库统计读取失败" }, { status: 500 });
  }
}
