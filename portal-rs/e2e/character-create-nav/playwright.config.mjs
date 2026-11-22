import { defineConfig } from '@playwright/test';
import browserConfig from '../playwright.config.mjs';

export default defineConfig({
  ...browserConfig,
  testDir: '.',
  testMatch: 'navigation.spec.mjs',
  use: { ...browserConfig.use, trace: 'off', screenshot: 'off' },
});
