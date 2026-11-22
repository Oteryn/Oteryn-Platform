import { defineConfig } from '@playwright/test';
import base from './playwright.config.mjs';

export default defineConfig({
  ...base,
  testMatch: 'sessions.spec.mjs',
  outputDir: '../test-results/authz-sessions',
});
