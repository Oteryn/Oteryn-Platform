import { defineConfig } from '@playwright/test';
import browserConfig from '../playwright.config.mjs';

export default defineConfig({
  testDir: '.',
  timeout: browserConfig.timeout,
  retries: 0,
  workers: 1,
  projects: browserConfig.projects.map(project => ({
    ...project,
    use: { ...browserConfig.use, ...project.use, trace: 'off', screenshot: 'off' },
  })),
});
