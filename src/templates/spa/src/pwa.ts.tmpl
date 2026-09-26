/**
 * Register the service worker - in a production build, and only there.
 *
 * In development it would serve yesterday's modules underneath Vite's hot
 * reload, and every edit would look like it did nothing. The build id in the
 * address is what makes each build a new worker with a cache of its own
 * (`public/sw.js`).
 */
export function registerServiceWorker(): boolean {
  if (!import.meta.env.PROD || !('serviceWorker' in navigator)) {
    return false
  }
  window.addEventListener('load', () => {
    void navigator.serviceWorker.register(`/sw.js?build=${__BUILD_ID__}`)
  })
  return true
}
