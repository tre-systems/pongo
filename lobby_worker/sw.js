// Service worker for Pongo's PWA: an offline-capable app shell with versioned
// caching. CACHE_VERSION is stamped at build time (scripts/stamp-sw.mjs) so every
// deploy ships a distinct sw.js — that byte change is what makes the browser fetch
// the new worker and lets pwa.js show the "update available" prompt.
const CACHE_VERSION = "__CACHE_VERSION__";
const CACHE_NAME = `pongo-${CACHE_VERSION}`;

// Precache enough to boot the menu and the offline VS-AI game. Everything else
// (the WASM, served with a ?v= cache-buster) is cached on first fetch below.
const APP_SHELL = [
  "/",
  "/style.css",
  "/manifest.webmanifest",
  "/script.js",
  "/pwa.js",
  "/wasm.js",
  "/audio.js",
  "/overlays.js",
  "/input.js",
];

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches
      .open(CACHE_NAME)
      .then((cache) => cache.addAll(APP_SHELL))
      .catch(() => {})
  );
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((names) =>
        Promise.all(names.filter((n) => n !== CACHE_NAME).map((n) => caches.delete(n)))
      )
      .then(() => self.clients.claim())
  );
});

self.addEventListener("fetch", (event) => {
  const req = event.request;
  if (req.method !== "GET") return;

  const url = new URL(req.url);
  if (url.origin !== self.location.origin) return;

  // Dynamic endpoints must never be cached (match creation must stay fresh;
  // WebSocket upgrades aren't fetch events, so /ws is untouched).
  if (
    url.pathname === "/create" ||
    url.pathname.startsWith("/ws/") ||
    url.pathname.startsWith("/join/")
  ) {
    return;
  }

  // Static assets (JS modules, WASM, CSS, icons, manifest) — cache-first for
  // speed and offline, populating the cache on first fetch.
  const isStatic =
    url.pathname.startsWith("/client_wasm/") ||
    url.pathname.startsWith("/icons/") ||
    /\.(js|css|wasm|png|svg|webmanifest)$/.test(url.pathname);

  if (isStatic) {
    event.respondWith(
      caches.match(req).then(
        (cached) =>
          cached ||
          fetch(req).then((res) => {
            if (res && res.status === 200) {
              const copy = res.clone();
              caches.open(CACHE_NAME).then((cache) => cache.put(req, copy));
            }
            return res;
          })
      )
    );
    return;
  }

  // Navigations and everything else — network-first so updates flow immediately,
  // falling back to the cached shell when offline.
  event.respondWith(
    fetch(req)
      .then((res) => {
        if (res && res.status === 200 && res.type === "basic") {
          const copy = res.clone();
          caches.open(CACHE_NAME).then((cache) => cache.put(req, copy));
        }
        return res;
      })
      .catch(() => caches.match(req).then((cached) => cached || caches.match("/")))
  );
});
