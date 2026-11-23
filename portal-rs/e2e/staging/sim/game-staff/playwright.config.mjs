import { defineConfig } from '@playwright/test';
import { randomUUID } from 'node:crypto';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { baseURL, proxy } from './helpers.mjs';

process.env.OTERYN_SIM_GAME_STAFF_RUN ??= randomUUID();

export default defineConfig({
  testDir: '.',
  outputDir: join(tmpdir(), 'oteryn-sim-game-staff', process.env.OTERYN_SIM_GAME_STAFF_RUN),
  workers: 2,
  retries: 0,
  timeout: 540_000,
  expect: { timeout: 5_000 },
  reporter: 'list',
  use: {
    baseURL,
    proxy,
    // The execution environment uses a managed HTTPS proxy.
    ignoreHTTPSErrors: Boolean(proxy),
    serviceWorkers: 'block',
    launchOptions: { executablePath: '/usr/bin/chromium' },
    actionTimeout: 12_000,
    navigationTimeout: 20_000,
    trace: 'off',
    screenshot: 'off',
    video: 'off',
  },
  projects: ['pl', 'en'].flatMap(lang => [1440, 390].map(width => ({
    name: `${lang}-${width}`,
    metadata: { lang },
    use: {
      locale: lang === 'pl' ? 'pl-PL' : 'en-US',
      viewport: { width, height: width === 390 ? 844 : 900 },
    },
  }))),
});
