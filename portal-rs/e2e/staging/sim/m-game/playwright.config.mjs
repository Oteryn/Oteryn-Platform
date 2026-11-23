import { defineConfig } from '@playwright/test';

const pinnedProxyKey = process.env.STAGING_CHROMIUM_PROXY_SPKI;
const proxy = process.env.HTTPS_PROXY || process.env.https_proxy;
const {
  STAGING_CF_ACCESS_CLIENT_ID: accessId,
  STAGING_CF_ACCESS_CLIENT_SECRET: accessSecret,
  ...browserEnv
} = process.env;

if (!accessId || !accessSecret) {
  throw new Error('The staging Cloudflare Access credentials are required.');
}

export default defineConfig({
  testDir: '.',
  outputDir: './test-results',
  reporter: 'line',
  retries: 0,
  workers: 2,
  timeout: 360_000,
  expect: { timeout: 8_000 },
  projects: ['pl', 'en'].flatMap(lang => [1440, 390].map(width => ({
    name: `${lang}-${width}`,
    metadata: { lang },
    use: {
      locale: lang === 'pl' ? 'pl-PL' : 'en-US',
      viewport: { width, height: width === 390 ? 844 : 900 },
      isMobile: width === 390,
      hasTouch: width === 390,
    },
  }))),
  use: {
    baseURL: 'https://staging.oteryn.com',
    browserName: 'chromium',
    proxy: proxy ? { server: proxy } : undefined,
    actionTimeout: 12_000,
    navigationTimeout: 20_000,
    serviceWorkers: 'block',
    trace: 'off',
    screenshot: 'off',
    video: 'off',
    launchOptions: {
      executablePath: '/usr/bin/chromium',
      env: browserEnv,
      args: [
        '--disable-background-networking',
        '--dns-prefetch-disable',
        ...(proxy && pinnedProxyKey ? [`--ignore-certificate-errors-spki-list=${pinnedProxyKey}`] : []),
      ],
    },
  },
});
