import { create } from "zustand";
import { cloudAuthClient, type CloudAuthSnapshot } from "@/src/services/cloudAuth";

type CloudAuthState = CloudAuthSnapshot & {
  origin: string;
  loading: boolean;
  error?: string;
  setOrigin(origin: string): void;
  login(email: string, password: string): Promise<void>;
  restore(): Promise<void>;
  logout(all?: boolean): Promise<void>;
  bindCurrentProfile(): Promise<void>;
  createCloudProfile(): Promise<void>;
  clearError(): void;
};

export const useCloudAuthStore = create<CloudAuthState>((set, get) => ({
  origin: "",
  scopes: [],
  authenticated: false,
  loading: false,
  clearError() { set({ error: undefined }); },
  setOrigin(origin) {
    set({ origin, error: undefined });
    try { cloudAuthClient.configure(origin); } catch (error) {
      set({ error: error instanceof Error ? error.message : "云服务地址无效" });
    }
  },
  async login(email, password) {
    set({ loading: true, error: undefined });
    try {
      cloudAuthClient.configure(get().origin);
      set({ ...(await cloudAuthClient.login(email, password)), loading: false });
    } catch (error) {
      set({ loading: false, error: error instanceof Error ? error.message : "登录失败" });
    }
  },
  async restore() {
    if (!get().origin) return;
    set({ loading: true, error: undefined });
    cloudAuthClient.configure(get().origin);
    set({ ...(await cloudAuthClient.restore()), loading: false });
  },
  async bindCurrentProfile() {
    const userId = get().user?.id;
    if (!userId || !window.syncApi) return;
    set({ loading: true, error: undefined });
    try {
      await window.syncApi.bindCurrentProfile();
      set({ binding: { profileId: get().binding?.profileId || "", cloudUserId: userId, bindingRequired: false, alreadyBound: true }, loading: false });
      await window.syncApi.now(false);
    } catch (error) {
      set({ loading: false, error: error instanceof Error ? error.message : String(error) });
    }
  },
  async createCloudProfile() {
    const user = get().user;
    if (!user || !window.syncApi) return;
    set({ loading: true, error: undefined });
    try {
      const profileId = await window.syncApi.createCloudProfile(user.displayName || user.email || "云端资料");
      set({ binding: { profileId, cloudUserId: user.id, bindingRequired: false, alreadyBound: true }, loading: false });
      await window.syncApi.now(true);
    } catch (error) {
      set({ loading: false, error: error instanceof Error ? error.message : String(error) });
    }
  },
  async logout(all = false) {
    set({ loading: true, error: undefined });
    try {
      await cloudAuthClient.logout(all);
      set({ ...cloudAuthClient.state(), loading: false });
    } catch (error) {
      set({ loading: false, error: error instanceof Error ? error.message : "退出失败" });
    }
  },
}));
