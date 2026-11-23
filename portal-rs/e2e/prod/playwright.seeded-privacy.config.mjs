import config from './playwright.prod.config.mjs';

export default { ...config, testMatch: 'privacy.seeded.spec.mjs' };
