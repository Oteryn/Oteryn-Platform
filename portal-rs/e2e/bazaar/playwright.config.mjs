import { defineConfig } from '@playwright/test';
import config from '../playwright.config.mjs';

export default defineConfig({
  ...config,
  globalSetup: undefined,
  testDir: '.',
  testMatch: 'legacy.spec.mjs',
});
