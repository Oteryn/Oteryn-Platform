import { defineConfig } from '@playwright/test';
import shared from '../playwright.config.mjs';

export default defineConfig({
  ...shared,
  testDir: '.',
  testMatch: 'matrix.spec.mjs',
});
