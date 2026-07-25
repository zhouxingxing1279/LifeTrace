const TRACKING_PARAMETERS = new Set([
  "fbclid", "gclid", "mc_cid", "mc_eid", "ref", "source",
]);

const INVISIBLE_CHARACTERS = /[\u00ad\u034f\u061c\u115f\u1160\u17b4\u17b5\u180b-\u180f\u200b-\u200f\u202a-\u202e\u2060-\u206f\u3164\ufeff\uffa0]/g;

export function normalizeUrl(value: string): string {
  const url = new URL(value.trim());
  url.hash = "";
  url.protocol = url.protocol.toLowerCase();
  url.hostname = url.hostname.toLowerCase();
  if ((url.protocol === "https:" && url.port === "443") || (url.protocol === "http:" && url.port === "80")) {
    url.port = "";
  }
  for (const key of [...url.searchParams.keys()]) {
    if (key.toLowerCase().startsWith("utm_") || TRACKING_PARAMETERS.has(key.toLowerCase())) {
      url.searchParams.delete(key);
    }
  }
  url.searchParams.sort();
  url.pathname = url.pathname.replace(/\/{2,}/g, "/");
  if (url.pathname !== "/") url.pathname = url.pathname.replace(/\/+$/, "");
  return url.toString();
}

export function normalizeContent(value: string): string {
  return value
    .normalize("NFKC")
    .replace(INVISIBLE_CHARACTERS, "")
    .replace(/\r\n?/g, "\n")
    .split(/\n+/)
    .map((line) => line.replace(/[^\S\n]+/g, " ").trim())
    .filter(Boolean)
    .join("\n")
    .trim();
}

export async function calculateContentHash(value: string): Promise<string> {
  const bytes = new TextEncoder().encode(normalizeContent(value));
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

export const countWords = (content: string) =>
  normalizeContent(content).match(/\b[A-Za-z]+(?:['’-][A-Za-z]+)*\b/g)?.length ?? 0;

export const englishRatio = (content: string) => {
  const letters = content.match(/\p{L}/gu) ?? [];
  if (!letters.length) return 0;
  return letters.filter((letter) => /[A-Za-z]/.test(letter)).length / letters.length;
};
