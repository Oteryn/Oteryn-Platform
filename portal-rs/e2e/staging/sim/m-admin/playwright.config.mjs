import { defineConfig } from '@playwright/test';

const edgeProxy = process.env.HTTPS_PROXY ?? process.env.https_proxy;
const localeHeaders = { pl: 'pl-PL,pl;q=0.9', en: 'en-US,en;q=0.9' };
const projects = Object.keys(localeHeaders).flatMap((language) =>
  [1440, 390].map((width) => ({
    name: `${language}-${width}`,
    metadata: { language, acceptLanguage: localeHeaders[language] },
    use: {
      locale: language === 'pl' ? 'pl-PL' : 'en-US',
      viewport: { width, height: width === 390 ? 844 : 900 },
      isMobile: width === 390,
      hasTouch: width === 390,
      deviceScaleFactor: width === 390 ? 2 : 1,
    },
  })),
);

export default defineConfig({
  testDir: '.',
  testMatch: '*.spec.mjs',
  outputDir: '/tmp/oteryn-m-admin-results',
  reporter: 'list',
  workers: 2,
  retries: 0,
  timeout: 720_000,
  expect: { timeout: 10_000 },
  projects,
  use: {
    browserName: 'chromium',
    launchOptions: { executablePath: '/usr/bin/chromium' },
    ...(edgeProxy ? { proxy: { server: edgeProxy } } : {}),
    actionTimeout: 15_000,
    navigationTimeout: 20_000,
    trace: 'off',
    screenshot: 'off',
    video: 'off',
  },
});
