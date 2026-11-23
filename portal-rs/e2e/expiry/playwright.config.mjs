import { defineConfig } from '@playwright/test';
import portal from '../playwright.config.mjs';

export default defineConfig({
  ...portal,
  testDir: '.',
  testMatch: '**/*.spec.mjs',
  outputDir: 'test-results/artifacts',
  reporter: [['json', { outputFile: 'test-results/expiry.json' }]],
});
