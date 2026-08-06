import { REQUESTED_SCOPES, type FetchLike, type WebSession } from "./cloud/types";

export interface AuthCapabilities {
  registrationMode: "open" | "invite" | "disabled" | string;
  passwordMinLength: number;
  passwordMaxBytes: number;
  webSessionEnabled: boolean;
}

export interface WebRegistrationInput {
  email: string;
  password: string;
  displayName: string;
  inviteToken: string;
  publicDevice: boolean;
}

async function readJson(response: Response): Promise<unknown> {
  const raw = await response.text();
  if (!raw) return null;
  try { return JSON.parse(raw) as unknown; }
  catch { return { message: raw }; }
}

function errorMessage(payload: unknown, fallback: string): string {
  if (!payload || typeof payload !== "object") return fallback;
  const value = payload as Record<string, unknown>;
  if (typeof value.message === "string" && value.message.trim()) return value.message;
  if (value.error && typeof value.error === "object") {
    const message = (value.error as Record<string, unknown>).message;
    if (typeof message === "string" && message.trim()) return message;
  }
  return fallback;
}

export class RegistrationApi {
  constructor(private readonly fetcher: FetchLike = fetch) {}

  private async request<T>(url: string, init: RequestInit = {}): Promise<T> {
    const headers = new Headers(init.headers);
    if (init.body !== undefined && !headers.has("content-type")) headers.set("content-type", "application/json");
    let response: Response;
    try { response = await this.fetcher(url, { ...init, credentials: "include", headers }); }
    catch { throw new Error("无法连接 LifeTrace 云端，请检查网络后重试"); }
    const payload = await readJson(response);
    if (!response.ok) throw new Error(errorMessage(payload, `请求失败 (${response.status})`));
    return payload as T;
  }

  capabilities(): Promise<AuthCapabilities> {
    return this.request("/api/v1/auth/capabilities");
  }

  register(input: WebRegistrationInput): Promise<WebSession> {
    return this.request("/api/v1/web/session/register", {
      method: "POST",
      body: JSON.stringify({
        email: input.email.trim(),
        password: input.password,
        displayName: input.displayName.trim() || null,
        inviteToken: input.inviteToken.trim() || null,
        requestedScopes: REQUESTED_SCOPES,
        publicDevice: input.publicDevice,
      }),
    });
  }
}
