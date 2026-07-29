import { defineConfig } from '@playwright/test';

const baseURL = process.env.ACCEPTANCE_BASE_URL ?? 'http://127.0.0.1:8080';
const outputDir = process.env.ACCEPTANCE_OUTPUT_DIR ?? '../../artifacts/acceptance/community-data-test-results';
const communityDataSpec = '**/community-data-acceptance.spec.mjs';

export default defineConfig({
  testDir: './tests',
  outputDir,
  fullyParallel: false,
  forbidOnly: Boolean(process.env.CI),
  retries: 0,
  workers: 1,
  timeout: 240_000,
  reporter: [
    ['line'],
    ['html', { outputFolder: '../../artifacts/acceptance/community-data-html-report', open: 'never' }],
    ['junit', { outputFile: '../../artifacts/acceptance/community-data-junit.xml', includeProjectInTestName: true }],
  ],
  use: {
    baseURL,
    actionTimeout: 15_000,
    navigationTimeout: 30_000,
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    video: 'off',
  },
  projects: [
    {
      name: 'community-data-chromium-desktop',
      testMatch: communityDataSpec,
      use: { browserName: 'chromium', viewport: { width: 1440, height: 1000 } },
    },
    {
      name: 'community-data-chromium-tablet',
      testMatch: communityDataSpec,
      use: { browserName: 'chromium', viewport: { width: 820, height: 1180 }, hasTouch: true },
    },
    {
      name: 'community-data-chromium-mobile',
      testMatch: communityDataSpec,
      use: { browserName: 'chromium', viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true },
    },
  ],
  expect: { timeout: 12_000 },
});
