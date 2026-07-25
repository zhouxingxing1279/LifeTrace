import { articleAnnotations, saveHighlight } from "@/src/server/englishRepository";

// 高亮独立持久化，阅读器重新打开文章时可以恢复。
export async function GET(request: Request) {
  try {
    const articleId = new URL(request.url).searchParams.get("articleId");
    if (!articleId) return Response.json({ error: "缺少文章编号" }, { status: 400 });
    return Response.json(await articleAnnotations(articleId));
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "高亮读取失败" }, { status: 500 });
  }
}

export async function POST(request: Request) {
  try {
    const body = await request.json() as { articleId?: string; text?: string; color?: "yellow" | "green" | "blue" };
    if (!body.articleId || !body.text?.trim()) return Response.json({ error: "请先选择需要高亮的句子" }, { status: 400 });
    return Response.json(await saveHighlight({ articleId: body.articleId, text: body.text, color: body.color }));
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "高亮保存失败" }, { status: 500 });
  }
}
