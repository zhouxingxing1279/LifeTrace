import {
  cancelWorkoutImport,
  confirmWorkoutImport,
  listWorkoutImports,
} from "@/src/server/xunjiImportRepository";
import type { XunjiWorkout } from "@/src/types";

export async function GET() {
  try {
    return Response.json({ items: await listWorkoutImports() });
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "导入记录读取失败" }, { status: 500 });
  }
}

// 解析与入库分离：只有用户明确确认时才会创建训练记录和联动数据。
export async function POST(request: Request) {
  try {
    const body = await request.json() as { importId?: string; action?: "confirm" | "cancel"; workout?: XunjiWorkout };
    if (!body.importId) return Response.json({ error: "缺少导入记录" }, { status: 400 });
    if (body.action === "cancel") return Response.json(await cancelWorkoutImport(body.importId));
    if (body.action !== "confirm") return Response.json({ error: "不支持的操作" }, { status: 400 });
    if (body.workout && (!Array.isArray(body.workout.exercises) || !body.workout.exercises.length)) {
      return Response.json({ error: "训练至少需要一个动作" }, { status: 400 });
    }
    return Response.json(await confirmWorkoutImport(body.importId, body.workout));
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "训练导入失败" }, { status: 500 });
  }
}

