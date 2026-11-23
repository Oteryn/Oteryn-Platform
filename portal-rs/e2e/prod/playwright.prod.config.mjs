import { defineConfig } from '@playwright/test';

if (!process.env.MONITOR_RUN_DIRECTORY || process.env.PLAYWRIGHT_NO_COPY_PROMPT !== '1') {
  throw new Error('Start the production monitor with node prod/run.mjs.');
}

const browserEnv = { ...process.env };
for (const key of ['MONITOR_EMAIL', 'MONITOR_PASSWORD', 'MONITOR_TOTP_SECRET',
  'MONITOR_SEEDED_PRIVACY_EMAIL', 'MONITOR_SEEDED_PRIVACY_PASSWORD']) {
  delete browserEnv[key];
}

export default defineConfig({
  testDir: '.',
  testMatch: 'monitor.spec.mjs',
  workers: 1,
  retries: 0,
  timeout: 360_000,
  outputDir: process.env.MONITOR_OUTPUT_DIR || 'test-results',
  reporter: 'dot',
  projects: [
    { name: 'pl-1440', metadata: { lang: 'pl', checkPublicEndpoints: true },
      use: { viewport: { width: 1440, height: 900 }, locale: 'pl-PL' } },
    { name: 'en-390', metadata: { lang: 'en', checkPublicEndpoints: false },
      use: { viewport: { width: 390, height: 844 }, locale: 'en-US', isMobile: true, hasTouch: true } },
  ],
  use: {
    browserName: 'chromium',
    baseURL: process.env.MONITOR_BASE_URL || 'https://oteryn.com',
    userAgent: 'OterynSyntheticMonitor/1.0 OterynE2E-32bec7675dad93274ee343bf',
    serviceWorkers: 'block',
    bypassCSP: false,
    trace: 'off',
    video: 'off',
    screenshot: 'off',
    launchOptions: {
      env: browserEnv,
      executablePath: process.env.CHROMIUM_PATH,
      args: ['--dns-prefetch-disable', '--disable-background-networking'],
    },
  },
});
