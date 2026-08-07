import { create } from "zustand";
import {
  cloudAuthClient,
  savedCloudOrigin,
  type CloudAuthCapabilities,
  type CloudAuthSnapshot,
  type CloudAuthUser,
} from "@/src/services/cloudAuth";

export type AuthPhase = "bootstrapping" | "refreshing" | "authenticated" | "anonymous" | "offline" | "error";

const USER_KEY = "lifetrace-cloud-user";
const LOCAL_ORIGIN = "http://127.0.0.1:8787";
type ImportMetaWithEnv = ImportMeta & { env?: Record<string, string | undefined> };

function storage(): Storage | undefined { return typeof window !== "undefined" ? window.localStorage : undefined; }
function buildOrigin(): string { return ((import.meta as ImportMetaWithEnv).env?.VITE_LIFETRACE_CLOUD_URL || "").trim(); }
function readOrigin(): string { return savedCloudOrigin() || buildOrigin() || LOCAL_ORIGIN; }

function readCachedUser(): CloudAuthUser | undefined {
  const value = storage()?.getItem(USER_KEY);
  if (!value) return undefined;
  try {
    const parsed = JSON.parse(value) as CloudAuthUser;
    return parsed?.id && parsed?.email ? parsed : undefined;
  } catch { return undefined; }
}

function writeCachedUser(user?: CloudAuthUser) {
  if (!user) storage()?.removeItem(USER_KEY);
  else storage()?.setItem(USER_KEY, JSON.stringify(user));
}

type CloudAuthState = CloudAuthSnapshot & {
  origin: string;
  phase: AuthPhase;
  loading: boolean;
  initialized: boolean;
  capabilities?: CloudAuthCapabilities;
  error?: string;
  setOrigin(origin: string): void;
  initialize(): Promise<void>;
  loadCapabilities(): Promise<CloudAuthCapabilities | undefined>;
  login(email: string, password: string): Promise<boolean>;
  register(input: { email: string; password: string; displayName?: string; inviteToken?: string }): Promise<boolean>;
  forgotPassword(email: string): Promise<boolean>;
  changePassword(currentPassword: string, newPassword: string): Promise<boolean>;
  restore(): Promise<void>;
  logout(all?: boolean): Promise<void>;
  clearError(): void;
};

function authenticatedPatch(snapshot: CloudAuthSnapshot) {
  writeCachedUser(snapshot.user);
  return { ...snapshot, phase: "authenticated" as const, loading: false, initialized: true, error: undefined };
}

export const useCloudAuthStore = create<CloudAuthState>((set, get) => ({
  origin: savedCloudOrigin() || buildOrigin() || LOCAL_ORIGIN,
  user: readCachedUser(),
  scopes: [],
  authenticated: false,
  phase: "bootstrapping",
  loading: false,
  initialized: false,

  clearError() { set({ error: undefined }); },
  setOrigin(origin) {
    set({ origin, error: undefined, capabilities: undefined });
    try {
      cloudAuthClient.configure(origin);
      set({ origin: cloudAuthClient.configuredOrigin() });
    } catch (error) { set({ error: error instanceof Error ? error.message : "云服务地址无效" }); }
  },

  async initialize() {
    if (get().initialized || get().loading) return;
    const origin = get().origin || readOrigin();
    set({ origin, phase: "refreshing", loading: true, error: undefined });
    try {
      cloudAuthClient.configure(origin);
      set({ origin: cloudAuthClient.configuredOrigin() });
      const hasCredential = await cloudAuthClient.hasStoredCredential();
      if (!hasCredential) {
        writeCachedUser(undefined);
        set({ user: undefined, session: undefined, binding: undefined, scopes: [], authenticated: false, phase: "anonymous", loading: false, initialized: true });
        return;
      }
      const snapshot = await cloudAuthClient.restore();
      set(authenticatedPatch(snapshot));
    } catch (error) {
      const hasCredential = await cloudAuthClient.hasStoredCredential();
      if (hasCredential) {
        set({ authenticated: false, phase: "offline", loading: false, initialized: true, error: "云端暂时不可用，将在网络恢复后重新连接" });
      } else {
        writeCachedUser(undefined);
        set({ user: undefined, session: undefined, binding: undefined, scopes: [], authenticated: false, phase: "anonymous", loading: false, initialized: true, error: undefined });
      }
      if (!hasCredential && error instanceof Error && !/Refresh Token/.test(error.message)) set({ error: error.message });
    }
  },

  async loadCapabilities() {
    try {
      cloudAuthClient.configure(get().origin);
      const capabilities = await cloudAuthClient.capabilities();
      set({ capabilities, error: undefined });
      return capabilities;
    } catch (error) { set({ error: error instanceof Error ? error.message : "无法读取注册能力" }); return undefined; }
  },

  async login(email, password) {
    set({ loading: true, phase: "refreshing", error: undefined });
    try {
      cloudAuthClient.configure(get().origin);
      set(authenticatedPatch(await cloudAuthClient.login(email, password)));
      return true;
    } catch (error) { set({ loading: false, phase: "anonymous", error: error instanceof Error ? error.message : "登录失败" }); return false; }
  },

  async register(input) {
    set({ loading: true, phase: "refreshing", error: undefined });
    try {
      cloudAuthClient.configure(get().origin);
      set(authenticatedPatch(await cloudAuthClient.register(input)));
      return true;
    } catch (error) { set({ loading: false, phase: "anonymous", error: error instanceof Error ? error.message : "注册失败" }); return false; }
  },

  async forgotPassword(email) {
    set({ loading: true, error: undefined });
    try { cloudAuthClient.configure(get().origin); await cloudAuthClient.forgotPassword(email); set({ loading: false }); return true; }
    catch (error) { set({ loading: false, error: error instanceof Error ? error.message : "无法提交密码重置请求" }); return false; }
  },

  async changePassword(currentPassword, newPassword) {
    set({ loading: true, error: undefined });
    try { await cloudAuthClient.changePassword(currentPassword, newPassword); set({ loading: false }); return true; }
    catch (error) { set({ loading: false, error: error instanceof Error ? error.message : "修改密码失败" }); return false; }
  },

  async restore() { await get().initialize(); },

  async logout(all = false) {
    set({ loading: true, error: undefined });
    try {
      await cloudAuthClient.logout(all);
      writeCachedUser(undefined);
      set({ ...cloudAuthClient.state(), user: undefined, phase: "anonymous", loading: false, initialized: true });
    } catch (error) { set({ loading: false, error: error instanceof Error ? error.message : "退出失败" }); }
  },
}));
