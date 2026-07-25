import { env } from "cloudflare:workers";

export interface ImportUpload {
  id: string;
  kind: "fitness" | "bill";
  filename: string;
  contentType: string;
  size: number;
  status: "pending" | "parsed";
  objectKey: string;
  createdAt: string;
  updatedAt: string;
}

const table = "import_uploads";

const ensureSchema = async () => {
  await env.DB.prepare(
    `CREATE TABLE IF NOT EXISTS ${table} (
      id TEXT PRIMARY KEY,
      data_json TEXT NOT NULL,
      updated_at TEXT NOT NULL
    )`,
  ).run();
};

const readItem = async (id: string) => {
  const row = await env.DB.prepare(`SELECT data_json FROM ${table} WHERE id = ?`)
    .bind(id).first<{ data_json: string }>();
  return row ? JSON.parse(row.data_json) as ImportUpload : null;
};

export async function GET(request: Request) {
  await ensureSchema();
  const id = new URL(request.url).searchParams.get("id");
  if (id) {
    const item = await readItem(id);
    if (!item) return Response.json({ error: "文件不存在" }, { status: 404 });
    const object = await env.UPLOADS.get(item.objectKey);
    if (!object) return Response.json({ error: "文件内容不存在" }, { status: 404 });
    return new Response(object.body, {
      headers: {
        "content-type": item.contentType || object.httpMetadata?.contentType || "application/octet-stream",
        "content-disposition": `inline; filename*=UTF-8''${encodeURIComponent(item.filename)}`,
        "cache-control": "no-store",
      },
    });
  }
  const rows = await env.DB.prepare(`SELECT data_json FROM ${table} ORDER BY updated_at DESC`)
    .all<{ data_json: string }>();
  return Response.json({ items: rows.results.map((row) => JSON.parse(row.data_json) as ImportUpload) });
}

export async function POST(request: Request) {
  await ensureSchema();
  const form = await request.formData();
  const file = form.get("file");
  const kind = form.get("kind");
  if (!(file instanceof File) || (kind !== "fitness" && kind !== "bill")) {
    return Response.json({ error: "请选择正确的导入文件" }, { status: 400 });
  }
  if (file.size <= 0 || file.size > 25 * 1024 * 1024) {
    return Response.json({ error: "单个文件必须小于 25MB" }, { status: 413 });
  }
  const id = crypto.randomUUID();
  const stamp = new Date().toISOString();
  const objectKey = `${kind}/${id}/${file.name.replace(/[^\p{L}\p{N}._-]+/gu, "_")}`;
  const item: ImportUpload = {
    id,
    kind,
    filename: file.name,
    contentType: file.type || "application/octet-stream",
    size: file.size,
    status: "pending",
    objectKey,
    createdAt: stamp,
    updatedAt: stamp,
  };
  await env.UPLOADS.put(objectKey, file.stream(), { httpMetadata: { contentType: item.contentType } });
  await env.DB.prepare(
    `INSERT INTO ${table} (id, data_json, updated_at) VALUES (?, ?, ?)`,
  ).bind(id, JSON.stringify(item), stamp).run();
  return Response.json({ item });
}

export async function PATCH(request: Request) {
  await ensureSchema();
  const body = await request.json() as { id?: string; status?: ImportUpload["status"] };
  if (!body.id || (body.status !== "pending" && body.status !== "parsed")) {
    return Response.json({ error: "状态更新格式无效" }, { status: 400 });
  }
  const item = await readItem(body.id);
  if (!item) return Response.json({ error: "文件不存在" }, { status: 404 });
  const updated = { ...item, status: body.status, updatedAt: new Date().toISOString() };
  await env.DB.prepare(`UPDATE ${table} SET data_json = ?, updated_at = ? WHERE id = ?`)
    .bind(JSON.stringify(updated), updated.updatedAt, updated.id).run();
  return Response.json({ item: updated });
}

export async function DELETE(request: Request) {
  await ensureSchema();
  const id = new URL(request.url).searchParams.get("id");
  if (!id) return Response.json({ error: "缺少文件 ID" }, { status: 400 });
  const item = await readItem(id);
  if (!item) return Response.json({ ok: true });
  await env.UPLOADS.delete(item.objectKey);
  await env.DB.prepare(`DELETE FROM ${table} WHERE id = ?`).bind(id).run();
  return Response.json({ ok: true });
}
