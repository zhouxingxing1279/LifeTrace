export interface LifeTraceTheme {
  id: string;
  name: string;
  source: "json" | "css";
  tokens: Record<string, string>;
  background?: { image: string };
  importedAt: string;
}

const ALLOWED_TOKEN_PREFIXES = ["--lt-", "--ui-"];
const MAX_TOKEN_VALUE_LENGTH = 500;
const MAX_BACKGROUND_LENGTH = 1_500_000;

function isSafeTokenValue(value: string): boolean {
  if (/url\s*\(|@import|expression\s*\(|javascript:|<|>|\{|\}/i.test(value)) return false;
  return true;
}

function pickName(name: unknown, fallback: string): string {
  return typeof name === "string" && name.trim() ? name.trim().slice(0, 40) : fallback;
}

function parseJsonTheme(text: string, fallbackName: string): LifeTraceTheme {
  let data: unknown;
  try {
    data = JSON.parse(text);
  } catch {
    throw new Error("主题文件不是有效的 JSON");
  }
  if (!data || typeof data !== "object" || Array.isArray(data)) {
    throw new Error("主题文件格式不正确");
  }
  const obj = data as Record<string, unknown>;
  const name = pickName(obj.name, fallbackName);
  const tokens: Record<string, string> = {};
  if (obj.tokens && typeof obj.tokens === "object" && !Array.isArray(obj.tokens)) {
    for (const [key, value] of Object.entries(obj.tokens as Record<string, unknown>)) {
      if (!ALLOWED_TOKEN_PREFIXES.some(prefix => key.startsWith(prefix))) continue;
      if (typeof value !== "string" || value.length > MAX_TOKEN_VALUE_LENGTH) continue;
      if (!isSafeTokenValue(value)) continue;
      tokens[key] = value;
    }
  }
  let background: LifeTraceTheme["background"];
  if (obj.background && typeof obj.background === "object" && !Array.isArray(obj.background)) {
    const bg = obj.background as Record<string, unknown>;
    if (typeof bg.image === "string" && bg.image.startsWith("data:image/") && bg.image.length < MAX_BACKGROUND_LENGTH) {
      background = { image: bg.image };
    }
  }
  if (Object.keys(tokens).length === 0 && !background) {
    throw new Error("主题文件里没有可用的令牌或背景图");
  }
  return {
    id: crypto.randomUUID(),
    name,
    source: "json",
    tokens,
    background,
    importedAt: new Date().toISOString(),
  };
}

function parseCssTheme(text: string, fallbackName: string): LifeTraceTheme {
  const nameMatch =
    text.match(/\/\*\s*(?:name|名称)\s*[:：]\s*([^*]{1,40}?)\s*\*\//i) ??
    text.match(/\/\*\s*([^*]{1,40}?)\s*\*\//);
  const name = pickName(nameMatch?.[1], fallbackName);
  const tokens: Record<string, string> = {};
  let background: LifeTraceTheme["background"];
  const declaration = /(--[a-zA-Z0-9-]+)\s*:\s*([^;}{]+);/g;
  let match: RegExpExecArray | null;
  while ((match = declaration.exec(text)) !== null) {
    const key = match[1].trim();
    const value = match[2].trim();
    if (!ALLOWED_TOKEN_PREFIXES.some(prefix => key.startsWith(prefix))) continue;
    if (key === "--lt-bg-image") {
      const image = value.match(/url\(\s*["']?(data:image\/[^"')]+)["']?\s*\)/i)?.[1];
      if (image && image.length < MAX_BACKGROUND_LENGTH) {
        background = { image };
        continue;
      }
    }
    if (value.length > MAX_TOKEN_VALUE_LENGTH || !isSafeTokenValue(value)) continue;
    tokens[key] = value;
  }
  if (Object.keys(tokens).length === 0 && !background) {
    throw new Error("主题文件里没有可用的令牌");
  }
  return {
    id: crypto.randomUUID(),
    name,
    source: "css",
    tokens,
    background,
    importedAt: new Date().toISOString(),
  };
}

export function parseThemeFile(text: string, filename: string): LifeTraceTheme {
  const baseName = filename.replace(/\.(json|css)$/i, "").trim() || "自定义主题";
  return /\.css$/i.test(filename) ? parseCssTheme(text, baseName) : parseJsonTheme(text, baseName);
}

export function buildThemeCss(theme: LifeTraceTheme): string {
  const lines: string[] = [':root[data-ui-style="editorial"] {'];
  for (const [key, value] of Object.entries(theme.tokens)) {
    lines.push(`  ${key}: ${value};`);
  }
  if (theme.background?.image) {
    lines.push(`  --lt-bg-image: url("${theme.background.image}");`);
  }
  lines.push("}");
  return lines.join("\n");
}
