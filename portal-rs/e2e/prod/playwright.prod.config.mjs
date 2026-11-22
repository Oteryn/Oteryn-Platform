import { defineConfig } from '@playwright/test';

// Playwright otherwise captures a failure-time accessibility snapshot.
process.env.PLAYWRIGHT_NO_COPY_PROMPT = '1';

export default defineConfig({
  testDir: '.',
  testMatch: 'monitor.spec.mjs',
  workers: 1,
  retries: 0,
  timeout: 360_000,
  outputDir: process.env.MONITOR_OUTPUT_DIR || 'test-results',
  reporter: [['list', { printSteps: true }]],
  use: {
    browserName: 'chromium',
    baseURL: process.env.MONITOR_BASE_URL || 'https://oteryn.com',
    userAgent: 'OterynSyntheticMonitor/1.0 OterynE2E-32bec7675dad93274ee343bf',
    viewport: { width: 1440, height: 900 },
    serviceWorkers: 'block',
    trace: 'off',
    video: 'off',
    screenshot: 'off',
    launchOptions: {
      executablePath: process.env.CHROMIUM_PATH,
      args: ['--dns-prefetch-disable', '--disable-background-networking'],
    },
  },
});
