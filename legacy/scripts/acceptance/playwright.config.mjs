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
  '**/portal-487-strictness-acceptance.spec.mjs',
  '**/accessibility-critical.spec.mjs',
  '**/soak-public.spec.mjs',
  '**/downloads-public-portability.spec.mjs',
  '**/error-state-acceptance.spec.mjs',
];
const specializedLifecycleIgnore = [
  '**/downloads-lifecycle-acceptance.spec.mjs',
  '**/events-public-acceptance.spec.mjs',
  '**/events-admin-acceptance.spec.mjs',
  '**/announcements-public-acceptance.spec.mjs',
  '**/announcements-admin-acceptance.spec.mjs',
  '**/support-legal-acceptance.spec.mjs',
  '**/editorial-media-acceptance.spec.mjs',
  '**/wiki-reconciliation-acceptance.spec.mjs',
];
const chromiumPrimaryIgnore = process.env.ACCEPTANCE_PROFILE === 'full'
  ? [...primaryIgnore, ...specializedLifecycleIgnore]
  : primaryIgnore;
const forcedZeroRetryProfiles = new Set(['critical', 'full', 'soak']);
const configuredRetries = process.env.ACCEPTANCE_ZERO_RETRIES === '1'
  || forcedZeroRetryProfiles.has(process.env.ACCEPTANCE_PROFILE ?? '')
  ? 0
  : process.env.CI ? 1 : 0;

const portabilityMatches = [
  '**/portability-critical.spec.mjs',
  '**/public-localization.spec.mjs',
  '**/public-wiki*.spec.mjs',
  '**/admin-wiki*.spec.mjs',
  '**/editorial-media-acceptance.spec.mjs',
  '**/public-game-catalog-acceptance.spec.mjs',
  '**/homepage-navigation-seo.spec.mjs',
  '**/community-data-acceptance.spec.mjs',
  '**/support-moderation-acceptance.spec.mjs',
];
const portabilityGrepInvert = /@portal-community-stress/u;

const responsiveMatches = [
  '**/responsive-critical.spec.mjs',
  '**/public-localization.spec.mjs',
  '**/public-wiki*.spec.mjs',
  '**/admin-wiki*.spec.mjs',
  '**/public-game-catalog-acceptance.spec.mjs',
  '**/admin-game-catalog-acceptance.spec.mjs',
  '**/homepage-navigation-seo.spec.mjs',
  '**/payment-foundation-acceptance.spec.mjs',
];

const accessibilityMatches = [
  '**/accessibility-critical.spec.mjs',
  '**/admin-wiki-editorial-media.spec.mjs',
  '**/public-game-catalog-acceptance.spec.mjs',
  '**/admin-game-catalog-acceptance.spec.mjs',
  '**/homepage-navigation-seo.spec.mjs',
];

const reporter = [
  ['line'],
  ['html', { outputFolder: '../../artifacts/acceptance/html-report', open: 'never' }],
  ['junit', { outputFile: '../../artifacts/acceptance/junit.xml', includeProjectInTestName: true }],
];

// Deep validation uploads only artifacts/deep after every terminal outcome. Mirror
// sanitized reporter output there so a fail-fast parent shell cannot discard the
// first actionable browser failure. Raw traces, screenshots and videos remain off
// because authenticated flows can contain cookies, reset URLs and enrollment data.
if (process.env.VALIDATION_SHA) {
  const runSuffix = (process.env.ACCEPTANCE_RUN_SUFFIX ?? 'unsuffixed')
    .replace(/[^a-zA-Z0-9._-]/g, '-');
  const deepReporterRoot = `../../artifacts/deep/playwright/${runSuffix}`;
  reporter.push(
    ['html', { outputFolder: `${deepReporterRoot}/html-report`, open: 'never' }],
    ['junit', { outputFile: `${deepReporterRoot}/junit.xml`, includeProjectInTestName: true }],
  );
}

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
  reporter,
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
      testIgnore: chromiumPrimaryIgnore,
      use: {
        browserName: 'chromium',
        viewport: desktopViewport,
      },
    },
    {
      name: 'portability-chromium',
      testMatch: portabilityMatches,
      grepInvert: portabilityGrepInvert,
      use: {
        browserName: 'chromium',
        viewport: desktopViewport,
      },
    },
    {
      name: 'portability-firefox',
      testMatch: portabilityMatches,
      grepInvert: portabilityGrepInvert,
      use: {
        browserName: 'firefox',
        viewport: desktopViewport,
      },
    },
    {
      name: 'portability-webkit',
      testMatch: portabilityMatches,
      grepInvert: portabilityGrepInvert,
      use: {
        browserName: 'webkit',
        viewport: desktopViewport,
      },
    },
    {
      name: 'downloads-portability-firefox',
      testMatch: '**/downloads-public-portability.spec.mjs',
      use: {
        browserName: 'firefox',
        viewport: desktopViewport,
      },
    },
    {
      name: 'downloads-portability-webkit',
      testMatch: '**/downloads-public-portability.spec.mjs',
      use: {
        browserName: 'webkit',
        viewport: desktopViewport,
      },
    },
    {
      name: 'responsive-desktop',
      testMatch: responsiveMatches,
      use: {
        browserName: 'chromium',
        viewport: desktopViewport,
      },
    },
    {
      name: 'responsive-tablet',
      testMatch: responsiveMatches,
      use: {
        browserName: 'chromium',
        viewport: tabletViewport,
        hasTouch: true,
      },
    },
    {
      name: 'responsive-mobile',
      testMatch: responsiveMatches,
      use: {
        browserName: 'chromium',
        viewport: mobileViewport,
        hasTouch: true,
        isMobile: true,
      },
    },
    {
      name: 'resilience-chromium',
      testMatch: [
        '**/resilience-critical.spec.mjs',
        '**/portal-487-strictness-acceptance.spec.mjs',
      ],
      use: {
        browserName: 'chromium',
        viewport: desktopViewport,
      },
    },
    {
      name: 'error-states-chromium-desktop',
      testMatch: '**/error-state-acceptance.spec.mjs',
      use: {
        browserName: 'chromium',
        viewport: desktopViewport,
      },
    },
    {
      name: 'error-states-chromium-tablet',
      testMatch: '**/error-state-acceptance.spec.mjs',
      use: {
        browserName: 'chromium',
        viewport: tabletViewport,
        hasTouch: true,
      },
    },
    {
      name: 'error-states-chromium-mobile',
      testMatch: '**/error-state-acceptance.spec.mjs',
      use: {
        browserName: 'chromium',
        viewport: mobileViewport,
        hasTouch: true,
        isMobile: true,
      },
    },
    {
      name: 'accessibility-chromium',
      testMatch: accessibilityMatches,
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
