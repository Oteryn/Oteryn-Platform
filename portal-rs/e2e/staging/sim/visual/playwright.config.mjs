import { defineConfig } from '@playwright/test';
import { origin, proxy } from './helpers.mjs';

// Snapshots on failure can include one-time links or authenticator material.
process.env.PLAYWRIGHT_NO_COPY_PROMPT = '1';

export default defineConfig({
  testDir: '.',
  outputDir: 'test-results',
  workers: 2,
  retries: 0,
  fullyParallel: true,
  timeout: 15 * 60_000,
  reporter: 'dot',
  use: {
    baseURL: origin,
    browserName: 'chromium',
    launchOptions: { executablePath: '/usr/bin/chromium' },
    proxy,
    trace: 'off',
    screenshot: 'off',
    video: 'off',
    serviceWorkers: 'block',
    ignoreHTTPSErrors: process.env.STAGING_IGNORE_HTTPS_ERRORS === '1',
    navigationTimeout: 20_000,
    actionTimeout: 10_000,
  },
  projects: ['en', 'pl'].flatMap(language => [1440, 390].map(width => ({
    name: `${language}-${width}`,
    metadata: { language },
    use: {
      locale: language === 'pl' ? 'pl-PL' : 'en-GB',
      viewport: { width, height: width === 390 ? 844 : 900 },
      hasTouch: width === 390,
      colorScheme: 'light',
    },
  }))),
});
