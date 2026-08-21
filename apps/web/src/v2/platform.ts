import type { LifeTraceState } from "./model";
import { cloudStateRepository, type CloudSession } from "./api/cloud";

export interface NativeStatus {
  storage?: unknown;
  sync?: unknown;
  photo?: unknown;
  vault?: unknown;
}

export interface PlatformAdapter {
  kind: "web" | "desktop";
  label: string;
  requiresAuthentication?: boolean;
  loadState(): Promise<LifeTraceState | null>;
  saveState(state: LifeTraceState): Promise<void>;
  getSession?(): Promise<CloudSession>;
  login?(email: string, password: string): Promise<CloudSession>;
  logout?(): Promise<void>;
  getNativeStatus?(): Promise<NativeStatus>;
  syncNow?(): Promise<unknown>;
  openExternal?(url: string): Promise<void>;
}

export const webPlatform: PlatformAdapter = {
  kind: "web",
  label: "Web Cloud",
  requiresAuthentication: true,
  async getSession() {
    return cloudStateRepository.getSession();
  },
  async login(email, password) {
    return cloudStateRepository.login(email, password);
  },
  async logout() {
    await cloudStateRepository.logout();
  },
  async loadState() {
    return cloudStateRepository.loadState();
  },
  async saveState(state) {
    await cloudStateRepository.saveState(state);
  },
  async syncNow() {
    return cloudStateRepository.loadState();
  },
  async openExternal(url) {
    window.open(url, "_blank", "noopener,noreferrer");
  }
};
