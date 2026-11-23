import { createHash, X509Certificate } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { defineConfig } from '@playwright/test';

const accessId = process.env.STAGING_CF_ACCESS_CLIENT_ID;
const accessSecret = process.env.STAGING_CF_ACCESS_CLIENT_SECRET;
if (!accessId || !accessSecret) {
  throw new Error('Set STAGING_CF_ACCESS_CLIENT_ID and STAGING_CF_ACCESS_CLIENT_SECRET.');
}

const proxyServer = process.env.HTTPS_PROXY || process.env.https_proxy;
const proxyCa = proxyServer && process.env.CODEX_PROXY_CERT;
const browserArgs = proxyCa ? [
  `--ignore-certificate-errors-spki-list=${createHash('sha256')
    .update(new X509Certificate(readFileSync(proxyCa)).publicKey.export({ type: 'spki', format: 'der' }))
    .digest('base64')}`,
] : [];

export default defineConfig({
  testDir: '.',
  outputDir: '/tmp/oteryn-support-flow-results',
  workers: 2,
  retries: 0,
  timeout: 480_000,
  reporter: 'list',
  use: {
    baseURL: 'https://staging.oteryn.com',
    browserName: 'chromium',
    launchOptions: { executablePath: '/usr/bin/chromium', args: browserArgs },
    proxy: proxyServer ? { server: proxyServer } : undefined,
    serviceWorkers: 'block',
    trace: 'off',
    screenshot: 'off',
    video: 'off',
  },
  projects: ['pl', 'en'].flatMap(language =>
    [1440, 390].map(width => ({
      name: `${language}-${width}`,
      metadata: { language },
      use: {
        locale: language === 'pl' ? 'pl-PL' : 'en-US',
        viewport: { width, height: width === 390 ? 844 : 900 },
      },
    })),
  ),
});
