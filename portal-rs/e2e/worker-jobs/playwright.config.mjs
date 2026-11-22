import { defineConfig } from '@playwright/test';
import shared from '../playwright.config.mjs';

export default defineConfig({
  ...shared,
  testDir: '.',
  testMatch: '*.spec.mjs',
  reporter: 'list',
  outputDir: process.env.PORTAL_WORKER_JOBS_ARTIFACTS,
  use: {
    ...shared.use,
    // The harness's loopback HTTPS proxy has a short HTTP/1.1 keep-alive
    // lifetime. Do not reuse its idle sockets across password-hashing steps.
    extraHTTPHeaders: { Connection: 'close' },
    trace: 'off',
    screenshot: 'off',
  },
});
