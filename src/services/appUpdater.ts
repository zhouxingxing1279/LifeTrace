import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type AppUpdateState =
  | { status: "idle" }
  | { status: "checking" }
  | {
      status: "available";
      version: string;
      currentVersion: string;
      notes: string | null;
      date: string | null;
    }
  | {
      status: "downloading";
      version: string;
      downloadedBytes: number;
      totalBytes: number | null;
      percentage: number | null;
    }
  | { status: "installing"; version: string }
  | { status: "upToDate" }
  | { status: "error"; message: string };

export interface AvailableAppUpdate {
  version: string;
  currentVersion: string;
  notes: string | null;
  date: string | null;
  downloadAndInstall: (
    onProgress?: (state: AppUpdateState) => void,
  ) => Promise<void>;
}

/** True only inside the Tauri desktop WebView, never in a plain browser/PWA. */
export function isTauriDesktopRuntime(): boolean {
  return (
    typeof window !== "undefined" &&
    "__TAURI_INTERNALS__" in window
  );
}

/** Whether the current build should run the silent startup update check. */
export function shouldAutoCheckForUpdate(options?: {
  isTauri?: boolean;
  isDev?: boolean;
}): boolean {
  const isTauri = options?.isTauri ?? isTauriDesktopRuntime();
  const isDev =
    options?.isDev ??
    (typeof import.meta !== "undefined" && import.meta.env?.DEV === true);
  return isTauri && !isDev;
}

/** Returns a 0-100 percentage, or null when the total size is unknown. */
export function calculateDownloadProgress(
  downloadedBytes: number,
  totalBytes: number | null,
): number | null {
  if (typeof totalBytes !== "number" || !Number.isFinite(totalBytes) || totalBytes <= 0) {
    return null;
  }
  const raw = (downloadedBytes / totalBytes) * 100;
  return Math.max(0, Math.min(100, Math.round(raw)));
}

/** Converts unknown errors thrown by the updater into user readable text. */
export function normalizeUpdateError(error: unknown): string {
  if (!error) return "更新失败，请稍后重试。";
  const message = error instanceof Error ? error.message : String(error);
  const lower = message.toLowerCase();
  if (/signature|signing|minisign|public key|key mismatch|invalid signature/i.test(lower)) {
    return "更新签名校验失败，安装包可能被篡改或签名密钥不匹配。";
  }
  if (/fetch|network|connect|timeout|tls|certificate|reqwest|dns|request/i.test(lower)) {
    return "网络连接失败，请检查网络后重试。";
  }
  if (/json|parse|malformed|unexpected|semver|version/i.test(lower)) {
    return "更新信息格式不正确，请稍后重试。";
  }
  if (/download|write|permission|disk|space/i.test(lower)) {
    return "下载或写入安装包失败，请检查磁盘空间后重试。";
  }
  if (/install|launch|spawn|execute/i.test(lower)) {
    return "安装更新失败，请稍后重试或到官网手动下载安装。";
  }
  if (lower.includes("not a tauri") || lower.includes("不支持自动更新")) {
    return message;
  }
  return message.trim() || "更新失败，请稍后重试。";
}

/**
 * Runs one async task at a time: concurrent callers share the in-flight
 * promise instead of starting a second task.
 */
export function createSingleFlight<T>() {
  let active: Promise<T> | null = null;
  return {
    run(task: () => Promise<T>): Promise<T> {
      if (active) return active;
      const current = Promise.resolve().then(task);
      active = current;
      current.then(
        () => {
          if (active === current) active = null;
        },
        () => {
          if (active === current) active = null;
        },
      );
      return current;
    },
  };
}

export interface UpdaterRuntime {
  isTauri: () => boolean;
  check: () => Promise<Update | null>;
  relaunch: () => Promise<void>;
}

export interface AppUpdaterService {
  checkForAppUpdate: () => Promise<AvailableAppUpdate | null>;
}

/**
 * Builds the updater service against an explicit runtime. The default export
 * below wires the real Tauri plugins; tests can inject fakes.
 */
export function createAppUpdaterService(runtime: UpdaterRuntime): AppUpdaterService {
  const checkFlight = createSingleFlight<AvailableAppUpdate | null>();
  const downloadFlight = createSingleFlight<void>();

  return {
    checkForAppUpdate() {
      if (!runtime.isTauri()) {
        throw new Error("当前环境不支持自动更新，请使用 LifeTrace 桌面版。");
      }
      return checkFlight.run(async () => {
        try {
          const update = await runtime.check();
          if (!update) return null;
          return {
            version: update.version,
            currentVersion: update.currentVersion,
            notes: update.body || null,
            date: update.date || null,
            downloadAndInstall: (onProgress) =>
              downloadFlight.run(async () => {
                let downloadedBytes = 0;
                let totalBytes: number | null = null;
                const emit = (state: AppUpdateState) => onProgress?.(state);

                try {
                  await update.downloadAndInstall((event: DownloadEvent) => {
                    if (event.event === "Started") {
                      downloadedBytes = 0;
                      totalBytes = event.data.contentLength ?? null;
                      emit({
                        status: "downloading",
                        version: update.version,
                        downloadedBytes: 0,
                        totalBytes,
                        percentage: calculateDownloadProgress(0, totalBytes),
                      });
                    } else if (event.event === "Progress") {
                      downloadedBytes += event.data.chunkLength;
                      emit({
                        status: "downloading",
                        version: update.version,
                        downloadedBytes,
                        totalBytes,
                        percentage: calculateDownloadProgress(
                          downloadedBytes,
                          totalBytes,
                        ),
                      });
                    } else if (event.event === "Finished") {
                      emit({ status: "installing", version: update.version });
                    }
                  });

                  await runtime.relaunch();
                } catch (error) {
                  throw new Error(normalizeUpdateError(error));
                }
              }),
          };
        } catch (error) {
          throw new Error(normalizeUpdateError(error));
        }
      });
    },
  };
}

const defaultService = createAppUpdaterService({
  isTauri: isTauriDesktopRuntime,
  check,
  relaunch,
});

/**
 * Checks the configured update endpoint. Returns null when the app is up to
 * date, otherwise an update handle that can download and install.
 *
 * Only works in the Tauri desktop runtime; throws a readable error otherwise
 * so callers can surface it in the UI instead of crashing the app.
 */
export async function checkForAppUpdate(): Promise<AvailableAppUpdate | null> {
  return defaultService.checkForAppUpdate();
}
