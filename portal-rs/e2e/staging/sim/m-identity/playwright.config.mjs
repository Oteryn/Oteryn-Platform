import { defineConfig } from '@playwright/test';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const proxy = process.env.HTTPS_PROXY || process.env.https_proxy;
const id = process.env.STAGING_CF_ACCESS_CLIENT_ID;
const secret = process.env.STAGING_CF_ACCESS_CLIENT_SECRET;
if (!id || !secret) throw new Error('The staging Cloudflare Access environment variables are required.');

export default defineConfig({
  testDir: '.',
  timeout: 390_000,
  expect: { timeout: 8_000 },
  workers: 2,
  retries: 0,
  reporter: 'list',
  outputDir: join(tmpdir(), 'oteryn-m-identity-results'),
  use: {
    baseURL: 'https://staging.oteryn.com',
    serviceWorkers: 'block',
    proxy: proxy ? { server: proxy } : undefined,
    ignoreHTTPSErrors: Boolean(proxy),
    browserName: 'chromium',
    launchOptions: { executablePath: '/usr/bin/chromium' },
    navigationTimeout: 20_000,
    actionTimeout: 12_000,
    trace: 'off',
    screenshot: 'off',
    video: 'off',
  },
  projects: ['pl', 'en'].flatMap(locale => [1440, 390].map(width => ({
    name: `${locale}-${width}`,
    metadata: { locale },
    use: { locale, viewport: { width, height: 900 } },
  }))),
});
