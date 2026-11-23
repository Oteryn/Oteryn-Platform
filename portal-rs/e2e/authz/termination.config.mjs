import { defineConfig } from '@playwright/test';
import base from './playwright.config.mjs';

export default defineConfig({
  ...base,
  testMatch: 'termination.spec.mjs',
  outputDir: '../test-results/authz-termination',
});
