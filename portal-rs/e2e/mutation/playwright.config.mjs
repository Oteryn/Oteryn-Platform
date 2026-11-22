import { defineConfig } from '@playwright/test';
import shared from '../playwright.config.mjs';

export default defineConfig({
  ...shared,
  globalSetup: undefined,
  testDir: '.',
  testMatch: 'matrix.spec.mjs',
});
