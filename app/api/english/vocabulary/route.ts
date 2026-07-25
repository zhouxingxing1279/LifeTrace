import { addVocabulary, listVocabulary, reviewVocabulary } from "@/src/server/englishRepository";

// 生词本 API：GET 查询、POST 加入、PATCH 执行艾宾浩斯复习。
export async function GET(request: Request) {
  try {
    const dueOnly = new URL(request.url).searchParams.get("due") === "1";
    return Response.json({ items: await listVocabulary(dueOnly) });
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "生词本读取失败" }, { status: 500 });
  }
}

export async function POST(request: Request) {
  try {
    const body = await request.json() as { word?: string; phonetic?: string; meaning?: string; example?: string; sourceArticleId?: string };
    if (!body.word || !body.meaning || !body.sourceArticleId) return Response.json({ error: "生词信息不完整" }, { status: 400 });
    return Response.json(await addVocabulary({
      word: body.word.trim(),
      phonetic: body.phonetic?.trim() ?? "",
      meaning: body.meaning.trim(),
      example: body.example?.trim() ?? "",
      sourceArticleId: body.sourceArticleId,
    }));
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "加入生词本失败" }, { status: 500 });
  }
}

export async function PATCH(request: Request) {
  try {
    const body = await request.json() as { id?: string; mastered?: boolean };
    if (!body.id || typeof body.mastered !== "boolean") return Response.json({ error: "复习参数不完整" }, { status: 400 });
    return Response.json(await reviewVocabulary(body.id, body.mastered));
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "生词复习保存失败" }, { status: 500 });
  }
}
