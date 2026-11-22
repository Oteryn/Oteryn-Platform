import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createRequire } from 'node:module';

const { defineConfig } = createRequire(import.meta.url)('@playwright/test');
const baseURL = new URL(process.env.LIVE_BASE_URL || 'https://oteryn.com');
if (!['https:', 'http:'].includes(baseURL.protocol) || baseURL.username || baseURL.password) {
  throw new Error('LIVE_BASE_URL must be an HTTP(S) URL without credentials');
}

// The runner starts a fresh process after a failing project. Keep the budget
// on disk so its replacement uses the same count and last request time.
if (!process.env.OTERYN_SMOKE_RUN_DIR) {
  process.env.OTERYN_SMOKE_RUN_DIR = mkdtempSync(join(tmpdir(), 'oteryn-smoke-'));
  writeFileSync(join(process.env.OTERYN_SMOKE_RUN_DIR, 'budget.json'), '[0,0]');
}

export default defineConfig({
  testDir: '.',
  testMatch: 'live.spec.mjs',
  workers: 1,
  retries: 0,
  timeout: 20 * 60_000,
  outputDir: join(process.env.OTERYN_SMOKE_RUN_DIR, 'results'),
  reporter: 'list',
  use: {
    browserName: 'chromium',
    baseURL: baseURL.origin,
    serviceWorkers: 'block',
    userAgent: 'OterynLiveSmoke/1.0 (Playwright)',
    // Only the Node request gate is allowed to reach the network.
    proxy: { server: 'http://127.0.0.1:1', bypass: '<-loopback>' },
    launchOptions: {
      executablePath: process.env.LIVE_CHROMIUM,
      args: ['--disable-quic', '--force-webrtc-ip-handling-policy=disable_non_proxied_udp'],
    },
  },
  projects: ['pl', 'en'].flatMap(language => [1440, 390].map(width => ({
    name: `${language}-${width}`,
    use: { locale: language, viewport: { width, height: width === 390 ? 844 : 900 } },
  }))),
});
