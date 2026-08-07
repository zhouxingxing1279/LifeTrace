import {
  bindFetch,
  clientLogger,
  instrumentedFetch,
  type FetchLike,
} from "./clientObservability";

const INSTRUMENTED_FETCH_KEY = "__lifetraceInstrumentedFetch" as const;
type MarkedFetch = FetchLike & { [INSTRUMENTED_FETCH_KEY]?: boolean };
let installed = false;

function requestAction(input: RequestInfo | URL): string {
  try {
    const raw = typeof input === "string"
      ? input
      : input instanceof URL
        ? input.toString()
        : input.url;
    const url = new URL(raw, typeof window !== "undefined" ? window.location.origin : "http://localhost");
    return `${url.origin === "null" ? "" : url.origin}${url.pathname}`;
  } catch {
    return "unknown-request";
  }
}

function isAlreadyInstrumented(fetcher: FetchLike): boolean {
  return Boolean((fetcher as MarkedFetch)[INSTRUMENTED_FETCH_KEY]);
}

export function installGlobalFetchInstrumentation(): void {
  if (installed || typeof globalThis.fetch !== "function") return;
  installed = true;

  const currentFetch = globalThis.fetch as FetchLike;
  if (isAlreadyInstrumented(currentFetch)) return;

  const nativeFetch = bindFetch(currentFetch);
  const observedFetch: MarkedFetch = (input, init) => instrumentedFetch(
    nativeFetch,
    input,
    init,
    {
      module: "http",
      action: requestAction(input),
      userMessage: "网络请求未能完成",
    },
  );

  Object.defineProperty(observedFetch, INSTRUMENTED_FETCH_KEY, {
    configurable: false,
    enumerable: false,
    value: true,
    writable: false,
  });

  globalThis.fetch = observedFetch as typeof globalThis.fetch;
  clientLogger.info("api.fetch.instrumentation.installed");
}
