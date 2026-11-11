import { defineConfig } from '@playwright/test';

const baseURL = process.env.ACCEPTANCE_BASE_URL ?? 'http://127.0.0.1:8080';
const outputDir = process.env.ACCEPTANCE_OUTPUT_DIR ?? '../../artifacts/acceptance/announcements-test-results';
const publicSpec = '**/announcements-public-acceptance.spec.mjs';
const completeSpecs = [publicSpec, '**/announcements-admin-acceptance.spec.mjs'];

export default defineConfig({
  testDir: './tests',
  outputDir,
  fullyParallel: false,
  forbidOnly: Boolean(process.env.CI),
  retries: 0,
  workers: 1,
  timeout: 180_000,
  reporter: [
    ['line'],
    ['html', { outputFolder: '../../artifacts/acceptance/announcements-html-report', open: 'never' }],
    ['junit', { outputFile: '../../artifacts/acceptance/announcements-junit.xml', includeProjectInTestName: true }],
  ],
  use: {
    baseURL,
    actionTimeout: 15_000,
    navigationTimeout: 30_000,
    trace: 'off',
    screenshot: 'off',
    video: 'off',
  },
  projects: [
    {
      name: 'announcements-chromium-desktop',
      testMatch: completeSpecs,
      use: { browserName: 'chromium', viewport: { width: 1440, height: 1000 } },
    },
    {
      name: 'announcements-chromium-tablet',
      testMatch: completeSpecs,
      use: { browserName: 'chromium', viewport: { width: 820, height: 1180 }, hasTouch: true },
    },
    {
      name: 'announcements-chromium-mobile',
      testMatch: completeSpecs,
      use: { browserName: 'chromium', viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true },
    },
    {
      name: 'announcements-firefox-desktop',
      testMatch: publicSpec,
      use: { browserName: 'firefox', viewport: { width: 1440, height: 1000 } },
    },
    {
      name: 'announcements-webkit-desktop',
      testMatch: publicSpec,
      use: { browserName: 'webkit', viewport: { width: 1440, height: 1000 } },
    },
  ],
  expect: { timeout: 10_000 },
});
