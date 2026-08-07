import type { DeviceInstallation, FetchLike, ManagedSession } from "./types";
import { API_BASE } from "./base";
import { browserFetch } from "./http";

async function parseResponse<T>(response: Response): Promise<T> {
  const raw = await response.text();
  const payload = raw ? JSON.parse(raw) as unknown : null;
  if (!response.ok) {
    const record = payload && typeof payload === "object" ? payload as Record<string, unknown> : {};
    const nested = record.error && typeof record.error === "object" ? record.error as Record<string, unknown> : {};
    const message = typeof record.message === "string"
      ? record.message
      : typeof nested.message === "string"
        ? nested.message
        : `请求失败 (${response.status})`;
    throw new Error(message);
  }
  return payload as T;
}

export class WebManagementApi {
  constructor(private readonly fetcher: FetchLike = browserFetch) {}

  private async request<T>(url: string, init: RequestInit = {}, csrfToken?: string): Promise<T> {
    const headers = new Headers(init.headers);
    if (init.body !== undefined && !headers.has("content-type")) headers.set("content-type", "application/json");
    if (csrfToken) headers.set("x-csrf-token", csrfToken);
    let response: Response;
    try {
      response = await this.fetcher(`${API_BASE}${url}`, { ...init, credentials: "include", headers });
    } catch {
      throw new Error("无法连接 LifeTrace 云端，请检查网络后重试");
    }
    return parseResponse<T>(response);
  }

  async devices(): Promise<DeviceInstallation[]> {
    return (await this.request<{ devices: DeviceInstallation[] }>("/api/v1/web/devices")).devices;
  }

  renameDevice(deviceId: string, deviceName: string, csrfToken: string): Promise<DeviceInstallation> {
    return this.request(`/api/v1/web/devices/${encodeURIComponent(deviceId)}`, {
      method: "PATCH",
      body: JSON.stringify({ deviceName: deviceName.trim() }),
    }, csrfToken);
  }

  async revokeDevice(deviceId: string, csrfToken: string): Promise<void> {
    await this.request(`/api/v1/web/devices/${encodeURIComponent(deviceId)}/revoke`, {
      method: "POST",
      body: "{}",
    }, csrfToken);
  }

  async sessions(): Promise<ManagedSession[]> {
    return (await this.request<{ sessions: ManagedSession[] }>("/api/v1/web/sessions")).sessions;
  }

  async revokeSession(sessionId: string, csrfToken: string): Promise<void> {
    await this.request(`/api/v1/web/sessions/${encodeURIComponent(sessionId)}`, {
      method: "DELETE",
      body: "{}",
    }, csrfToken);
  }
}
