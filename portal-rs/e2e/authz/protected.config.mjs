import { defineConfig } from '@playwright/test';
import base from './playwright.config.mjs';

export default defineConfig({
  ...base,
  timeout: 240_000,
  testMatch: 'protected.spec.mjs',
  outputDir: '../test-results/authz-protected',
});
