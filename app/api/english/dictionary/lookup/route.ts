const SERVICE_URL = process.env.XUNJI_SERVICE_URL ?? "http://127.0.0.1:8001";

export async function GET(request: Request) {
  const source = new URL(request.url);
  const word = source.searchParams.get("word")?.trim();
  if (!word) return Response.json({ error: "请选择一个英文单词" }, { status: 400 });
  const target = new URL("/api/dictionary/lookup", SERVICE_URL);
  target.searchParams.set("word", word);
  for (const key of ["articleId", "sentence"]) {
    const value = source.searchParams.get(key);
    if (value) target.searchParams.set(key, value);
  }
  try {
    const response = await fetch(target, { signal: AbortSignal.timeout(5000) });
    const payload = await response.json();
    return Response.json(payload, { status: response.status });
  } catch {
    return Response.json({ error: "本地离线词典服务未启动，请重启 Life trace" }, { status: 503 });
  }
}
