import { defineConfig } from '@playwright/test';
import base from './playwright.config.mjs';

export default defineConfig({
  ...base,
  testMatch: 'checkout-unavailable.spec.mjs',
  outputDir: '../test-results/authz-checkout-unavailable',
});
