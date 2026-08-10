import { defineConfig } from '@playwright/test';

const baseURL = process.env.ACCEPTANCE_BASE_URL ?? 'http://127.0.0.1:8080';
const outputDir = process.env.ACCEPTANCE_OUTPUT_DIR ?? '../../artifacts/acceptance/wiki-reconciliation-test-results';
const wikiSpecs = [
  '**/wiki-reconciliation-acceptance.spec.mjs',
  '**/wiki-strictness-acceptance.spec.mjs',
];

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
    ['html', { outputFolder: '../../artifacts/acceptance/wiki-reconciliation-html-report', open: 'never' }],
    ['junit', { outputFile: '../../artifacts/acceptance/wiki-reconciliation-junit.xml', includeProjectInTestName: true }],
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
      name: 'wiki-reconciliation-chromium-desktop',
      testMatch: wikiSpecs,
      use: { browserName: 'chromium', viewport: { width: 1440, height: 1000 } },
    },
    {
      name: 'wiki-reconciliation-chromium-tablet',
      testMatch: wikiSpecs,
      use: { browserName: 'chromium', viewport: { width: 820, height: 1180 }, hasTouch: true },
    },
    {
      name: 'wiki-reconciliation-chromium-mobile',
      testMatch: wikiSpecs,
      use: { browserName: 'chromium', viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true },
    },
    {
      name: 'wiki-reconciliation-firefox-desktop',
      testMatch: wikiSpecs,
      use: { browserName: 'firefox', viewport: { width: 1440, height: 1000 } },
    },
    {
      name: 'wiki-reconciliation-webkit-desktop',
      testMatch: wikiSpecs,
      use: { browserName: 'webkit', viewport: { width: 1440, height: 1000 } },
    },
  ],
  expect: { timeout: 10_000 },
});
