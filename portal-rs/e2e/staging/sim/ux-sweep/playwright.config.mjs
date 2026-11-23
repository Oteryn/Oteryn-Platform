import { defineConfig } from '@playwright/test';
import { origin, projects } from './helpers.mjs';

const proxyURL = process.env.HTTPS_PROXY || process.env.https_proxy;

export default defineConfig({
  testDir: '.',
  outputDir: process.env.OTERYN_UX_OUTPUT || '/tmp/oteryn-ux-sweep',
  timeout: 20 * 60_000,
  expect: { timeout: 10_000 },
  workers: 2,
  retries: 0,
  reporter: 'line',
  use: {
    baseURL: origin,
    browserName: 'chromium',
    launchOptions: { executablePath: '/usr/bin/chromium' },
    ...(proxyURL && { proxy: { server: proxyURL } }),
    trace: 'off',
    screenshot: 'off',
    video: 'off',
    navigationTimeout: 25_000,
  },
  projects: projects.map(({ name, lang, viewport }) => ({
    name,
    use: { locale: lang, viewport },
    metadata: { lang, viewport },
  })),
});
