import { defineConfig } from '@playwright/test';
import browserConfig from '../playwright.config.mjs';

export default defineConfig({
  testDir: '.',
  testMatch: process.env.PORTAL_E2E_SEC_HEADERS_MODE === 'without-mfa'
    ? 'plain-login.spec.mjs' : ['browser.spec.mjs', 'routes.spec.mjs'],
  timeout: browserConfig.timeout,
  workers: 1,
  retries: 0,
  reporter: 'list',
  projects: browserConfig.projects.map(project => ({
    ...project,
    use: { ...browserConfig.use, ...project.use, trace: 'off', screenshot: 'off', video: 'off' },
  })),
});
