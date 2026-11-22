import { defineConfig } from '@playwright/test';
import base from '../playwright.config.mjs';

export default defineConfig({
  ...base,
  testDir: '.',
  outputDir: '../test-results/abuse',
  reporter: [['list'], ['json', { outputFile: 'test-results/abuse-results.json' }]],
});
