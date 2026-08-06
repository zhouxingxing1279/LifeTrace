import type { SessionBindingResult } from "@/src/services/cloudSync";

export type CloudAuthUser = {
  id: string;
  email: string;
  displayName?: string | null;
  state: string;
};

export type CloudAuthSession = {
  id: string;
  appId: string;
  deviceId: string;
  status: string;
  createdAt: string;
  lastSeenAt: string;
  absoluteExpiresAt: string;
};

export type CloudTokenResponse = {
  accessToken: string;
  refreshToken?: string | null;
  tokenType: string;
  expiresIn: number;
  refreshExpiresIn?: number | null;
  user: CloudAuthUser;
  session: CloudAuthSession;
  scopes: string[];
};

export type CloudAuthSnapshot = {
  user?: CloudAuthUser;
  session?: CloudAuthSession;
  scopes: string[];
  authenticated: boolean;
  binding?: SessionBindingResult;
};

type CredentialApi = {
  set(refreshToken: string): Promise<void>;
  get(): Promise<string | null>;
  clear(): Promise<void>;
};

const APP_ID = "lifetrace-desktop";
const DEVICE_KEY = "lifetrace-cloud-device-id";

function credentialApi(): CredentialApi {
  const api = window.cloudCredentialApi;
  if (!api) {
    return {
      set: async () => { throw new Error("Windows 安全凭据存储不可用"); },
      get: async () => null,
      clear: async () => undefined,
    };
  }
  return api;
}

function deviceId(): string {
  let value = localStorage.getItem(DEVICE_KEY);
  if (!value) {
    value = crypto.randomUUID();
    localStorage.setItem(DEVICE_KEY, value);
  }
  return value;
}

function normalizeOrigin(value: string): string {
  const parsed = new URL(value.trim());
  if (!/^https?:$/.test(parsed.protocol)) throw new Error("云服务地址必须使用 HTTP 或 HTTPS");
  return parsed.origin;
}

async function parseResponse<T>(response: Response): Promise<T> {
  if (response.ok) return response.json() as Promise<T>;
  const payload = await response.json().catch(() => ({})) as { message?: string; code?: string };
  throw new Error(payload.message || payload.code || `云服务返回 HTTP ${response.status}`);
}

export class CloudAuthClient {
  private origin = "";
  private accessToken?: string;
  private refreshFlight?: Promise<CloudTokenResponse>;
  private snapshot: CloudAuthSnapshot = { scopes: [], authenticated: false };

  configure(origin: string) {
    this.origin = normalizeOrigin(origin);
  }

  state(): CloudAuthSnapshot {
    return { ...this.snapshot, scopes: [...this.snapshot.scopes] };
  }

  private ensureOrigin() {
    if (!this.origin) throw new Error("请先填写云服务地址");
  }

  async login(email: string, password: string): Promise<CloudAuthSnapshot> {
    this.ensureOrigin();
    const response = await fetch(`${this.origin}/api/v1/auth/login`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        email,
        password,
        appId: APP_ID,
        deviceId: deviceId(),
        deviceName: "LifeTrace Windows Desktop",
        platform: "windows",
        clientVersion: "0.2.1",
        requestedScopes: [],
        publicDevice: false,
      }),
    });
    return this.acceptTokens(await parseResponse<CloudTokenResponse>(response));
  }

  private async acceptTokens(tokens: CloudTokenResponse): Promise<CloudAuthSnapshot> {
    this.accessToken = tokens.accessToken;
    if (tokens.refreshToken) await credentialApi().set(tokens.refreshToken);
    const binding = window.syncApi
      ? await window.syncApi.setSession(this.origin, tokens.accessToken, deviceId())
      : undefined;
    this.snapshot = {
      authenticated: true,
      binding,
      user: tokens.user,
      session: tokens.session,
      scopes: tokens.scopes,
    };
    return this.state();
  }

  async refresh(): Promise<CloudTokenResponse> {
    if (this.refreshFlight) return this.refreshFlight;
    this.ensureOrigin();
    this.refreshFlight = (async () => {
      const refreshToken = await credentialApi().get();
      if (!refreshToken) throw new Error("没有可用的安全 Refresh Token");
      const response = await fetch(`${this.origin}/api/v1/auth/refresh`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ refreshToken, appId: APP_ID, deviceId: deviceId() }),
      });
      try {
        const tokens = await parseResponse<CloudTokenResponse>(response);
        await this.acceptTokens(tokens);
        return tokens;
      } catch (error) {
        if (response.status === 401 || response.status === 403) await this.clearLocal();
        throw error;
      }
    })().finally(() => { this.refreshFlight = undefined; });
    return this.refreshFlight;
  }

  async restore(): Promise<CloudAuthSnapshot> {
    try {
      await this.refresh();
    } catch {
      await this.clearLocal();
    }
    return this.state();
  }

  async request(input: string, init: RequestInit = {}): Promise<Response> {
    this.ensureOrigin();
    if (!this.accessToken) await this.refresh();
    const execute = () => fetch(`${this.origin}${input}`, {
      ...init,
      headers: { ...init.headers, authorization: `Bearer ${this.accessToken}` },
    });
    let response = await execute();
    if (response.status === 401) {
      await this.refresh();
      response = await execute();
    }
    return response;
  }

  async logout(all = false): Promise<void> {
    try {
      if (this.accessToken && this.origin) {
        await fetch(`${this.origin}/api/v1/auth/${all ? "logout-all" : "logout"}`, {
          method: "POST",
          headers: { authorization: `Bearer ${this.accessToken}` },
        });
      }
    } finally {
      await this.clearLocal();
    }
  }

  async clearLocal(): Promise<void> {
    this.accessToken = undefined;
    this.snapshot = { scopes: [], authenticated: false };
    await credentialApi().clear();
    if (window.syncApi) await window.syncApi.clearSession().catch(() => undefined);
  }
}

export const cloudAuthClient = new CloudAuthClient();
