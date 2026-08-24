import type { FetchLike } from "./types";

let runtimeCloudFetch: FetchLike | undefined;

/**
 * Allow native shells to reuse the browser cloud feature layer without sending
 * requests through the WebView's ordinary fetch implementation. The browser
 * leaves this unset; the Tauri desktop shell installs its authenticated native
 * transport while the shared cloud workspace is mounted.
 */
export function setCloudFetchOverride(fetcher?: FetchLike): void {
  runtimeCloudFetch = fetcher;
}

/**
 * Return an ordinary function that always invokes the browser's fetch with
 * globalThis as its receiver. Storing native fetch directly on an API class and
 * later calling it as `this.fetcher(...)` changes the receiver to the class
 * instance in Chromium and throws before a request reaches the network stack.
 */
export function createBrowserFetch(): FetchLike {
  return (input, init) => {
    if (runtimeCloudFetch) return runtimeCloudFetch(input, init);
    return globalThis.fetch(input, init);
  };
}

export const browserFetch: FetchLike = createBrowserFetch();
