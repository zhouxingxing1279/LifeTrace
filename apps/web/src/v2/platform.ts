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

// Browser Web is cloud-first. Keep unsynced V2 state only for the lifetime of
// the current document so SPA navigation remains responsive without turning
// the browser into a second durable application database. Durable data belongs
// behind the authenticated cloud/sync adapter; Desktop owns local-first storage.
let sessionState: LifeTraceState | null = null;

const cloneState = (state: LifeTraceState): LifeTraceState => structuredClone(state);

export const webPlatform: PlatformAdapter = {
  kind: "web",
  label: "Web",
  async loadState() {
    return sessionState ? cloneState(sessionState) : null;
  },
  async saveState(state) {
    sessionState = cloneState(state);
  },
  async openExternal(url) {
    window.open(url, "_blank", "noopener,noreferrer");
  }
};
