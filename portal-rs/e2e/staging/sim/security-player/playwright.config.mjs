import { defineConfig } from '@playwright/test';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { createHash, X509Certificate } from 'node:crypto';
import { readFileSync } from 'node:fs';

const proxyCertificate = process.env.CODEX_PROXY_CERT;
if (proxyCertificate && !process.env.NODE_EXTRA_CA_CERTS) {
  // Test workers import this CA when Node starts.
  process.env.NODE_EXTRA_CA_CERTS = proxyCertificate;
}
const proxyPin = proxyCertificate && createHash('sha256').update(
  new X509Certificate(readFileSync(proxyCertificate)).publicKey.export({ type: 'spki', format: 'der' }),
).digest('base64');

export default defineConfig({
  testDir: '.',
  outputDir: join(tmpdir(), 'oteryn-staging-security-player'),
  timeout: 420_000,
  expect: { timeout: 10_000 },
  workers: 2,
  retries: 0,
  reporter: 'list',
  use: {
    browserName: 'chromium',
    launchOptions: {
      executablePath: '/usr/bin/chromium',
      args: proxyPin ? [`--ignore-certificate-errors-spki-list=${proxyPin}`] : [],
      env: Object.fromEntries(Object.entries(process.env).filter(([key]) =>
        !key.startsWith('STAGING_CF_ACCESS_'))),
    },
    trace: 'off',
    screenshot: 'off',
    video: 'off',
  },
  projects: ['pl', 'en'].flatMap(locale => [1440, 390].map(width => ({
    name: `${locale}-${width}`,
    metadata: { locale },
    use: { locale, viewport: { width, height: width === 390 ? 844 : 900 } },
  }))),
});
