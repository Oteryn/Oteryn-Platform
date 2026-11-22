import { defineConfig } from '@playwright/test';
import base from './playwright.config.mjs';

export default defineConfig({
  ...base,
  testMatch: 'protected.spec.mjs',
  outputDir: '../test-results/authz-protected',
});
