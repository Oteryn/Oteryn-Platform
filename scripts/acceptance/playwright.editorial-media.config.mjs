import { defineConfig } from '@playwright/test';

const baseURL = process.env.ACCEPTANCE_BASE_URL ?? 'http://127.0.0.1:8080';
const outputDir = process.env.ACCEPTANCE_OUTPUT_DIR ?? '../../artifacts/acceptance/editorial-media-test-results';
const editorialMediaSpecs = [
  '**/editorial-media-acceptance.spec.mjs',
  '**/editorial-media-strictness-acceptance.spec.mjs',
];

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
    ['html', { outputFolder: '../../artifacts/acceptance/editorial-media-html-report', open: 'never' }],
    ['junit', { outputFile: '../../artifacts/acceptance/editorial-media-junit.xml', includeProjectInTestName: true }],
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
      name: 'editorial-media-chromium-desktop',
      testMatch: editorialMediaSpecs,
      grep: /@portal-editorial-media/u,
      use: { browserName: 'chromium', viewport: { width: 1440, height: 1000 } },
    },
    {
      name: 'editorial-media-chromium-tablet',
      testMatch: editorialMediaSpecs,
      grep: /@portal-editorial-media/u,
      use: { browserName: 'chromium', viewport: { width: 820, height: 1180 }, hasTouch: true },
    },
    {
      name: 'editorial-media-chromium-mobile',
      testMatch: editorialMediaSpecs,
      grep: /@portal-editorial-media/u,
      use: { browserName: 'chromium', viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true },
    },
  ],
  expect: { timeout: 10_000 },
});
