import { defineConfig } from '@playwright/test';
import portal from '../playwright.config.mjs';

const harnessOrigin = 'https://localhost:18443';
if (new URL(portal.use.baseURL).href !== new URL(harnessOrigin).href) {
  throw new Error('Expiry requires the loopback portal from expiry/launch.mjs');
}

export default defineConfig({
  ...portal,
  testDir: '.',
  testMatch: '**/*.spec.mjs',
  outputDir: 'test-results/artifacts',
  reporter: [['json', { outputFile: 'test-results/expiry.json' }]],
  use: { ...portal.use, ignoreHTTPSErrors: false },
});
