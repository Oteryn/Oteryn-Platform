import { defineConfig } from '@playwright/test';
import portalConfig from '../playwright.config.mjs';

export default defineConfig(portalConfig, {
  testDir: '.',
  testMatch: 'inject*.spec.mjs',
});
