import { run } from './harness.mjs';

await run({ config: 'api/playwright.config.mjs', args: process.argv.slice(2) });
