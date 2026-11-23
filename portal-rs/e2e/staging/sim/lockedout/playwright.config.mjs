import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { defineConfig } from '@playwright/test';

// Disable Playwright's automatic error-context snapshots on credential pages.
process.env.PLAYWRIGHT_NO_COPY_PROMPT = '1';

const id = process.env.STAGING_CF_ACCESS_CLIENT_ID;
const secret = process.env.STAGING_CF_ACCESS_CLIENT_SECRET;
if (!id || !secret) throw new Error('Missing staging Cloudflare Access credentials');

const proxy = process.env.HTTPS_PROXY || process.env.https_proxy;
const proxyPin = process.env.STAGING_CHROMIUM_TLS_SPKI;
if (proxyPin && !/^[A-Za-z0-9+/]{43}=$/.test(proxyPin)) {
  throw new Error('Expected a base64 SHA-256 public-key pin');
}

export default defineConfig({
  testDir: '.',
  testMatch: '*.spec.mjs',
  outputDir: mkdtempSync(join(tmpdir(), 'oteryn-lockedout-')),
  reporter: [['line']],
  workers: 2,
  retries: 0,
  timeout: 480_000,
  use: {
    baseURL: 'https://staging.oteryn.com',
    ...(proxy && { proxy: { server: proxy, bypass: process.env.NO_PROXY || process.env.no_proxy } }),
    launchOptions: {
      executablePath: '/usr/bin/chromium',
      ...(proxyPin && { args: [`--ignore-certificate-errors-spki-list=${proxyPin}`] }),
    },
    serviceWorkers: 'block',
    actionTimeout: 15_000,
    navigationTimeout: 30_000,
    trace: 'off',
    screenshot: 'off',
    video: 'off',
  },
  projects: ['pl', 'en'].flatMap(lang => [1440, 390].map(width => ({
    name: `${lang}-${width}`,
    metadata: { lang },
    use: {
      locale: lang === 'pl' ? 'pl-PL' : 'en-GB',
      viewport: { width, height: width === 390 ? 844 : 1000 },
      isMobile: width === 390,
      hasTouch: width === 390,
    },
  }))),
});
