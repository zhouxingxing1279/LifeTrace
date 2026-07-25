import {
  createFailedWorkoutImport,
  createWorkoutImport,
} from "@/src/server/xunjiImportRepository";
import type { XunjiWorkout } from "@/src/types";

const SERVICE_URL = process.env.XUNJI_SERVICE_URL ?? "http://127.0.0.1:8001";
const SERVICE_TIMEOUT_MS = 90_000;

const validWorkout = (value: unknown): value is XunjiWorkout => {
  if (!value || typeof value !== "object") return false;
  const workout = value as Partial<XunjiWorkout>;
  return workout.source === "xunji"
    && typeof workout.date === "string"
    && typeof workout.title === "string"
    && Array.isArray(workout.exercises)
    && workout.exercises.length > 0;
};

// 网页只访问同源 API；该路由把图片转发给本机 FastAPI，图片不会写入 D1。
export async function POST(request: Request) {
  let servicePayload: Record<string, unknown> = {};
  try {
    const incoming = await request.formData();
    const image = incoming.get("image");
    if (!(image instanceof File)) return Response.json({ error: "请选择训记分享图片" }, { status: 400 });
    if (image.size > 15 * 1024 * 1024) return Response.json({ error: "图片不能超过 15MB" }, { status: 400 });

    const outgoing = new FormData();
    outgoing.set("image", image, image.name);
    const response = await fetch(`${SERVICE_URL}/api/xunji/parse`, {
      method: "POST",
      body: outgoing,
      signal: AbortSignal.timeout(SERVICE_TIMEOUT_MS),
    });
    servicePayload = await response.json() as Record<string, unknown>;
    if (!response.ok) {
      const message = typeof servicePayload.error === "string" ? servicePayload.error : "训记分享解析失败";
      const failed = await createFailedWorkoutImport(message, servicePayload);
      return Response.json({ error: message, importId: failed.id }, { status: response.status });
    }
    const shareUrl = String(servicePayload.shareUrl ?? "");
    const parsedUrl = new URL(shareUrl);
    const validSharePath = parsedUrl.pathname === "/app_share" || parsedUrl.pathname.startsWith("/app_share/");
    if (parsedUrl.protocol !== "https:" || parsedUrl.hostname !== "api.xunjiapp.cn" || !validSharePath) {
      throw new Error("解析服务返回了不允许的分享地址");
    }
    if (!validWorkout(servicePayload.workout)) throw new Error("解析结果缺少标准训练数据");
    const record = await createWorkoutImport({
      shareUrl,
      rawData: servicePayload.rawData,
      workout: servicePayload.workout,
    });
    return Response.json({
      importId: record.id,
      shareUrl,
      parser: servicePayload.parser,
      workout: record.workout,
    });
  } catch (error) {
    const message = error instanceof Error && error.name === "TimeoutError"
      ? "训记分享页面访问超时"
      : error instanceof Error ? error.message : "训记解析服务暂时不可用";
    const failed = await createFailedWorkoutImport(message, servicePayload);
    return Response.json({ error: message, importId: failed.id }, { status: 502 });
  }
}
