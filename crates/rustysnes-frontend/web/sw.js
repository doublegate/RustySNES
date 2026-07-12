// RustySNES wasm demo — PWA/offline service worker (`v1.6.0 "Lighthouse"`).
//
// Strategy: stale-while-revalidate for every same-origin GET. trunk fingerprints the wasm/JS
// glue by content hash in release builds, so a cached copy of a given hashed asset is always
// correct (it can never go stale under a different name); `index.html` itself is unhashed and
// always re-fetched in the background so a new deploy is picked up on the next load without
// requiring an explicit cache-bust. CACHE_VERSION only needs bumping if this file's own caching
// strategy changes, not on every app release.
const CACHE_VERSION = "v1";
const CACHE_NAME = `rustysnes-shell-${CACHE_VERSION}`;

self.addEventListener("install", (event) => {
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(
          keys.filter((key) => key !== CACHE_NAME).map((key) => caches.delete(key))
        )
      )
      .then(() => self.clients.claim())
  );
});

self.addEventListener("fetch", (event) => {
  const request = event.request;
  if (request.method !== "GET") return;
  const url = new URL(request.url);
  if (url.origin !== self.location.origin) return;
  // Never intercept the rustdoc (/api/) or MkDocs (/docs/) sections — this service worker's
  // scope is the wasm demo only; those are plain static sites with their own caching needs.
  if (url.pathname.includes("/api/") || url.pathname.includes("/docs/")) return;

  event.respondWith(
    caches.open(CACHE_NAME).then((cache) =>
      cache.match(request).then((cached) => {
        const network = fetch(request)
          .then((response) => {
            if (response && response.ok) {
              cache.put(request, response.clone());
            }
            return response;
          })
          // A truly offline first visit has no cached response either — respondWith() requires
          // resolving to an actual Response, never undefined, or the fetch handler throws.
          .catch(
            () =>
              cached ||
              new Response("Offline and not yet cached.", {
                status: 503,
                statusText: "Offline",
              })
          );
        return cached || network;
      })
    )
  );
});
