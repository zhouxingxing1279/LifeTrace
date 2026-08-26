import { invoke } from "@tauri-apps/api/core";

const nativeFetch = window.fetch.bind(window);
const backendOrigin = "http://127.0.0.1:3103";

interface LocalServiceStatus {
  phase?: string;
  error?: string | null;
}

const delay = (milliseconds: number) => new Promise((resolve) => window.setTimeout(resolve, milliseconds));

async function fetchWithTimeout(url: string, timeoutMilliseconds: number) {
  const controller = new AbortController();
  const timer = window.setTimeout(() => controller.abort(), timeoutMilliseconds);
  try {
    return await nativeFetch(url, {
      cache: "no-store",
      signal: controller.signal,
    });
  } finally {
    window.clearTimeout(timer);
  }
}

async function readLocalServiceStatus(): Promise<LocalServiceStatus | undefined> {
  try {
    return await invoke<LocalServiceStatus>("local_service_status");
  } catch {
    return undefined;
  }
}

export async function waitForTauriBackend(
  timeoutMilliseconds = 45_000,
  onStatus?: (message: string) => void,
) {
  const deadline = Date.now() + timeoutMilliseconds;
  const startedAt = Date.now();
  let lastError: unknown;
  let lastStatus: LocalServiceStatus | undefined;
  let lastReportedSecond = -1;

  while (Date.now() < deadline) {
    lastStatus = await readLocalServiceStatus();
    if (lastStatus?.phase === "failed") {
      throw new Error(`Rust 本地服务启动失败：${lastStatus.error || "未知错误"}`);
    }

    const elapsedSeconds = Math.floor((Date.now() - startedAt) / 1_000);
    if (elapsedSeconds !== lastReportedSecond && elapsedSeconds % 3 === 0) {
      lastReportedSecond = elapsedSeconds;
      onStatus?.(`正在启动本地 SQLite 服务… ${elapsedSeconds}s`);
    }

    try {
      const response = await fetchWithTimeout(`${backendOrigin}/api/health`, 2_000);
      if (response.ok) {
        const payload = await response.json() as { ok?: boolean; runtime?: string };
        if (payload.ok && payload.runtime === "tauri-rust") return;
      }
      lastError = new Error(`本地服务返回 HTTP ${response.status}`);
    } catch (error) {
      lastError = error;
    }

    await delay(200);
  }

  const stateDetail = lastStatus?.phase ? ` 服务状态：${lastStatus.phase}。` : "";
  const errorDetail = lastError instanceof Error ? ` 最近错误：${lastError.message}` : "";
  throw new Error(
    `Rust 本地服务在 ${Math.round(timeoutMilliseconds / 1_000)} 秒内未能启动。${stateDetail}${errorDetail}`,
  );
}
