import {
  clientLogger,
  getRecentClientLogs,
  sanitizeLogValue,
  type ClientLogEvent,
} from "./clientObservability";

export interface ClientDiagnosticSnapshot {
  generatedAt: string;
  runtime: "browser" | "tauri";
  location?: string;
  userAgent?: string;
  language?: string;
  platform?: string;
  logPath?: string;
  recentEvents: ClientLogEvent[];
  nativeLogTail?: string;
}

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function invokeTauri<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(command, args);
}

export async function getClientLogPath(): Promise<string | null> {
  if (!isTauriRuntime()) return null;
  try {
    return await invokeTauri<string>("client_log_path");
  } catch (error) {
    clientLogger.warn("diagnostics.log_path.failed", undefined, error);
    return null;
  }
}

export async function readRecentNativeClientLog(maxBytes = 256 * 1024): Promise<string> {
  if (!isTauriRuntime()) return "";
  try {
    return await invokeTauri<string>("client_log_read_recent", {
      maxBytes: Math.max(4 * 1024, Math.min(maxBytes, 1024 * 1024)),
    });
  } catch (error) {
    clientLogger.warn("diagnostics.log_read.failed", { maxBytes }, error);
    return "";
  }
}

function safeLocation(): string | undefined {
  if (typeof window === "undefined") return undefined;
  try {
    return `${window.location.origin}${window.location.pathname}`;
  } catch {
    return undefined;
  }
}

export async function createClientDiagnosticSnapshot(): Promise<ClientDiagnosticSnapshot> {
  const [logPath, nativeLogTail] = await Promise.all([
    getClientLogPath(),
    readRecentNativeClientLog(),
  ]);

  const snapshot: ClientDiagnosticSnapshot = {
    generatedAt: new Date().toISOString(),
    runtime: isTauriRuntime() ? "tauri" : "browser",
    location: safeLocation(),
    userAgent: typeof navigator !== "undefined" ? navigator.userAgent : undefined,
    language: typeof navigator !== "undefined" ? navigator.language : undefined,
    platform: typeof navigator !== "undefined" ? navigator.platform : undefined,
    logPath: logPath ?? undefined,
    recentEvents: getRecentClientLogs(),
    nativeLogTail: nativeLogTail || undefined,
  };

  return sanitizeLogValue(snapshot) as unknown as ClientDiagnosticSnapshot;
}

export async function copyClientDiagnostics(): Promise<ClientDiagnosticSnapshot> {
  const snapshot = await createClientDiagnosticSnapshot();
  const text = JSON.stringify(snapshot, null, 2);

  if (typeof navigator === "undefined" || !navigator.clipboard?.writeText) {
    throw new Error("当前运行环境不支持复制诊断信息");
  }

  await navigator.clipboard.writeText(text);
  clientLogger.info("diagnostics.copy.succeeded", {
    eventCount: snapshot.recentEvents.length,
    bytes: text.length,
  });
  return snapshot;
}
