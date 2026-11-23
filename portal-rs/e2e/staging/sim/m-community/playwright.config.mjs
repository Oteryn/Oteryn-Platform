import { defineConfig } from '@playwright/test';

const proxy = process.env.HTTPS_PROXY || process.env.https_proxy;

export default defineConfig({
  testDir: '.',
  outputDir: './test-results',
  fullyParallel: true,
  workers: 2,
  retries: 0,
  timeout: 900_000,
  reporter: 'list',
  use: {
    browserName: 'chromium',
    baseURL: 'https://staging.oteryn.com',
    launchOptions: { executablePath: '/usr/bin/chromium' },
    ...(proxy ? { proxy: { server: proxy } } : {}),
    ignoreHTTPSErrors: Boolean(proxy && process.env.CODEX_PROXY_CERT),
    serviceWorkers: 'block',
    trace: 'off',
    screenshot: 'off',
    video: 'off',
    actionTimeout: 12_000,
    navigationTimeout: 20_000,
  },
  projects: ['pl', 'en'].flatMap(lang => [1440, 390].map(width => ({
    name: `${lang}-${width}`,
    metadata: { lang },
    use: {
      locale: lang === 'pl' ? 'pl-PL' : 'en-GB',
      viewport: { width, height: width === 390 ? 844 : 900 },
    },
  }))),
});
