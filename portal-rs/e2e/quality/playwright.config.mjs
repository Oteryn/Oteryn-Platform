import { defineConfig } from '@playwright/test';
import harness from '../playwright.config.mjs';

export default defineConfig({
  ...harness,
  testDir: '.',
  testMatch: 'quality.spec.mjs',
  timeout: 0,
  outputDir: '../test-results/quality',
  reporter: [['list'], ['json', { outputFile: '../test-results/quality-results.json' }]],
  use: { ...harness.use, trace: 'off', screenshot: 'off' },
});
