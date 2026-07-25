import { getVocabularySettings, saveVocabularySettings } from "@/src/server/vocabulary/repository";
export async function GET() { return Response.json(await getVocabularySettings()); }
export async function PATCH(request: Request) {
  try { return Response.json(await saveVocabularySettings(await request.json())); }
  catch (error) { return Response.json({ error: error instanceof Error ? error.message : "设置保存失败" }, { status: 500 }); }
}
