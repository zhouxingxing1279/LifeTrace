import { getVocabularySettings, listUserVocabulary } from "@/src/server/vocabulary/repository";

export async function GET() {
  const settings = await getVocabularySettings();
  const result = await listUserVocabulary({ sort: "review", due: true, pageSize: settings.dailyReviewLimit });
  return Response.json({ items: result.items });
}
