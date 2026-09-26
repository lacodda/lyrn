// The service worker: the application still opens when the network does not.
//
// Network first, the cache as the fallback. A deploy then shows on the next
// load, not the one after - which is what serving from the cache first would
// give - and the cached copy is only what appears when there is no network at
// all. The API is never cached here: yesterday's data shown as today's is
// worse than an honest error.
//
// Each build registers this script under its own `?build=` (src/pwa.ts), so a
// new build is a new worker with a new cache, and the old cache - with every
// hashed file the old build asked for - is dropped when it takes over.

const CACHE = `shell-${new URL(self.location.href).searchParams.get('build') ?? 'dev'}`

self.addEventListener('install', (event) => {
  // The shell is taken now rather than on the first visit to `/`: offline,
  // any address falls back to it, including ones never opened online.
  event.waitUntil(caches.open(CACHE).then((cache) => cache.add('/')))
  // Take over as soon as installed: a worker left waiting for every tab to
  // close would keep serving the old build's fallback.
  void self.skipWaiting()
})

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((key) => key !== CACHE).map((key) => caches.delete(key))))
      .then(() => self.clients.claim()),
  )
})

self.addEventListener('fetch', (event) => {
  const request = event.request
  const url = new URL(request.url)
  if (request.method !== 'GET' || url.origin !== self.location.origin || url.pathname.startsWith('/api/')) {
    return
  }

  event.respondWith(
    fetch(request)
      .then((response) => {
        if (response.ok) {
          const copy = response.clone()
          void caches.open(CACHE).then((cache) => cache.put(request, copy))
        }
        return response
      })
      .catch(async () => {
        const cached = await caches.match(request)
        if (cached) return cached
        // A screen at any address is the same page: offline, every
        // navigation falls back to the shell.
        if (request.mode === 'navigate') {
          const shell = await caches.match('/')
          if (shell) return shell
        }
        return Response.error()
      }),
  )
})
