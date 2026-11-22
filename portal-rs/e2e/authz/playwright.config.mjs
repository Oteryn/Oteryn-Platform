import { defineConfig } from '@playwright/test';
import base from '../playwright.config.mjs';

export default defineConfig({
  ...base,
  testDir: '.',
  testMatch: 'matrix.spec.mjs',
  outputDir: '../test-results/authz',
  reporter: [['list']],
  use: {
    ...base.use,
    trace: 'off',
    screenshot: 'off',
  },
});
