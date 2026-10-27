import { defineConfig } from '@playwright/test'

export default defineConfig({
  testDir: 'tests/e2e',
  timeout: 90_000,
  expect: { timeout: 15_000 },
  fullyParallel: false,
  workers: 1,
  retries: 0,
  reporter: 'list',
  use: { baseURL: 'http://127.0.0.1:43129', viewport: { width: 1280, height: 800 } },
  webServer: {
    command: 'npm run build && npx vite preview --host 127.0.0.1 --port 43129 --strictPort',
    url: 'http://127.0.0.1:43129',
    reuseExistingServer: false,
    timeout: 120_000,
  },
})
