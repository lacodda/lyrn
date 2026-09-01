import { fileURLToPath, URL } from 'node:url'
import { defineConfig } from 'vitest/config'

export default defineConfig({
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
  test: {
    environment: 'jsdom',
    // Testing-library unmounts between tests only when it can see the test
    // hooks; without this every render stacks up in the same document and the
    // second `getByRole` finds two of everything.
    globals: true,
    include: ['src/**/*.test.{ts,tsx}'],
  },
})
