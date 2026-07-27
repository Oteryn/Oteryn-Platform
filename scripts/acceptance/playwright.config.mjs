import { defineConfig } from '@playwright/test';

const baseURL = process.env.ACCEPTANCE_BASE_URL ?? 'http://127.0.0.1:8080';
const outputDir = process.env.ACCEPTANCE_OUTPUT_DIR ?? '../../artifacts/acceptance/test-results';
const desktopViewport = { width: 1440, height: 1000 };
const tabletViewport = { width: 820, height: 1180 };
const mobileViewport = { width: 390, height: 844 };
const primaryIgnore = [
  '**/full-acceptance.spec.mjs',
  '**/portability-critical.spec.mjs',
  '**/responsive-critical.spec.mjs',
  '**/resilience-critical.spec.mjs',
  '**/accessibility-critical.spec.mjs',
  '**/soak-public.spec.mjs',
];
const portabilityTests = [
  '**/portability-critical.spec.mjs',
  '**/public-wiki*.spec.mjs',
  '**/admin-wiki*.spec.mjs',
  '**/homepage-navigation-seo.spec.mjs',
  '**/events-public-acceptance.spec.mjs',
];
const responsiveTests = [
  '**/responsive-critical.spec.mjs',
  '**/public-wiki*.spec.mjs',
  '**/admin-wiki*.spec.mjs',
  '**/homepage-navigation-seo.spec.mjs',
  '**/events-public-acceptance.spec.mjs',
  '**/events-admin-acceptance.spec.mjs',
];
const configuredRetries = process.env.ACCEPTANCE_ZERO_RETRIES === '1' ? 0 : process.env.CI ? 1 : 0;

export default defineConfig({
  testDir: './tests',
  // The original monolithic serial acceptance spec is retained as historical source
  // while the executable suite uses isolated, independently seeded scenarios.
  testIgnore: '**/full-acceptance.spec.mjs',
  outputDir,
  fullyParallel: false,
  forbidOnly: Boolean(process.env.CI),
  retries: configuredRetries,
  workers: 1,
  timeout: 120_000,
  reporter: [
    ['line'],
    ['html', { outputFolder: '../../artifacts/acceptance/html-report', open: 'never' }],
    ['junit', { outputFile: '../../artifacts/acceptance/junit.xml', includeProjectInTestName: true }],
  ],
  use: {
    baseURL,
    actionTimeout: 15_000,
    navigationTimeout: 30_000,
    // Raw Playwright traces and automatic failure screenshots can capture session
    // cookies, reset URLs, TOTP enrollment secrets or recovery codes. Secret-bearing
    // full, portability, responsive and accessibility flows therefore use sanitized
    // diagnostics. The non-secret smoke/soak paths may opt into bounded evidence.
    trace: 'off',
    screenshot: 'off',
    video: 'off',
  },
  projects: [
    {
      name: 'chromium-primary',
      testIgnore: primaryIgnore,
      use: {
        browserName: 'chromium',
        viewport: desktopViewport,
      },
    },
    {
      name: 'portability-chromium',
      testMatch: portabilityTests,
      use: {
        browserName: 'chromium',
        viewport: desktopViewport,
      },
    },
    {
      name: 'portability-firefox',
      testMatch: portabilityTests,
      use: {
        browserName: 'firefox',
        viewport: desktopViewport,
      },
    },
    {
      name: 'portability-webkit',
      testMatch: portabilityTests,
      use: {
        browserName: 'webkit',
        viewport: desktopViewport,
      },
    },
    {
      name: 'responsive-desktop',
      testMatch: responsiveTests,
      use: {
        browserName: 'chromium',
        viewport: desktopViewport,
      },
    },
    {
      name: 'responsive-tablet',
      testMatch: responsiveTests,
      use: {
        browserName: 'chromium',
        viewport: tabletViewport,
        hasTouch: true,
      },
    },
    {
      name: 'responsive-mobile',
      testMatch: responsiveTests,
      use: {
        browserName: 'chromium',
        viewport: mobileViewport,
        hasTouch: true,
        isMobile: true,
      },
    },
    {
      name: 'resilience-chromium',
      testMatch: '**/resilience-critical.spec.mjs',
      use: {
        browserName: 'chromium',
        viewport: desktopViewport,
      },
    },
    {
      name: 'accessibility-chromium',
      testMatch: ['**/accessibility-critical.spec.mjs', '**/admin-wiki-editorial-media.spec.mjs', '**/homepage-navigation-seo.spec.mjs'],
      use: {
        browserName: 'chromium',
        viewport: desktopViewport,
      },
    },
    {
      name: 'soak-chromium',
      testMatch: '**/soak-public.spec.mjs',
      use: {
        browserName: 'chromium',
        viewport: desktopViewport,
      },
    },
  ],
  expect: {
    timeout: 10_000,
  },
});
