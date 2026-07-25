import { deleteUserVocabulary, getUserVocabulary, updateUserVocabulary } from "@/src/server/vocabulary/repository";

export async function GET(_: Request, context: { params: Promise<{ id: string }> }) {
  const { id } = await context.params;
  const item = await getUserVocabulary(id);
  return item ? Response.json(item) : Response.json({ error: "生词不存在" }, { status: 404 });
}

export async function PATCH(request: Request, context: { params: Promise<{ id: string }> }) {
  try {
    const { id } = await context.params;
    return Response.json(await updateUserVocabulary(id, await request.json()));
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "生词更新失败" }, { status: 500 });
  }
}

export async function DELETE(_: Request, context: { params: Promise<{ id: string }> }) {
  const { id } = await context.params;
  await deleteUserVocabulary(id);
  return Response.json({ ok: true });
}
