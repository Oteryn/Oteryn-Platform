import { defineConfig } from '@playwright/test';
import { createHash, X509Certificate } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const proxy = process.env.HTTPS_PROXY ? { server: process.env.HTTPS_PROXY } : undefined;
const args = [];
if (proxy && process.env.CODEX_PROXY_CERT) {
  const certificate = new X509Certificate(readFileSync(process.env.CODEX_PROXY_CERT));
  const spki = certificate.publicKey.export({ type: 'spki', format: 'der' });
  args.push(`--ignore-certificate-errors-spki-list=${createHash('sha256').update(spki).digest('base64')}`);
}

export default defineConfig({
  testDir: '.',
  outputDir: join(tmpdir(), 'oteryn-security-ops-results'),
  reporter: 'line',
  workers: 2,
  retries: 0,
  timeout: 600_000,
  use: {
    baseURL: 'https://staging.oteryn.com',
    actionTimeout: 12_000,
    navigationTimeout: 25_000,
    browserName: 'chromium',
    launchOptions: { executablePath: '/usr/bin/chromium', args },
    proxy,
    serviceWorkers: 'block',
    trace: 'off',
    screenshot: 'off',
    video: 'off',
  },
  projects: ['pl', 'en'].flatMap(language => [1440, 390].map(width => ({
    name: `${language}-${width}`,
    metadata: { language },
    use: {
      locale: language === 'pl' ? 'pl-PL' : 'en-GB',
      viewport: { width, height: width === 390 ? 844 : 900 },
    },
  }))),
});
