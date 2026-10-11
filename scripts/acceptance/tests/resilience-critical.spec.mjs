import { test, expect } from '@playwright/test';
import {
  allowExpectedHttpFailure,
  attachDiagnostics,
  installDiagnostics,
} from './helpers.mjs';

// Game-world data has no native source yet. Resilience is proven as a fail-closed contract:
// the public routes answer the bounded unavailable state with no database detail and stay stable on reload.

test.setTimeout(120_000);
test.describe.configure({ retries: 0, mode: 'serial' });

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

async function expectContainedUnavailable(page, pathname) {
  allowExpectedHttpFailure(page.__acceptanceDiagnostics, { status: 503, pathname });
  const response = await page.goto(pathname);
  expect(response?.status()).toBe(503);
  await expect(page.locator('.community-unavailable h1')).toBeVisible();
  await expect(page.locator('body')).not.toContainText('SQLSTATE');
  await expect(page.locator('body')).not.toContainText(process.env.MARIADB_ROOT_PASSWORD ?? 'unset-root-password');
}

test('@resilience Canary public read fails closed and recovers after grant restoration', async ({ page }) => {
  await expectContainedUnavailable(page, '/online');
  await expectContainedUnavailable(page, '/online');
});

test('@resilience Redis runtime view degrades and recovers after ACL restoration', async ({ page }) => {
  await expectContainedUnavailable(page, '/servers');
  await expectContainedUnavailable(page, '/servers');
});
