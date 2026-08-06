import { create } from "zustand";
import {
  checkForAppUpdate,
  normalizeUpdateError,
  type AppUpdateState,
  type AvailableAppUpdate,
} from "@/src/services/appUpdater";

type CheckOrigin = "manual" | "auto";

interface AppUpdaterStore {
  state: AppUpdateState;
  update: AvailableAppUpdate | null;
  checking: boolean;
  check: (origin?: CheckOrigin) => Promise<void>;
  download: () => Promise<void>;
  dismiss: () => void;
}

export const useAppUpdaterStore = create<AppUpdaterStore>((set, get) => ({
  state: { status: "idle" },
  update: null,
  checking: false,

  check: async (origin = "manual") => {
    const current = get().state;
    if (
      get().checking ||
      current.status === "downloading" ||
      current.status === "installing"
    ) {
      return;
    }
    set({ checking: true, state: { status: "checking" } });
    try {
      const update = await checkForAppUpdate();
      if (update) {
        set({
          update,
          checking: false,
          state: {
            status: "available",
            version: update.version,
            currentVersion: update.currentVersion,
            notes: update.notes,
            date: update.date,
          },
        });
      } else {
        set({ update: null, checking: false, state: { status: "upToDate" } });
      }
    } catch (error) {
      const message = normalizeUpdateError(error);
      if (origin === "manual") {
        set({ update: null, checking: false, state: { status: "error", message } });
      } else {
        // Silent startup checks must never block the app: log and stay idle.
        console.warn("[appUpdater] 启动静默检查更新失败：", error);
        set({ update: null, checking: false, state: { status: "idle" } });
      }
    }
  },

  download: async () => {
    const { update, state } = get();
    if (!update) return;
    if (state.status === "downloading" || state.status === "installing") return;
    set({
      state: {
        status: "downloading",
        version: update.version,
        downloadedBytes: 0,
        totalBytes: null,
        percentage: null,
      },
    });
    try {
      await update.downloadAndInstall((next) => {
        if (next.status === "downloading" || next.status === "installing") {
          set({ state: next });
        }
      });
      // The service relaunches the app after a successful install; if the
      // process is still alive (e.g. the update did not require a restart),
      // keep the "installing" notice visible instead of resetting the state.
    } catch (error) {
      set({ state: { status: "error", message: normalizeUpdateError(error) } });
    }
  },

  dismiss: () => set({ state: { status: "idle" }, update: null }),
}));
