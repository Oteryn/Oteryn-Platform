import { defineConfig } from '@playwright/test';
import shared from './playwright.config.mjs';

export default defineConfig({
  ...shared,
  testIgnore: [],
  testMatch: '**/game-data.spec.mjs',
  globalSetup: './game-data.setup.mjs',
});
