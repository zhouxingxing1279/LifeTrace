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

type ManagementRecord = {
  current: boolean;
  lastSeenAt: string;
  revokedAt?: string | null;
};

function timestamp(value: string): number {
  const parsed = Date.parse(value);
  return Number.isNaN(parsed) ? 0 : parsed;
}

function preferredRecord<T extends ManagementRecord>(existing: T, candidate: T): T {
  if (existing.current !== candidate.current) return candidate.current ? candidate : existing;

  const existingActive = !existing.revokedAt;
  const candidateActive = !candidate.revokedAt;
  if (existingActive !== candidateActive) return candidateActive ? candidate : existing;

  return timestamp(candidate.lastSeenAt) > timestamp(existing.lastSeenAt) ? candidate : existing;
}

function normalized(value: string): string {
  return value.trim().toLocaleLowerCase();
}

/**
 * The management backend can return multiple installation rows for the same
 * visible device after repeated logins/registrations. The Settings page should
 * show one logical device instead of one row per timestamped installation.
 */
export function dedupeDeviceInstallations(devices: DeviceInstallation[]): DeviceInstallation[] {
  const unique = new Map<string, DeviceInstallation>();

  for (const device of devices) {
    const key = JSON.stringify([
      normalized(device.appId),
      normalized(device.deviceName),
      normalized(device.platform),
    ]);
    const existing = unique.get(key);
    unique.set(key, existing ? preferredRecord(existing, device) : device);
  }

  return [...unique.values()].sort((left, right) => timestamp(right.lastSeenAt) - timestamp(left.lastSeenAt));
}

/**
 * Sessions are deduplicated by the content visible in Settings/Security. A new
 * session id or a different lastSeenAt must not create another visually
 * identical row for the same app/device/security classification.
 */
export function dedupeManagedSessions(sessions: ManagedSession[]): ManagedSession[] {
  const unique = new Map<string, ManagedSession>();

  for (const session of sessions) {
    const key = JSON.stringify([
      normalized(session.appId),
      normalized(session.deviceId),
      session.publicDevice,
    ]);
    const existing = unique.get(key);
    unique.set(key, existing ? preferredRecord(existing, session) : session);
  }

  return [...unique.values()].sort((left, right) => timestamp(right.lastSeenAt) - timestamp(left.lastSeenAt));
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
    const devices = (await this.request<{ devices: DeviceInstallation[] }>("/api/v1/web/devices")).devices;
    return dedupeDeviceInstallations(devices);
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
    const sessions = (await this.request<{ sessions: ManagedSession[] }>("/api/v1/web/sessions")).sessions;
    return dedupeManagedSessions(sessions);
  }

  async revokeSession(sessionId: string, csrfToken: string): Promise<void> {
    await this.request(`/api/v1/web/sessions/${encodeURIComponent(sessionId)}`, {
      method: "DELETE",
      body: "{}",
    }, csrfToken);
  }
}
