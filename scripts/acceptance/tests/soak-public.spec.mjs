import fs from 'node:fs';
import path from 'node:path';
import { test, expect } from '@playwright/test';
import {
  attachDiagnostics,
  installDiagnostics,
  repoRoot,
  testedSha,
} from './helpers.mjs';

const requestedSeconds = Number.parseInt(process.env.ACCEPTANCE_SOAK_SECONDS ?? '300', 10);
const soakSeconds = Number.isFinite(requestedSeconds) && requestedSeconds > 0 ? requestedSeconds : 300;
const routes = [
  { path: '/', assert: async (page) => expect(page.getByRole('heading', { name: 'Oteryn Platform' })).toBeVisible() },
  { path: '/news', assert: async (page) => expect(page.getByRole('heading', { name: 'News' }).first()).toBeVisible() },
  { path: '/login', assert: async (page) => expect(page.getByRole('heading', { name: 'Sign in to Oteryn Platform' }).first()).toBeVisible() },
];

function percentile(values, fraction) {
  if (values.length === 0) return null;
  const sorted = [...values].sort((a, b) => a - b);
  const index = Math.min(sorted.length - 1, Math.max(0, Math.ceil(sorted.length * fraction) - 1));
  return sorted[index];
}

test.setTimeout((soakSeconds + 90) * 1000);
test.describe.configure({ retries: 0 });

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

test('@soak public read-only journeys remain stable for the bounded soak window', async ({ page }) => {
  const startedAt = Date.now();
  const deadline = startedAt + soakSeconds * 1000;
  const samples = [];
  let iterations = 0;

  while (Date.now() < deadline) {
    iterations += 1;
    for (const route of routes) {
      const started = performance.now();
      const response = await page.goto(route.path, { waitUntil: 'domcontentloaded' });
      const elapsedMs = Math.round((performance.now() - started) * 1000) / 1000;

      expect(response?.status(), `Expected HTTP 200 for ${route.path}`).toBe(200);
      await route.assert(page);
      samples.push({ route: route.path, elapsed_ms: elapsedMs });

      if (Date.now() >= deadline) break;
    }
  }

  const durations = samples.map((sample) => sample.elapsed_ms);
  const metrics = {
    exact_tested_sha: testedSha,
    environment: 'controlled-production-like',
    profile: 'soak',
    browser_project: 'soak-chromium',
    target_duration_seconds: soakSeconds,
    actual_duration_seconds: Math.round((Date.now() - startedAt) / 1000),
    iterations,
    request_count: samples.length,
    latency_ms: {
      min: durations.length > 0 ? Math.min(...durations) : null,
      p50: percentile(durations, 0.50),
      p95: percentile(durations, 0.95),
      max: durations.length > 0 ? Math.max(...durations) : null,
    },
    per_route: Object.fromEntries(routes.map((route) => {
      const routeDurations = samples.filter((sample) => sample.route === route.path).map((sample) => sample.elapsed_ms);
      return [route.path, {
        requests: routeDurations.length,
        p50_ms: percentile(routeDurations, 0.50),
        p95_ms: percentile(routeDurations, 0.95),
        max_ms: routeDurations.length > 0 ? Math.max(...routeDurations) : null,
      }];
    })),
    performance_budget: 'none-calibration-only',
  };

  const output = path.join(repoRoot, 'artifacts', 'acceptance', 'soak-browser-metrics.json');
  fs.mkdirSync(path.dirname(output), { recursive: true });
  fs.writeFileSync(output, `${JSON.stringify(metrics, null, 2)}\n`, 'utf8');

  expect(samples.length).toBeGreaterThan(0);
});
