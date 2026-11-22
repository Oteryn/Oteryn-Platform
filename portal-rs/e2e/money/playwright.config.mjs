import config from '../playwright.config.mjs';

export default {
  ...config,
  globalSetup: undefined,
  testDir: '.',
  testMatch: '**/*.spec.mjs',
  outputDir: '../test-results/money',
};
