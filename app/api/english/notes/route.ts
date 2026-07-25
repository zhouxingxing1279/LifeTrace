import { articleAnnotations, saveNote } from "@/src/server/englishRepository";

// 笔记可以关联用户选中的原文，也可以作为文章级独立思考。
export async function GET(request: Request) {
  try {
    const articleId = new URL(request.url).searchParams.get("articleId");
    if (!articleId) return Response.json({ error: "缺少文章编号" }, { status: 400 });
    return Response.json(await articleAnnotations(articleId));
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "笔记读取失败" }, { status: 500 });
  }
}

export async function POST(request: Request) {
  try {
    const body = await request.json() as { articleId?: string; quote?: string; content?: string };
    if (!body.articleId || !body.content?.trim()) return Response.json({ error: "笔记内容不能为空" }, { status: 400 });
    return Response.json(await saveNote({ articleId: body.articleId, quote: body.quote, content: body.content }));
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "笔记保存失败" }, { status: 500 });
  }
}
