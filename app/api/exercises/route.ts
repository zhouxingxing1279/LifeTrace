import { env } from "cloudflare:workers";
import type { ExerciseDefinition } from "@/src/types";
import { localizeExercise } from "@/src/data/exerciseChinese";

const SOURCE_URL = "https://raw.githubusercontent.com/yuhonas/free-exercise-db/main/dist/exercises.json";
const IMAGE_BASE_URL = "https://raw.githubusercontent.com/yuhonas/free-exercise-db/main/exercises/";

const normalize = (value: string) => value.trim().toLowerCase();

async function ensureLibrary() {
  await env.DB.prepare("CREATE TABLE IF NOT EXISTS exercise_library (id TEXT PRIMARY KEY, data_json TEXT NOT NULL, updated_at TEXT NOT NULL)").run();
  const count = await env.DB.prepare("SELECT COUNT(*) AS count FROM exercise_library").first<{ count: number }>();
  if (count?.count) return;

  const response = await fetch(SOURCE_URL, { headers: { "user-agent": "HengXu-Personal-OS" } });
  if (!response.ok) throw new Error(`动作库下载失败（${response.status}）`);
  const exercises = await response.json() as ExerciseDefinition[];
  const importedAt = new Date().toISOString();

  for (let index = 0; index < exercises.length; index += 75) {
    const statements = exercises.slice(index, index + 75).map((exercise) =>
      env.DB.prepare("INSERT OR IGNORE INTO exercise_library (id, data_json, updated_at) VALUES (?, ?, ?)")
        .bind(exercise.id, JSON.stringify(exercise), importedAt),
    );
    await env.DB.batch(statements);
  }
}

export async function GET(request: Request) {
  try {
    await ensureLibrary();
    const url = new URL(request.url);
    const query = normalize(url.searchParams.get("q") ?? "");
    const muscle = normalize(url.searchParams.get("muscle") ?? "");
    const equipment = normalize(url.searchParams.get("equipment") ?? "");
    const category = normalize(url.searchParams.get("category") ?? "");
    const limit = Math.min(Math.max(Number(url.searchParams.get("limit") ?? 48), 1), 1000);
    const rows = await env.DB.prepare("SELECT data_json FROM exercise_library ORDER BY id").all<{ data_json: string }>();
    const all = rows.results.map((row, index) => {
      const exercise = JSON.parse(row.data_json) as ExerciseDefinition;
      return { ...exercise, ...localizeExercise(exercise, index) };
    });
    const filtered = all.filter((exercise) => {
      const searchable = [exercise.name, exercise.nameZh ?? "", exercise.equipment ?? "", exercise.category, ...exercise.primaryMuscles, ...exercise.secondaryMuscles].join(" ").toLowerCase();
      return (!query || searchable.includes(query))
        && (!muscle || exercise.primaryMuscles.some((item) => normalize(item) === muscle))
        && (!equipment || normalize(exercise.equipment ?? "") === equipment)
        && (!category || normalize(exercise.category) === category);
    });
    const unique = (items: string[]) => [...new Set(items.filter(Boolean))].sort();
    return Response.json({
      items: filtered.slice(0, limit).map((exercise) => ({
        ...exercise,
        imageUrls: exercise.images.map((image) => `${IMAGE_BASE_URL}${image}`),
      })),
      total: all.length,
      filteredTotal: filtered.length,
      facets: {
        muscles: unique(all.flatMap((exercise) => exercise.primaryMuscles)),
        equipment: unique(all.map((exercise) => exercise.equipment ?? "")),
        categories: unique(all.map((exercise) => exercise.category)),
      },
      source: { repository: "yuhonas/free-exercise-db", license: "Unlicense", url: SOURCE_URL },
    });
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "动作库读取失败" }, { status: 500 });
  }
}
