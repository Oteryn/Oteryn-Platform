import { defineConfig } from '@playwright/test';
import fs from 'node:fs';
import dns from 'node:dns';
dns.setDefaultResultOrder('ipv4first');
import os from 'node:os';
import path from 'node:path';
process.env.PORTAL_E2E_AUTH_DIRECTORY ||= fs.mkdtempSync(path.join(os.tmpdir(),'portal-e2e-auth-'));
export default defineConfig({
  testDir: './specs', timeout: 120_000, expect: { timeout: 10_000 },
  fullyParallel: false, workers: 1, retries: 0,
  reporter: [['list'], ['json', { outputFile: 'test-results/results.json' }]],
  use: { baseURL: process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443', ignoreHTTPSErrors: true,
    actionTimeout: 30_000, navigationTimeout: 30_000, browserName: 'chromium', launchOptions: { executablePath: process.env.CHROMIUM_PATH || '/usr/bin/chromium', args: ['--no-sandbox'] },
    trace: 'retain-on-failure', screenshot: 'only-on-failure' },
  projects: ['pl', 'en'].flatMap(locale => [1440, 390].map(width => ({ name: `${locale}-${width}`, use: { locale, viewport: { width, height: 900 } }, metadata: { locale } }))),
});
