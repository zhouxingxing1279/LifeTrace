// LifeTrace Cloud API base URL.
//
// Production defaults to same-origin requests (for example `/api/v1/web/session`).
// Caddy owns the public listener and proxies `/api/*` and `/health/*` to the
// internal `lifetrace-cloud:8787` service, so the browser must not bypass the
// reverse proxy by connecting to port 8787 directly.
//
// Local development keeps the same relative URLs; vite.browser.config.ts
// proxies them to http://127.0.0.1:8787. Set VITE_LIFETRACE_CLOUD_URL only
// when an explicit cross-origin Cloud endpoint is required.
const env = (import.meta as unknown as { env?: Record<string, string | undefined> }).env;
const configuredBase = env?.VITE_LIFETRACE_CLOUD_URL;

export function normalizeApiBase(value?: string): string {
  return (value?.trim() ?? "").replace(/\/+$/, "");
}

export const API_BASE = normalizeApiBase(configuredBase);
