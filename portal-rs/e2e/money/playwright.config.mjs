import config from '../playwright.config.mjs';

export default {
  ...config,
  testDir: '.',
  outputDir: '../test-results/money',
};
