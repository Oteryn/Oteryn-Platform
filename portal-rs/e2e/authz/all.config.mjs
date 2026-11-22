import { defineConfig } from '@playwright/test';
import base from './playwright.config.mjs';

export default defineConfig({
  ...base,
  testMatch: '*.spec.mjs',
  testIgnore: ['checkout-unavailable.spec.mjs', 'bazaar.spec.mjs'],
  outputDir: '../test-results/authz-all',
});
