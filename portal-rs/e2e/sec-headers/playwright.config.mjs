import { defineConfig } from '@playwright/test';
import browserConfig from '../playwright.config.mjs';

export default defineConfig({
  testDir: '.',
  timeout: browserConfig.timeout,
  workers: 1,
  retries: 0,
  reporter: 'list',
  projects: browserConfig.projects.map(project => ({
    ...project,
    use: { ...browserConfig.use, ...project.use, trace: 'off', screenshot: 'off', video: 'off' },
  })),
});
