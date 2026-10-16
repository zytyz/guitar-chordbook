// Offline support: always try the network first so a new version shows up
// straight away, and fall back to the last copy seen when offline.

const CACHE = "chordbook";
const SHELL = [
  "./",
  "index.html",
  "style.css",
  "app.js",
  "pkg/chordbook_web.js",
  "pkg/chordbook_web_bg.wasm",
  "manifest.webmanifest",
  "icon.svg",
  "fonts/LiberationSans-Regular.ttf",
  "fonts/LiberationSans-Bold.ttf",
];

self.addEventListener("install", (event) => {
  event.waitUntil(caches.open(CACHE).then((cache) => cache.addAll(SHELL)));
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(self.clients.claim());
});

self.addEventListener("fetch", (event) => {
  const { request } = event;
  if (request.method !== "GET" || new URL(request.url).origin !== self.location.origin) return;
  event.respondWith(
    fetch(request)
      .then((response) => {
        if (response.ok) {
          const copy = response.clone();
          caches.open(CACHE).then((cache) => cache.put(request, copy));
        }
        return response;
      })
      .catch(() => caches.match(request, { ignoreSearch: true })),
  );
});
