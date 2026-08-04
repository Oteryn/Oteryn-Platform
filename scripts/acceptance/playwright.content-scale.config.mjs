import { defineConfig } from '@playwright/test';

const baseURL = process.env.ACCEPTANCE_BASE_URL ?? 'http://127.0.0.1:8080';
const outputDir = process.env.ACCEPTANCE_OUTPUT_DIR ?? '../../artifacts/acceptance/content-scale-test-results';
const contentScaleSpecs = [
  '**/content-scale-acceptance.spec.mjs',
  '**/content-scale-events-acceptance.spec.mjs',
  '**/content-scale-media-acceptance.spec.mjs',
  '**/content-scale-wiki-acceptance.spec.mjs',
];
const deepValidation = Boolean(process.env.VALIDATION_SHA);
const reporter = [
  ['line'],
  ['html', { outputFolder: '../../artifacts/acceptance/content-scale-html-report', open: 'never' }],
  ['junit', { outputFile: '../../artifacts/acceptance/content-scale-junit.xml', includeProjectInTestName: true }],
];

// The deep workflow uploads only artifacts/deep after every terminal outcome.
// Mirror sanitized JUnit there before a fail-fast shell can exit. Raw traces and
// screenshots are disabled for the deep run because authenticated browser state
// may contain session material.
if (deepValidation) {
  reporter.push(
    ['junit', { outputFile: '../../artifacts/deep/playwright/content-scale/junit.xml', includeProjectInTestName: true }],
  );
}

export default defineConfig({
  testDir: './tests',
  outputDir,
  fullyParallel: false,
  forbidOnly: Boolean(process.env.CI),
  retries: 0,
  workers: 1,
  timeout: 180_000,
  reporter,
  use: {
    baseURL,
    actionTimeout: 15_000,
    navigationTimeout: 30_000,
    trace: deepValidation ? 'off' : 'retain-on-failure',
    screenshot: deepValidation ? 'off' : 'only-on-failure',
    video: 'off',
  },
  projects: [
    {
      name: 'content-scale-chromium-desktop',
      testMatch: contentScaleSpecs,
      use: { browserName: 'chromium', viewport: { width: 1440, height: 1000 } },
    },
    {
      name: 'content-scale-chromium-tablet',
      testMatch: contentScaleSpecs,
      use: { browserName: 'chromium', viewport: { width: 820, height: 1180 }, hasTouch: true },
    },
    {
      name: 'content-scale-chromium-mobile',
      testMatch: contentScaleSpecs,
      use: { browserName: 'chromium', viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true },
    },
  ],
  expect: { timeout: 12_000 },
});
