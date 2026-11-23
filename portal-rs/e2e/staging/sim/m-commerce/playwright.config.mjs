import { defineConfig } from '@playwright/test';
import { origin, proxy } from './helpers.mjs';

export default defineConfig({
  testDir: '.',
  outputDir: './test-results',
  reporter: 'list',
  workers: 2,
  retries: 0,
  timeout: 9 * 60_000,
  use: {
    baseURL: origin,
    browserName: 'chromium',
    launchOptions: { executablePath: '/usr/bin/chromium' },
    proxy,
    serviceWorkers: 'block',
    trace: 'off',
    screenshot: 'off',
    video: 'off',
  },
  projects: ['pl', 'en'].flatMap(lang =>
    [1440, 390].map(width => ({
      name: `${lang}-${width}`,
      metadata: { lang },
      use: {
        locale: lang === 'pl' ? 'pl-PL' : 'en-GB',
        viewport: { width, height: width === 390 ? 844 : 900 },
        hasTouch: width === 390,
        isMobile: width === 390,
      },
    })),
  ),
});
