import { defineConfig } from '@playwright/test';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const clientId = process.env.STAGING_CF_ACCESS_CLIENT_ID;
const clientSecret = process.env.STAGING_CF_ACCESS_CLIENT_SECRET;
if (!clientId || !clientSecret) throw new Error('Staging Cloudflare Access credentials are required');

const proxyUrl = process.env.HTTPS_PROXY || process.env.https_proxy;
const proxy = proxyUrl ? { server: new URL(proxyUrl).origin } : undefined;

export default defineConfig({
  testDir: '.',
  outputDir: join(tmpdir(), 'oteryn-newcomer-playwright'),
  workers: 2,
  retries: 0,
  timeout: 480_000,
  expect: { timeout: 8_000 },
  reporter: 'list',
  use: {
    baseURL: 'https://staging.oteryn.com',
    browserName: 'chromium',
    launchOptions: { executablePath: '/usr/bin/chromium' },
    proxy,
    // Chromium does not pick up the execution environment's proxy CA.
    ignoreHTTPSErrors: Boolean(proxy && process.env.CODEX_PROXY_CERT),
    serviceWorkers: 'block',
    trace: 'off',
    video: 'off',
    screenshot: 'only-on-failure',
    navigationTimeout: 20_000,
    actionTimeout: 10_000,
  },
  projects: ['pl', 'en'].flatMap(locale => [1440, 390].map(width => ({
    name: `${locale}-${width}`,
    metadata: { lang: locale },
    use: { locale, viewport: { width, height: width === 390 ? 844 : 900 }, hasTouch: width === 390 },
  }))),
});
