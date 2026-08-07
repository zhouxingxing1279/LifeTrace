import type { FetchLike } from "./types";

/**
 * Return an ordinary function that always invokes the browser's fetch with
 * globalThis as its receiver. Storing native fetch directly on an API class and
 * later calling it as `this.fetcher(...)` changes the receiver to the class
 * instance in Chromium and throws before a request reaches the network stack.
 */
export function createBrowserFetch(): FetchLike {
  return (input, init) => globalThis.fetch(input, init);
}

export const browserFetch: FetchLike = createBrowserFetch();
