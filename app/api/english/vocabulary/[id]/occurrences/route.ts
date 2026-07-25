import { addOccurrence } from "@/src/server/vocabulary/repository";

export async function POST(request: Request, context: { params: Promise<{ id: string }> }) {
  const { id } = await context.params;
  const body = await request.json() as { articleId?: string; articleTitle?: string; sourceSentence?: string };
  if (!body.sourceSentence?.trim()) return Response.json({ error: "来源句子不能为空" }, { status: 400 });
  return Response.json(await addOccurrence(id, { ...body, sourceSentence: body.sourceSentence }));
}
