import { addUserVocabulary, listUserVocabulary } from "@/src/server/vocabulary/repository";

export async function GET(request: Request) {
  try {
    const url = new URL(request.url);
    return Response.json(await listUserVocabulary({
      query: url.searchParams.get("query") ?? undefined, status: url.searchParams.get("status") ?? undefined,
      sort: url.searchParams.get("sort") ?? undefined, articleId: url.searchParams.get("articleId") ?? undefined,
      pos: url.searchParams.get("pos") ?? undefined, tag: url.searchParams.get("tag") ?? undefined,
      page: Number(url.searchParams.get("page") || 1), pageSize: Number(url.searchParams.get("pageSize") || 50),
    }));
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "生词本读取失败" }, { status: 500 });
  }
}

export async function POST(request: Request) {
  try {
    const body = await request.json() as Parameters<typeof addUserVocabulary>[0];
    if (!body.word || !body.normalizedWord || !body.lemma || !body.selectedMeanings?.length) {
      return Response.json({ error: "请选择至少一条需要记忆的释义" }, { status: 400 });
    }
    return Response.json(await addUserVocabulary(body));
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "加入生词本失败" }, { status: 500 });
  }
}
