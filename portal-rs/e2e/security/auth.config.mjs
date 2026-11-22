import { defineConfig } from '@playwright/test';
import base from '../playwright.config.mjs';

if (new URL(base.use.baseURL).origin !== 'https://localhost:18443') {
  throw new Error('Auth qualification requires the loopback HTTPS browser launcher');
}

export default defineConfig({
  ...base,
  testDir: '.',
  testMatch: 'auth*.spec.mjs',
  outputDir: '../test-results/auth',
  reporter: [['list']],
  use: { ...base.use, trace: 'off', screenshot: 'off' },
});
