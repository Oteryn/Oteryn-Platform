import config from '../playwright.config.mjs';

export default {
  ...config,
  testDir: '..',
  testMatch: [config.testMatch ?? 'specs/**/*.@(spec|test).?(c|m)[jt]s?(x)', 'tiers/smoke.spec.mjs'].flat(),
};
