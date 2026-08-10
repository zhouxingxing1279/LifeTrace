export type ClientLogLevel = "debug" | "info" | "warn" | "error" | "fatal";
export type ClientRuntime = "browser" | "tauri" | "node" | "unknown";
export type ClientErrorCategory =
  | "programming"
  | "network"
  | "http"
  | "timeout"
  | "aborted"
  | "parse"
  | "unknown";
export type RequestStage =
  | "request.prepare"
  | "fetch.invoke"
  | "fetch.await"
  | "response.read"
  | "response.parse"
  | "response.http";

export type FetchLike = (
  input: RequestInfo | URL,
  init?: RequestInit,
) => Promise<Response>;

export interface SerializedClientError {
  name: string;
  message: string;
  stack?: string;
  code?: string | number;
  cause?: SerializedClientError;
  value?: unknown;
}

export interface ClientLogContext {
  module?: string;
  action?: string;
  operationId?: string;
  clientTraceId?: string;
  errorId?: string;
  route?: string;
  [key: string]: unknown;
}

export interface ClientLogEvent {
  schemaVersion: 1;
  timestamp: string;
  level: ClientLogLevel;
  event: string;
  runtime: ClientRuntime;
  sessionId: string;
  context?: Record<string, unknown>;
  data?: unknown;
  error?: SerializedClientError;
}

export interface RequestLogContext extends ClientLogContext {
  module: string;
  action: string;
  userMessage?: string;
}

interface FetchOwner {
  fetch: FetchLike;
}

interface RequestFailureOptions extends RequestLogContext {
  stage: RequestStage;
  requestSent: boolean;
  method: string;
  url: string;
  clientTraceId: string;
  durationMs: number;
  status?: number;
  message?: string;
}

const LOG_STORAGE_KEY = "lifetrace.client.logs.v1";
const MAX_MEMORY_EVENTS = 200;
const MAX_PERSISTED_EVENTS = 100;
const MAX_STRING_LENGTH = 4_000;
const MAX_OBJECT_DEPTH = 6;
const MAX_CAUSE_DEPTH = 5;
const SENSITIVE_KEY_PATTERN = /authorization|cookie|set-cookie|password|passwd|token|secret|api[-_]?key|credential/i;
const SENSITIVE_CONTENT_KEYS = new Set([
  "body",
  "bodytext",
  "bodyhtml",
  "bodyhtmlsanitized",
  "raw",
  "rawbody",
  "rawcontent",
  "mailbody",
  "mailcontent",
  "messagebody",
  "notificationbody",
  "notificationcontent",
]);
const BEARER_PATTERN = /Bearer\s+[A-Za-z0-9._~+/=-]+/gi;
const JWT_PATTERN = /eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}/g;
const PRODUCTION_BUILD = Boolean(import.meta.env?.PROD);

const memoryEvents: ClientLogEvent[] = [];
let globalHandlersInstalled = false;
let tauriTransportUnavailable = false;
let tauriInvokePromise: Promise<typeof import("@tauri-apps/api/core").invoke> | null = null;

function detectRuntime(): ClientRuntime {
  if (typeof window !== "undefined") {
    if ("__TAURI_INTERNALS__" in window) return "tauri";
    return "browser";
  }
  if (typeof process !== "undefined" && process.versions?.node) return "node";
  return "unknown";
}

function randomId(prefix: string): string {
  const value = typeof crypto !== "undefined" && typeof crypto.randomUUID === "function"
    ? crypto.randomUUID()
    : `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 12)}`;
  return `${prefix}-${value}`;
}

const sessionId = randomId("session");

export function createOperationId(): string {
  return randomId("op");
}

export function createClientTraceId(): string {
  return randomId("trace");
}

export function createErrorId(): string {
  return randomId("err");
}

function scrubString(value: string): string {
  const redacted = value
    .replace(BEARER_PATTERN, "Bearer [REDACTED]")
    .replace(JWT_PATTERN, "[REDACTED_JWT]");
  return redacted.length > MAX_STRING_LENGTH
    ? `${redacted.slice(0, MAX_STRING_LENGTH)}…[TRUNCATED]`
    : redacted;
}

function isSensitiveLogKey(key: string): boolean {
  if (SENSITIVE_KEY_PATTERN.test(key)) return true;
  const normalized = key.toLowerCase().replace(/[-_]/g, "");
  return SENSITIVE_CONTENT_KEYS.has(normalized);
}

export function sanitizeLogValue(
  value: unknown,
  depth = 0,
  seen: WeakSet<object> = new WeakSet<object>(),
  key = "",
): unknown {
  if (isSensitiveLogKey(key)) return "[REDACTED]";
  if (value === null || value === undefined) return value;
  if (typeof value === "string") return scrubString(value);
  if (typeof value === "number" || typeof value === "boolean") return value;
  if (typeof value === "bigint") return value.toString();
  if (typeof value === "function") return `[Function ${value.name || "anonymous"}]`;
  if (typeof value === "symbol") return value.toString();
  if (depth >= MAX_OBJECT_DEPTH) return "[MAX_DEPTH]";
  if (typeof value !== "object") return String(value);
  if (seen.has(value)) return "[CIRCULAR]";
  seen.add(value);

  if (Array.isArray(value)) {
    return value.slice(0, 50).map((item) => sanitizeLogValue(item, depth + 1, seen));
  }

  const result: Record<string, unknown> = {};
  for (const [entryKey, entryValue] of Object.entries(value).slice(0, 100)) {
    result[entryKey] = sanitizeLogValue(entryValue, depth + 1, seen, entryKey);
  }
  return result;
}

function errorCode(error: Error): string | number | undefined {
  const value = error as Error & { code?: unknown };
  return typeof value.code === "string" || typeof value.code === "number"
    ? value.code
    : undefined;
}

export function serializeClientError(
  error: unknown,
  depth = 0,
  seen: WeakSet<object> = new WeakSet<object>(),
): SerializedClientError {
  if (error instanceof Error) {
    if (seen.has(error)) {
      return { name: error.name || "Error", message: "[CIRCULAR_ERROR]" };
    }
    seen.add(error);
    const cause = (error as Error & { cause?: unknown }).cause;
    return {
      name: scrubString(error.name || "Error"),
      message: scrubString(error.message || String(error)),
      stack: error.stack ? scrubString(error.stack) : undefined,
      code: errorCode(error),
      cause: cause !== undefined && depth < MAX_CAUSE_DEPTH
        ? serializeClientError(cause, depth + 1, seen)
        : undefined,
    };
  }

  return {
    name: typeof error,
    message: scrubString(typeof error === "string" ? error : String(error)),
    value: sanitizeLogValue(error),
  };
}

function persistBrowserEvent(event: ClientLogEvent): void {
  if (typeof window === "undefined") return;
  try {
    const storage = window.localStorage;
    const raw = storage.getItem(LOG_STORAGE_KEY);
    const existing = raw ? JSON.parse(raw) as unknown : [];
    const events = Array.isArray(existing) ? existing : [];
    events.push(event);
    storage.setItem(
      LOG_STORAGE_KEY,
      JSON.stringify(events.slice(-MAX_PERSISTED_EVENTS)),
    );
  } catch {
    // Logging must never break application behavior.
  }
}

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function tauriInvoke(): Promise<typeof import("@tauri-apps/api/core").invoke> {
  if (!tauriInvokePromise) {
    tauriInvokePromise = import("@tauri-apps/api/core").then((module) => module.invoke);
  }
  return tauriInvokePromise;
}

async function persistTauriEvent(event: ClientLogEvent): Promise<void> {
  if (!isTauriRuntime() || tauriTransportUnavailable) return;
  try {
    const invoke = await tauriInvoke();
    await invoke("client_log_write", { event });
  } catch (error) {
    tauriTransportUnavailable = true;
    console.warn("[LifeTrace Logger] Tauri file transport unavailable", serializeClientError(error));
  }
}

function writeConsole(event: ClientLogEvent): void {
  const method = event.level === "fatal" || event.level === "error"
    ? console.error
    : event.level === "warn"
      ? console.warn
      : event.level === "debug"
        ? console.debug
        : console.info;
  try {
    method.call(console, `[LifeTrace] ${event.event}`, event);
  } catch {
    // Console implementations can be replaced by embedded runtimes.
  }
}

function emitEvent(event: ClientLogEvent): void {
  memoryEvents.push(event);
  if (memoryEvents.length > MAX_MEMORY_EVENTS) {
    memoryEvents.splice(0, memoryEvents.length - MAX_MEMORY_EVENTS);
  }
  writeConsole(event);
  persistBrowserEvent(event);
  if (isTauriRuntime()) {
    queueMicrotask(() => {
      void persistTauriEvent(event);
    });
  }
}

export interface ClientLogger {
  child(context: ClientLogContext): ClientLogger;
  debug(event: string, data?: unknown, error?: unknown): void;
  info(event: string, data?: unknown, error?: unknown): void;
  warn(event: string, data?: unknown, error?: unknown): void;
  error(event: string, data?: unknown, error?: unknown): void;
  fatal(event: string, data?: unknown, error?: unknown): void;
}

class StructuredClientLogger implements ClientLogger {
  constructor(private readonly baseContext: ClientLogContext = {}) {}

  child(context: ClientLogContext): ClientLogger {
    return new StructuredClientLogger({ ...this.baseContext, ...context });
  }

  debug(event: string, data?: unknown, error?: unknown): void {
    this.write("debug", event, data, error);
  }

  info(event: string, data?: unknown, error?: unknown): void {
    this.write("info", event, data, error);
  }

  warn(event: string, data?: unknown, error?: unknown): void {
    this.write("warn", event, data, error);
  }

  error(event: string, data?: unknown, error?: unknown): void {
    this.write("error", event, data, error);
  }

  fatal(event: string, data?: unknown, error?: unknown): void {
    this.write("fatal", event, data, error);
  }

  private write(level: ClientLogLevel, event: string, data?: unknown, error?: unknown): void {
    if (level === "debug" && PRODUCTION_BUILD) return;
    try {
      const context = sanitizeLogValue(this.baseContext) as Record<string, unknown>;
      emitEvent({
        schemaVersion: 1,
        timestamp: new Date().toISOString(),
        level,
        event: scrubString(event),
        runtime: detectRuntime(),
        sessionId,
        context: Object.keys(context).length ? context : undefined,
        data: data === undefined ? undefined : sanitizeLogValue(data),
        error: error === undefined ? undefined : serializeClientError(error),
      });
    } catch (loggingError) {
      try {
        console.error("[LifeTrace Logger] Failed to create log event", loggingError);
      } catch {
        // Nothing else is safe to do here.
      }
    }
  }
}

export const clientLogger: ClientLogger = new StructuredClientLogger();

export function getRecentClientLogs(): ClientLogEvent[] {
  return memoryEvents.map((event) => structuredClone(event));
}

export function clearRecentClientLogs(): void {
  memoryEvents.length = 0;
  if (typeof window !== "undefined") {
    try {
      window.localStorage?.removeItem(LOG_STORAGE_KEY);
    } catch {
      // Ignore storage errors.
    }
  }
}

export function installGlobalErrorHandlers(): void {
  if (globalHandlersInstalled || typeof window === "undefined") return;
  globalHandlersInstalled = true;

  window.addEventListener("error", (event) => {
    const errorId = createErrorId();
    clientLogger.fatal(
      "renderer.uncaught_error",
      {
        errorId,
        message: event.message,
        filename: event.filename,
        line: event.lineno,
        column: event.colno,
        route: window.location?.pathname,
      },
      event.error ?? new Error(event.message || "Unknown renderer error"),
    );
  });

  window.addEventListener("unhandledrejection", (event) => {
    const errorId = createErrorId();
    clientLogger.error(
      "renderer.unhandled_rejection",
      { errorId, route: window.location?.pathname },
      event.reason,
    );
  });

  clientLogger.info("renderer.error_handlers.installed", {
    route: window.location?.pathname,
  });
}

export function classifyClientError(
  error: unknown,
  stage: RequestStage,
): ClientErrorCategory {
  if (error instanceof DOMException && error.name === "AbortError") return "aborted";
  const serialized = serializeClientError(error);
  const text = `${serialized.name}: ${serialized.message}`;

  if (/AbortError|aborted|abort(ed)?/i.test(text)) return "aborted";
  if (/TimeoutError|timed?\s*out|timeout/i.test(text)) return "timeout";
  if (
    /window\.fetch|instance of Window|Illegal invocation|is not a function|Cannot read (properties|property)|undefined is not/i.test(text)
  ) return "programming";
  if (stage === "response.parse") return "parse";
  if (stage === "response.http") return "http";
  if (stage === "fetch.invoke") return "programming";
  if (stage === "fetch.await" && error instanceof TypeError) return "network";
  return "unknown";
}

export class ClientRequestError extends Error {
  readonly category: ClientErrorCategory;
  readonly stage: RequestStage;
  readonly requestSent: boolean;
  readonly clientTraceId: string;
  readonly status?: number;
  readonly errorId: string;

  constructor(
    message: string,
    options: {
      cause: unknown;
      category: ClientErrorCategory;
      stage: RequestStage;
      requestSent: boolean;
      clientTraceId: string;
      status?: number;
      errorId?: string;
    },
  ) {
    super(message, { cause: options.cause });
    this.name = "ClientRequestError";
    this.category = options.category;
    this.stage = options.stage;
    this.requestSent = options.requestSent;
    this.clientTraceId = options.clientTraceId;
    this.status = options.status;
    this.errorId = options.errorId ?? createErrorId();
  }
}

function errorMessage(error: unknown): string {
  if (error instanceof Error && error.message.trim()) return error.message.trim();
  const value = String(error).trim();
  return value || "Unknown error";
}

export function createLoggedRequestError(
  cause: unknown,
  options: RequestFailureOptions,
): ClientRequestError {
  if (cause instanceof ClientRequestError) return cause;
  const category = classifyClientError(cause, options.stage);
  const errorId = createErrorId();
  const originalMessage = errorMessage(cause);
  const message = options.message
    ?? (category === "programming"
      ? `LifeTrace 客户端请求执行失败：${originalMessage}`
      : options.userMessage
        ? `${options.userMessage}（${originalMessage}）`
        : originalMessage);

  const requestError = new ClientRequestError(message, {
    cause,
    category,
    stage: options.stage,
    requestSent: options.requestSent,
    clientTraceId: options.clientTraceId,
    status: options.status,
    errorId,
  });

  clientLogger.error(
    "api.request.failed",
    {
      module: options.module,
      action: options.action,
      operationId: options.operationId,
      clientTraceId: options.clientTraceId,
      errorId,
      method: options.method,
      url: options.url,
      stage: options.stage,
      requestSent: options.requestSent,
      category,
      status: options.status,
      durationMs: options.durationMs,
    },
    requestError,
  );
  return requestError;
}

export function bindFetch(fetcher: FetchLike, owner?: FetchOwner): FetchLike {
  const inferredOwner = owner
    ?? (typeof globalThis.fetch === "function" && fetcher === globalThis.fetch
      ? globalThis as unknown as FetchOwner
      : undefined);

  if (inferredOwner && inferredOwner.fetch === fetcher) {
    return (input, init) => fetcher.call(inferredOwner, input, init);
  }
  return (input, init) => fetcher(input, init);
}

function requestUrl(input: RequestInfo | URL): string {
  const raw = typeof input === "string"
    ? input
    : input instanceof URL
      ? input.toString()
      : input.url;
  try {
    const base = typeof window !== "undefined" ? window.location.origin : "http://localhost";
    const parsed = new URL(raw, base);
    const origin = parsed.origin === "null" ? "" : parsed.origin;
    return `${origin}${parsed.pathname}`;
  } catch {
    return raw.split(/[?#]/, 1)[0] || "unknown-request";
  }
}

export async function instrumentedFetch(
  fetcher: FetchLike,
  input: RequestInfo | URL,
  init: RequestInit | undefined,
  context: RequestLogContext,
): Promise<Response> {
  const startedAt = performance.now();
  const url = requestUrl(input);
  const method = (init?.method || "GET").toUpperCase();
  const clientTraceId = context.clientTraceId || createClientTraceId();
  const operationId = context.operationId || createOperationId();
  const safeFetcher = bindFetch(fetcher);

  clientLogger.info("api.request.start", {
    module: context.module,
    action: context.action,
    operationId,
    clientTraceId,
    method,
    url,
  });

  let responsePromise: Promise<Response>;
  try {
    clientLogger.debug("api.fetch.invoke", {
      module: context.module,
      action: context.action,
      operationId,
      clientTraceId,
      method,
      url,
    });
    responsePromise = safeFetcher(input, init);
  } catch (cause) {
    throw createLoggedRequestError(cause, {
      ...context,
      operationId,
      clientTraceId,
      method,
      url,
      stage: "fetch.invoke",
      requestSent: false,
      durationMs: Math.round(performance.now() - startedAt),
    });
  }

  let response: Response;
  try {
    response = await responsePromise;
  } catch (cause) {
    throw createLoggedRequestError(cause, {
      ...context,
      operationId,
      clientTraceId,
      method,
      url,
      stage: "fetch.await",
      requestSent: true,
      durationMs: Math.round(performance.now() - startedAt),
    });
  }

  clientLogger.info("api.response.received", {
    module: context.module,
    action: context.action,
    operationId,
    clientTraceId,
    method,
    url,
    status: response.status,
    ok: response.ok,
    durationMs: Math.round(performance.now() - startedAt),
  });
  return response;
}