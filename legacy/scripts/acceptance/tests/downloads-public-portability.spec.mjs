import { test, expect } from '@playwright/test';
import path from 'node:path';
import {
  attachDiagnostics,
  installDiagnostics,
  repoRoot,
  runBinary,
} from './helpers.mjs';

test.setTimeout(60_000);
test.describe.configure({ retries: 0 });

function downloadsState(...args) {
  const output = runBinary('php', [
    path.join(repoRoot, 'scripts/acceptance/seed-downloads-state.php'),
    ...args,
  ]);

  return JSON.parse(output);
}

test.beforeAll(() => {
  const state = downloadsState('seed-portability');
  if (state.seeded !== true || state.version !== '7.0.0-portability') {
    throw new Error('Downloads portability fixture did not seed the expected current release.');
  }
});

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

async function assertNoPageOverflow(page) {
  const dimensions = await page.evaluate(() => ({
    viewport: window.innerWidth,
    document: document.documentElement.scrollWidth,
  }));

  expect(dimensions.document, `Unexpected page overflow on ${page.url()}`).toBeLessThanOrEqual(dimensions.viewport + 1);
}

test('@portal-downloads-portability current public release renders in bounded browser engines', async ({ page }) => {
  await page.goto('/en/download');
  await expect(page.getByRole('heading', { name: 'Download Center' })).toBeVisible();
  await expect(page.getByRole('heading', { name: /Oteryn Client/u }).first()).toBeVisible();
  await expect(page.getByRole('table', { name: /Client artifacts/u }).first()).toBeVisible();
  await expect(page.locator('article.card a[href^="https://downloads.example.test/"]').first()).toBeVisible();
  await assertNoPageOverflow(page);

  await page.goto('/en/download/windows');
  await expect(page.getByRole('heading', { name: /Oteryn Client/u }).first()).toBeVisible();
  await assertNoPageOverflow(page);

  await page.goto('/pl/download');
  await expect(page.getByRole('heading', { name: /Klient Oteryn/u }).first()).toBeVisible();
  await expect(page.getByRole('table', { name: /Artefakty klienta/u }).first()).toBeVisible();
  await assertNoPageOverflow(page);

  expect(page.__acceptanceDiagnostics.pageErrors).toEqual([]);
  expect(page.__acceptanceDiagnostics.serverErrors).toEqual([]);
});
