import { syncVoaArticles } from "@/src/server/englishRepository";

export async function POST(request: Request) {
  try {
    const body = await request.json().catch(() => ({})) as { force?: boolean };
    return Response.json(await syncVoaArticles(Boolean(body.force)));
  } catch (error) {
    return Response.json(
      { error: error instanceof Error ? error.message : "VOA 文章同步失败" },
      { status: 502 },
    );
  }
}
