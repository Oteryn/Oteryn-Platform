import { defineConfig } from '@playwright/test';

const proxyServer = process.env.HTTPS_PROXY || process.env.https_proxy;

export default defineConfig({
  testDir: '.',
  outputDir: './test-results',
  timeout: 720_000,
  expect: { timeout: 8_000 },
  fullyParallel: true,
  workers: 2,
  retries: 0,
  reporter: 'list',
  use: {
    baseURL: 'https://staging.oteryn.com',
    browserName: 'chromium',
    launchOptions: { executablePath: '/usr/bin/chromium' },
    proxy: proxyServer ? { server: proxyServer } : undefined,
    serviceWorkers: 'block',
    ignoreHTTPSErrors: Boolean(proxyServer),
    trace: 'off',
    screenshot: 'off',
    video: 'off',
    navigationTimeout: 25_000,
    actionTimeout: 15_000,
  },
  projects: ['pl', 'en'].flatMap(lang => [1440, 390].map(width => ({
    name: `${lang}-${width}`,
    metadata: { lang },
    use: { locale: lang, viewport: { width, height: width === 390 ? 844 : 960 } },
  }))),
});
