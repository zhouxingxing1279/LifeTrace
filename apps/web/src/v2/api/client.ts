export interface ApiErrorBody {
  code?: string;
  message?: string;
  retryable?: boolean;
  fieldErrors?: Array<{ field?: string; code?: string; message?: string }>;
}

export class LifeTraceApiError extends Error {
  readonly status: number;
  readonly code?: string;
  readonly retryable: boolean;

  constructor(status: number, body: ApiErrorBody | null, fallback: string) {
    super(body?.message || fallback);
    this.name = "LifeTraceApiError";
    this.status = status;
    this.code = body?.code;
    this.retryable = Boolean(body?.retryable);
  }
}

type ImportMetaWithEnv = ImportMeta & { env?: Record<string, string | undefined> };

function configuredOrigin(): string {
  const raw = ((import.meta as ImportMetaWithEnv).env?.VITE_LIFETRACE_CLOUD_URL || "").trim();
  if (!raw) return "";
  return raw.replace(/\/+$/, "");
}

function requestUrl(path: string): string {
  const origin = configuredOrigin();
  return origin ? `${origin}${path.startsWith("/") ? path : `/${path}`}` : path;
}

async function errorBody(response: Response): Promise<ApiErrorBody | null> {
  try {
    const value = await response.json() as ApiErrorBody;
    return value && typeof value === "object" ? value : null;
  } catch {
    return null;
  }
}

export interface ApiRequestOptions {
  method?: "GET" | "POST" | "PATCH" | "DELETE";
  body?: unknown;
  csrfToken?: string | null;
  signal?: AbortSignal;
}

export async function apiRequest<T>(path: string, options: ApiRequestOptions = {}): Promise<T> {
  const headers = new Headers({ Accept: "application/json" });
  if (options.body !== undefined) headers.set("content-type", "application/json");
  if (options.csrfToken) headers.set("x-csrf-token", options.csrfToken);

  const response = await fetch(requestUrl(path), {
    method: options.method || "GET",
    credentials: "include",
    headers,
    body: options.body === undefined ? undefined : JSON.stringify(options.body),
    signal: options.signal
  });

  if (!response.ok) {
    throw new LifeTraceApiError(response.status, await errorBody(response), `${response.status} ${response.statusText}`);
  }
  if (response.status === 204) return undefined as T;
  return await response.json() as T;
}

export function isAuthenticationError(error: unknown): boolean {
  return error instanceof LifeTraceApiError && (error.status === 401 || error.code === "AUTH_INVALID");
}
