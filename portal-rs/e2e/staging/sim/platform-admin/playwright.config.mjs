import { randomUUID } from 'node:crypto';
import { defineConfig } from '@playwright/test';

process.env.OTERYN_SIM_RUN_ID ??= randomUUID();
const proxy = process.env.HTTPS_PROXY;
export default defineConfig({
  testDir: '.',
  outputDir: './test-results',
  reporter: 'list',
  workers: 2,
  retries: 0,
  timeout: 420_000,
  expect: { timeout: 10_000 },
  use: {
    baseURL: 'https://staging.oteryn.com',
    browserName: 'chromium',
    launchOptions: { executablePath: '/usr/bin/chromium' },
    proxy: proxy ? { server: proxy } : undefined,
    // The environment proxy CA is trusted by Node, which makes the credentialed requests.
    // Chromium may make a native request on a redirect without the service headers.
    ignoreHTTPSErrors: Boolean(proxy),
    serviceWorkers: 'block',
    trace: 'off',
    screenshot: 'off',
    video: 'off',
    actionTimeout: 15_000,
    navigationTimeout: 25_000,
  },
  projects: ['pl', 'en'].flatMap(lang => [1440, 390].map(width => ({
    name: lang + '-' + width,
    metadata: { lang },
    use: { locale: lang, viewport: { width, height: width === 390 ? 844 : 900 } },
  }))),
});
