import { defineConfig } from '@playwright/test';
import { createHash, X509Certificate } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { accessHeaders, stagingOrigin, stagingProxy } from './transport.mjs';

accessHeaders();
process.env.OTERYN_BLACKBOX_RUN_ID ||= `${Date.now()}-${process.pid}`;

// Chromium does not inherit Node's extra CA. Trust the execution proxy's
// certificate by public key when its CA is explicitly provided.
const proxyCA = stagingProxy && process.env.CODEX_PROXY_CERT;
const proxyPin = proxyCA && createHash('sha256')
  .update(new X509Certificate(readFileSync(proxyCA)).publicKey.export({ type: 'spki', format: 'der' }))
  .digest('base64');

export default defineConfig({
  testDir: '.',
  testMatch: '*.spec.mjs',
  globalSetup: './setup.mjs',
  outputDir: join(tmpdir(), `oteryn-staging-blackbox-${process.env.OTERYN_BLACKBOX_RUN_ID}`),
  timeout: 420_000,
  expect: { timeout: 8_000 },
  workers: 2,
  retries: 0,
  reporter: './reporter.mjs',
  use: {
    baseURL: stagingOrigin,
    browserName: 'chromium',
    launchOptions: {
      executablePath: '/usr/bin/chromium',
      args: proxyPin ? [`--ignore-certificate-errors-spki-list=${proxyPin}`] : [],
    },
    serviceWorkers: 'block',
    actionTimeout: 15_000,
    navigationTimeout: 25_000,
    trace: 'off',
    video: 'off',
    screenshot: 'off',
  },
  projects: ['pl', 'en'].flatMap(locale => [1440, 390].map(width => ({
    name: `${locale}-${width}`,
    use: { locale, viewport: { width, height: 900 } },
    metadata: { locale },
  }))),
});
