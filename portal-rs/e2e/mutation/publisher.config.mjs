import { defineConfig } from '@playwright/test';
import shared from '../playwright.config.mjs';
import { GamePublishers } from '../game-publishers.mjs';

const report = process.env.PORTAL_E2E_GAME_DATA_REPORT;
if (!report) throw new Error('Set PORTAL_E2E_GAME_DATA_REPORT to the report from harness.mjs --game-data');
const game = new GamePublishers(report);
process.env.PORTAL_E2E_DATABASE_URL = game.env.OTERYN_DATABASE_URL;

export default defineConfig({
  ...shared,
  testDir: '.',
  testMatch: 'publisher.spec.mjs',
  use: { ...shared.use, baseURL: game.origin },
  projects: shared.projects.map(project => ({
    ...project,
    use: {
      ...project.use,
      extraHTTPHeaders: { 'accept-language': project.metadata.locale },
    },
  })),
});
