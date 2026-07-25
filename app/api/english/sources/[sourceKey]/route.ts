import { setSourceEnabled } from "@/src/server/englishSync/storage";

export async function PATCH(request: Request, context: { params: Promise<{ sourceKey: string }> }) {
  try {
    const { sourceKey } = await context.params;
    const body = await request.json() as { enabled?: boolean };
    if (typeof body.enabled !== "boolean") return Response.json({ error: "enabled 必须是布尔值" }, { status: 400 });
    const source = await setSourceEnabled(sourceKey, body.enabled);
    return source ? Response.json({ source }) : Response.json({ error: "数据源不存在" }, { status: 404 });
  } catch (error) {
    return Response.json({ error: error instanceof Error ? error.message : "数据源更新失败" }, { status: 500 });
  }
}
