import { defineConfig } from '@playwright/test';
import shared from '../playwright.config.mjs';

export default defineConfig({
  ...shared,
  globalSetup: undefined,
  testDir: '.',
  testMatch: 'matrix.spec.mjs',
  timeout: 300_000,
  grepInvert: /@uncredited-payment|@game-publisher/,
  reporter: [['./reporter.mjs']],
  use: { ...shared.use, trace: 'off', screenshot: 'off' },
});
