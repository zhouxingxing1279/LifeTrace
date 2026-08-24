export type FetchLike = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;

export class ClientRequestError extends Error {
  readonly category: "programming" | "network" | "unknown";
  readonly errorId: string;

  constructor(message: string, options?: { cause?: unknown; category?: "programming" | "network" | "unknown" }) {
    super(message, { cause: options?.cause });
    this.name = "ClientRequestError";
    this.category = options?.category ?? "unknown";
    this.errorId = typeof crypto !== "undefined" && crypto.randomUUID ? crypto.randomUUID() : String(Date.now());
  }
}

export function bindFetch(fetcher: FetchLike): FetchLike {
  return (input, init) => {
    try {
      return fetcher(input, init);
    } catch (cause) {
      throw new ClientRequestError("LifeTrace 客户端请求执行失败", { cause, category: "programming" });
    }
  };
}

function write(level: "info" | "warn" | "error", event: string, data?: unknown, error?: unknown) {
  const method = level === "error" ? console.error : level === "warn" ? console.warn : console.info;
  method(`[LifeTrace] ${event}`, data ?? "", error ?? "");
}

export const clientLogger = {
  info: (event: string, data?: unknown, error?: unknown) => write("info", event, data, error),
  warn: (event: string, data?: unknown, error?: unknown) => write("warn", event, data, error),
  error: (event: string, data?: unknown, error?: unknown) => write("error", event, data, error),
};

export function installGlobalErrorHandlers(): void {
  if (typeof window === "undefined") return;
  window.addEventListener("error", (event) => clientLogger.error("renderer.uncaught_error", { route: location.pathname }, event.error));
  window.addEventListener("unhandledrejection", (event) => clientLogger.error("renderer.unhandled_rejection", { route: location.pathname }, event.reason));
}
