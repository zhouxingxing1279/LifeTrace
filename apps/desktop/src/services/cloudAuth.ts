import type { SessionBindingResult } from "@/src/services/cloudSync";
import { clientLogger } from "@/src/services/clientObservability";

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

export type CloudAuthCapabilities = {
  registrationMode: string;
  passwordMinLength: number;
  passwordMaxBytes: number;
  accessTokenTtlSeconds: number;
  refreshIdleTtlSeconds: number;
  refreshAbsoluteTtlSeconds: number;
  webSessionEnabled: boolean;
  supportedApps: string[];
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

class CloudHttpError extends Error {
  constructor(message: string, readonly status: number, readonly code?: string) {
    super(message);
    this.name = "CloudHttpError";
  }
}

async function parseResponse<T>(response: Response, action: string): Promise<T> {
  if (response.ok) {
    try {
      return await response.json() as T;
    } catch (cause) {
      const error = new Error("云服务返回了无法解析的数据", { cause });
      clientLogger.error("cloud.response.parse_failed", {
        action,
        status: response.status,
        stage: "response.parse",
      }, error);
      throw error;
    }
  }

  const payload = await response.json().catch((cause) => {
    clientLogger.warn("cloud.response.error_body_parse_failed", {
      action,
      status: response.status,
      stage: "response.parse",
    }, cause);
    return {};
  }) as { message?: string; code?: string };
  const error = new CloudHttpError(
    payload.message || payload.code || `云服务返回 HTTP ${response.status}`,
    response.status,
    payload.code,
  );
  clientLogger.error("cloud.response.http_failed", {
    action,
    status: response.status,
    stage: "response.http",
    requestSent: true,
  }, error);
  throw error;
}

function nativeRequestPayload(email: string, password: string) {
  return {
    email,
    password,
    appId: APP_ID,
    deviceId: deviceId(),
    deviceName: "LifeTrace Windows Desktop",
    platform: "windows",
    clientVersion: "0.2.1",
    requestedScopes: [],
  };
}

export class CloudAuthClient {
  private origin = "";
  private accessToken?: string;
  private refreshFlight?: Promise<CloudTokenResponse>;
  private snapshot: CloudAuthSnapshot = { scopes: [], authenticated: false };

  configure(origin: string) {
    this.origin = normalizeOrigin(origin);
    clientLogger.info("cloud.auth.configured", { origin: this.origin });
  }

  configuredOrigin(): string {
    return this.origin;
  }

  state(): CloudAuthSnapshot {
    return { ...this.snapshot, scopes: [...this.snapshot.scopes] };
  }

  private ensureOrigin() {
    if (!this.origin) throw new Error("云服务尚未配置");
  }

  async hasStoredCredential(): Promise<boolean> {
    try {
      return Boolean(await credentialApi().get());
    } catch (error) {
      clientLogger.warn("cloud.auth.credential_probe_failed", undefined, error);
      return false;
    }
  }

  async capabilities(): Promise<CloudAuthCapabilities> {
    this.ensureOrigin();
    const response = await fetch(`${this.origin}/api/v1/auth/capabilities`);
    return parseResponse<CloudAuthCapabilities>(response, "capabilities");
  }

  async login(email: string, password: string): Promise<CloudAuthSnapshot> {
    this.ensureOrigin();
    clientLogger.info("cloud.auth.login_started", { origin: this.origin });
    const response = await fetch(`${this.origin}/api/v1/auth/login`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        ...nativeRequestPayload(email, password),
        publicDevice: false,
      }),
    });
    const snapshot = await this.acceptTokens(await parseResponse<CloudTokenResponse>(response, "login"));
    clientLogger.info("cloud.auth.login_succeeded", {
      userId: snapshot.user?.id,
      sessionId: snapshot.session?.id,
    });
    return snapshot;
  }

  async register(input: {
    email: string;
    password: string;
    displayName?: string;
    inviteToken?: string;
  }): Promise<CloudAuthSnapshot> {
    this.ensureOrigin();
    clientLogger.info("cloud.auth.register_started", { origin: this.origin });
    const response = await fetch(`${this.origin}/api/v1/auth/register`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        ...nativeRequestPayload(input.email, input.password),
        displayName: input.displayName?.trim() || undefined,
        inviteToken: input.inviteToken?.trim() || undefined,
      }),
    });
    const snapshot = await this.acceptTokens(await parseResponse<CloudTokenResponse>(response, "register"));
    clientLogger.info("cloud.auth.register_succeeded", {
      userId: snapshot.user?.id,
      sessionId: snapshot.session?.id,
    });
    return snapshot;
  }

  async forgotPassword(email: string): Promise<void> {
    this.ensureOrigin();
    const response = await fetch(`${this.origin}/api/v1/auth/password/forgot`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ email }),
    });
    await parseResponse<{ accepted: boolean }>(response, "forgot-password");
  }

  async changePassword(currentPassword: string, newPassword: string): Promise<void> {
    const response = await this.request("/api/v1/auth/password/change", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ currentPassword, newPassword }),
    });
    await parseResponse<{ accepted: boolean }>(response, "change-password");
  }

  private async selectUserProfile(tokens: CloudTokenResponse): Promise<SessionBindingResult | undefined> {
    if (!window.syncApi) return undefined;

    let binding = await window.syncApi.setSession(this.origin, tokens.accessToken, deviceId());
    if (!binding.bindingRequired && binding.cloudUserId === tokens.user.id) return binding;

    const profiles = await window.syncApi.profiles();
    const existing = profiles.find((profile) => profile.cloudUserId === tokens.user.id);
    if (existing) {
      await window.syncApi.setActiveProfile(existing.id);
      clientLogger.info("cloud.auth.profile_selected", { userId: tokens.user.id, profileId: existing.id });
    } else {
      const profileId = await window.syncApi.createCloudProfile(tokens.user.displayName || tokens.user.email || "LifeTrace 用户");
      clientLogger.info("cloud.auth.profile_created", { userId: tokens.user.id, profileId });
    }

    binding = await window.syncApi.setSession(this.origin, tokens.accessToken, deviceId());
    if (binding.cloudUserId !== tokens.user.id || binding.bindingRequired) {
      throw new Error("无法切换到当前账号的数据空间");
    }
    return binding;
  }

  private async acceptTokens(tokens: CloudTokenResponse): Promise<CloudAuthSnapshot> {
    this.accessToken = tokens.accessToken;
    if (tokens.refreshToken) await credentialApi().set(tokens.refreshToken);
    const binding = await this.selectUserProfile(tokens);
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
        const tokens = await parseResponse<CloudTokenResponse>(response, "refresh");
        await this.acceptTokens(tokens);
        clientLogger.info("cloud.auth.refresh_succeeded", {
          userId: tokens.user.id,
          sessionId: tokens.session.id,
        });
        return tokens;
      } catch (error) {
        clientLogger.error("cloud.auth.refresh_failed", {
          status: response.status,
        }, error);
        if (response.status === 401 || response.status === 403) await this.clearLocal();
        throw error;
      }
    })().finally(() => { this.refreshFlight = undefined; });
    return this.refreshFlight;
  }

  async restore(): Promise<CloudAuthSnapshot> {
    try {
      await this.refresh();
      return this.state();
    } catch (error) {
      clientLogger.warn("cloud.auth.restore_failed", undefined, error);
      throw error;
    }
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
      clientLogger.warn("cloud.auth.request_unauthorized", {
        path: input,
        status: response.status,
      });
      await this.refresh();
      response = await execute();
    }
    return response;
  }

  async logout(all = false): Promise<void> {
    try {
      if (this.accessToken && this.origin) {
        const response = await fetch(`${this.origin}/api/v1/auth/${all ? "logout-all" : "logout"}`, {
          method: "POST",
          headers: { authorization: `Bearer ${this.accessToken}` },
        });
        if (!response.ok) {
          const error = new Error(`退出登录请求失败 (${response.status})`);
          clientLogger.warn("cloud.auth.logout_http_failed", {
            all,
            status: response.status,
          }, error);
        }
      }
    } catch (error) {
      clientLogger.error("cloud.auth.logout_failed", { all }, error);
      throw error;
    } finally {
      await this.clearLocal();
    }
  }

  async clearLocal(): Promise<void> {
    this.accessToken = undefined;
    this.snapshot = { scopes: [], authenticated: false };
    await credentialApi().clear();
    if (window.syncApi) {
      await window.syncApi.clearSession().catch((error) => {
        clientLogger.warn("cloud.auth.sync_session_clear_failed", undefined, error);
      });
      try {
        const profiles = await window.syncApi.profiles();
        const localProfile = profiles.find((profile) => !profile.cloudUserId);
        if (localProfile) await window.syncApi.setActiveProfile(localProfile.id);
      } catch (error) {
        clientLogger.warn("cloud.auth.local_profile_restore_failed", undefined, error);
      }
    }
  }
}

export const cloudAuthClient = new CloudAuthClient();
