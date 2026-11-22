import { defineConfig } from '@playwright/test';
import browserConfig from '../playwright.config.mjs';

export default defineConfig({
  testDir: '.',
  timeout: browserConfig.timeout,
  retries: 0,
  workers: 1,
  use: { trace: 'off', screenshot: 'off', baseURL: process.env.PORTAL_E2E_ORIGIN },
  projects: [
    { name: 'api', testIgnore: 'oauth.spec.mjs' },
    ...browserConfig.projects.map(project => ({
      ...project,
      testMatch: 'oauth.spec.mjs',
      use: { ...browserConfig.use, ...project.use, trace: 'off', screenshot: 'off' },
    })),
  ],
});
