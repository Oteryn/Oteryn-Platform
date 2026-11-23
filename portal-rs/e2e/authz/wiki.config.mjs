import { defineConfig } from '@playwright/test';
import base from './playwright.config.mjs';

export default defineConfig({
  ...base,
  testMatch: 'wiki.spec.mjs',
  outputDir: '../test-results/authz-wiki',
});
