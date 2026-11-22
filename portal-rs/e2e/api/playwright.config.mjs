import { defineConfig } from '@playwright/test';
import browserConfig from '../playwright.config.mjs';

const browserSpecs = ['oauth.spec.mjs', 'collection.spec.mjs'];

export default defineConfig({
  testDir: '.',
  timeout: browserConfig.timeout,
  retries: 0,
  workers: 1,
  use: { trace: 'off', screenshot: 'off', baseURL: process.env.PORTAL_E2E_ORIGIN },
  projects: [
    { name: 'api', testIgnore: browserSpecs },
    ...browserConfig.projects.map(project => ({
      ...project,
      testMatch: browserSpecs,
      use: { ...browserConfig.use, ...project.use, trace: 'off', screenshot: 'off' },
    })),
  ],
});
