import { defineConfig } from '@playwright/test';
import { proxy, staging } from './helpers.mjs';

export default defineConfig({
  testDir: '.',
  outputDir: './test-results',
  workers: 2,
  retries: 0,
  timeout: 390_000,
  reporter: 'list',
  use: {
    baseURL: staging.origin,
    browserName: 'chromium',
    launchOptions: { executablePath: '/usr/bin/chromium' },
    proxy,
    // Chromium does not trust the execution environment's HTTPS proxy CA.
    ignoreHTTPSErrors: Boolean(proxy),
    actionTimeout: 10_000,
    navigationTimeout: 20_000,
    trace: 'off',
    screenshot: 'off',
    video: 'off',
    serviceWorkers: 'block',
  },
  projects: ['pl', 'en'].flatMap(locale => [1440, 390].map(width => ({
    name: `${locale}-${width}`,
    metadata: { locale },
    use: { locale, viewport: { width, height: width === 390 ? 844 : 900 } },
  }))),
});
