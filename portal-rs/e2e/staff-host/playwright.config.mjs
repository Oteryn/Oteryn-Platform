import { defineConfig } from '@playwright/test';
import base from '../playwright.config.mjs';

export default defineConfig({
  ...base,
  testDir: '.',
  testMatch: 'dashboard-return.spec.mjs',
});
