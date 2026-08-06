const VERSION = "lifetrace-web-cloud-v1";

self.addEventListener("install", () => {
  // Do not pre-cache the application shell. LifeTrace Web requires a network
  // connection and keeps all business data in LifeTrace Cloud.
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches.keys()
      .then((keys) => Promise.all(keys.map((key) => caches.delete(key))))
      .then(() => self.clients.claim()),
  );
});

self.addEventListener("message", (event) => {
  if (event.data?.type === "SKIP_WAITING") self.skipWaiting();
});

// Intentionally no fetch handler: pages, assets and API calls remain network
// requests. This prevents stale application code and private API responses from
// being served from a browser cache.
void VERSION;
