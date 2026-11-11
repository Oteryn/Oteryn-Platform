import { defineConfig } from '@playwright/test';

const baseURL = process.env.ACCEPTANCE_BASE_URL ?? 'http://127.0.0.1:8080';
const outputDir = process.env.ACCEPTANCE_OUTPUT_DIR ?? '../../artifacts/acceptance/support-legal-test-results';
const supportLegalSpec = '**/support-legal-acceptance.spec.mjs';

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
    ['html', { outputFolder: '../../artifacts/acceptance/support-legal-html-report', open: 'never' }],
    ['junit', { outputFile: '../../artifacts/acceptance/support-legal-junit.xml', includeProjectInTestName: true }],
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
      name: 'support-legal-chromium-desktop',
      testMatch: supportLegalSpec,
      grep: /@portal-support-legal/u,
      use: { browserName: 'chromium', viewport: { width: 1440, height: 1000 } },
    },
    {
      name: 'support-legal-chromium-tablet',
      testMatch: supportLegalSpec,
      grep: /@portal-support-legal/u,
      use: { browserName: 'chromium', viewport: { width: 820, height: 1180 }, hasTouch: true },
    },
    {
      name: 'support-legal-chromium-mobile',
      testMatch: supportLegalSpec,
      grep: /@portal-support-legal/u,
      use: { browserName: 'chromium', viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true },
    },
    {
      name: 'support-legal-firefox-desktop',
      testMatch: supportLegalSpec,
      grep: /@portal-support-legal-public/u,
      use: { browserName: 'firefox', viewport: { width: 1440, height: 1000 } },
    },
    {
      name: 'support-legal-webkit-desktop',
      testMatch: supportLegalSpec,
      grep: /@portal-support-legal-public/u,
      use: { browserName: 'webkit', viewport: { width: 1440, height: 1000 } },
    },
  ],
  expect: { timeout: 10_000 },
});
