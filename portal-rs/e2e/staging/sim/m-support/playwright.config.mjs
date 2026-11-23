import { defineConfig } from '@playwright/test';

const origin = 'https://staging.oteryn.com';
const proxyURL = process.env.HTTPS_PROXY || process.env.https_proxy;
const proxy = proxyURL ? { server: new URL(proxyURL).origin } : undefined;

export default defineConfig({
  testDir: '.',
  testMatch: '*.spec.mjs',
  outputDir: '/tmp/oteryn-sim-m-support-artifacts',
  reporter: 'dot',
  workers: 2,
  retries: 0,
  timeout: 390_000,
  expect: { timeout: 10_000 },
  use: {
    baseURL: origin,
    browserName: 'chromium',
    launchOptions: { executablePath: '/usr/bin/chromium' },
    proxy,
    serviceWorkers: 'block',
    ignoreHTTPSErrors: Boolean(proxy && process.env.CODEX_PROXY_CERT),
    trace: 'off',
    screenshot: 'off',
    video: 'off',
  },
  projects: ['pl', 'en'].flatMap(lang => [1440, 390].map(width => ({
    name: `${lang}-${width}`,
    metadata: { lang },
    use: {
      locale: lang,
      viewport: { width, height: width === 390 ? 844 : 900 },
      isMobile: width === 390,
      hasTouch: width === 390,
    },
  }))),
});
