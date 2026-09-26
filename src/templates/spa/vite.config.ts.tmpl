import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { fileURLToPath, URL } from 'node:url'

export default defineConfig({
  plugins: [react(), tailwindcss()],
{{#pwa}}
  // A new id per build: the service worker is registered under it, so each
  // build is a new worker with a cache of its own (public/sw.js).
  define: {
    __BUILD_ID__: JSON.stringify(Date.now().toString(36)),
  },
{{/pwa}}
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
{{#auth}}
  // The session is a cookie, so the API has to be same-origin. In development
  // Vite forwards `/api` to the service - the line's service form listens on
  // 8080 - and in production the service serves this build itself.
  server: {
    proxy: { '/api': 'http://localhost:8080' },
  },
{{/auth}}
})
