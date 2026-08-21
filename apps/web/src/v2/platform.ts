import type { LifeTraceState } from "./model";

export interface NativeStatus {
  storage?: unknown;
  sync?: unknown;
  photo?: unknown;
  vault?: unknown;
}

export interface PlatformAdapter {
  kind: "web" | "desktop";
  label: string;
  loadState(): Promise<LifeTraceState | null>;
  saveState(state: LifeTraceState): Promise<void>;
  getNativeStatus?(): Promise<NativeStatus>;
  syncNow?(): Promise<unknown>;
  openExternal?(url: string): Promise<void>;
}

const STORAGE_KEY = "lifetrace.frontend-v2.state";

export const webPlatform: PlatformAdapter = {
  kind: "web",
  label: "Web",
  async loadState() {
    try {
      const raw = window.localStorage.getItem(STORAGE_KEY);
      return raw ? (JSON.parse(raw) as LifeTraceState) : null;
    } catch {
      return null;
    }
  },
  async saveState(state) {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  },
  async openExternal(url) {
    window.open(url, "_blank", "noopener,noreferrer");
  }
};

export { STORAGE_KEY };
