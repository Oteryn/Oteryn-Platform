import path from 'node:path';
import { defineConfig } from '@playwright/test';
import base from '../playwright.config.mjs';
import { originProxySettings } from './origin-proxy.mjs';

const { PORTAL_E2E_ORIGIN: origin, PORTAL_E2E_CRAWL_PROXY: proxy, PORTAL_E2E_CRAWL_OUTPUT: outputDir } = process.env;
if (!origin || new URL(origin).hostname !== 'localhost' || !proxy || !outputDir) {
  throw new Error('Run the empty-Game crawl through node crawl/run.mjs');
}

export default defineConfig({
  ...base,
  testDir: '.',
  testMatch: '**/routes.spec.mjs',
  timeout: 600_000,
  outputDir,
  reporter: [['line'], ['json', { outputFile: path.join(path.dirname(outputDir), 'results.json') }]],
  use: {
    ...base.use,
    baseURL: origin,
    proxy: originProxySettings(proxy),
    serviceWorkers: 'block',
    trace: 'off',
    screenshot: 'off',
  },
});
