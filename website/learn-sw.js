// Lets the flashcard page open without a network once it has been visited.
const CACHE = "qt-learn-v1";
const FILES = ["learn.html", "assets/learn.js", "assets/learn.css", "assets/site.css", "assets/icon.svg", "assets/icon-256.png", "learn.webmanifest"];

self.addEventListener("install", (event) => {
  event.waitUntil(caches.open(CACHE).then((cache) => cache.addAll(FILES)));
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(caches.keys().then((keys) => Promise.all(keys.filter((key) => key.startsWith("qt-learn-") && key !== CACHE).map((key) => caches.delete(key)))));
  self.clients.claim();
});

// Network first so updates arrive; the cache answers when offline.
self.addEventListener("fetch", (event) => {
  const url = new URL(event.request.url);
  if (event.request.method !== "GET" || url.origin !== location.origin) return;
  event.respondWith(
    fetch(event.request)
      .then((response) => {
        if (!response.ok) return response;
        const copy = response.clone();
        return caches
          .open(CACHE)
          .then((cache) => cache.put(event.request, copy))
          .catch(() => {})
          .then(() => response);
      })
      .catch(() => caches.match(event.request, { ignoreSearch: true })),
  );
});
