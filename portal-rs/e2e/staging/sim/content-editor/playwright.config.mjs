import { defineConfig } from '@playwright/test';
import { chromiumOptions, staging } from './helpers.mjs';

export default defineConfig({
  testDir: '.',
  globalSetup: './helpers.mjs',
  outputDir: '/tmp/oteryn-content-editor-results',
  reporter: 'line',
  timeout: 660_000,
  expect: { timeout: 10_000 },
  workers: 2,
  retries: 0,
  use: {
    baseURL: staging,
    browserName: 'chromium',
    launchOptions: chromiumOptions(),
    trace: 'off',
    screenshot: 'off',
    video: 'off',
    serviceWorkers: 'block',
    actionTimeout: 12_000,
    navigationTimeout: 90_000,
  },
  projects: ['pl', 'en'].flatMap(locale => [1440, 390].map(width => ({
    name: `${locale}-${width}`,
    metadata: { locale },
    use: {
      locale: locale === 'pl' ? 'pl-PL' : 'en-US',
      viewport: { width, height: width === 390 ? 844 : 900 },
      isMobile: width === 390,
      hasTouch: width === 390,
    },
  }))),
});
