import { defineConfig } from '@playwright/test';
import base from '../playwright.config.mjs';

export default defineConfig({
  ...base,
  timeout: 180_000,
  testDir: '.',
  testMatch: 'limits.spec.mjs',
  outputDir: '../test-results/abuse',
  reporter: [['list'], ['json', { outputFile: 'test-results/abuse-results.json' }]],
});
