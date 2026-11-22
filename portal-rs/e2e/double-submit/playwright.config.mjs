import { defineConfig } from '@playwright/test';
import browser from '../playwright.config.mjs';

export default defineConfig({
  ...browser,
  testDir: '.',
  testMatch: '*.spec.mjs',
  use: {
    ...browser.use,
    trace: 'off',
    screenshot: 'off',
    launchOptions: {
      ...browser.use.launchOptions,
      ignoreDefaultArgs: ['--disable-back-forward-cache'],
    },
  },
});
