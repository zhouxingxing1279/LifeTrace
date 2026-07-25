import { getArticle, listArticles, LEVELS } from "@/src/server/englishRepository";
import type { CEFRLevel } from "@/src/types/english";

// GET /api/english/articles：支持按等级、分类筛选，也可通过 id 读取单篇文章。
export async function GET(request: Request) {
  try {
    const url = new URL(request.url);
    const id = url.searchParams.get("id");
    if (id) {
      const article = await getArticle(id);
      return article ? Response.json(article) : Response.json({ error: "文章不存在" }, { status: 404 });
    }
    const levelValue = url.searchParams.get("level");
    const level = LEVELS.includes(levelValue as CEFRLevel) ? levelValue as CEFRLevel : undefined;
    return Response.json({ articles: await listArticles(level, url.searchParams.get("category") ?? undefined) });
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "文章列表读取失败" }, { status: 500 });
  }
}
