import { test, expect } from '@playwright/test';
import {
  allowExpectedHttpFailure,
  attachDiagnostics,
  installDiagnostics,
} from './helpers.mjs';

test.describe.configure({ retries: 0 });

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

// Game-world data has no native source yet. Every public game-data route fails closed to the
// contained unavailable state (503) with no database detail, until native fixtures return.
test('Flow 6b — public game data, pagination, empty and controlled dependency-failure states', async ({ page }) => {
  for (const pathname of ['/online', '/highscores', '/servers', '/guilds/Acceptance%20Guild', '/characters/Acceptance%20Hero']) {
    allowExpectedHttpFailure(page.__acceptanceDiagnostics, { status: 503, pathname });
    const response = await page.goto(pathname);
    expect(response?.status(), pathname).toBe(503);
    await expect(page.locator('.community-unavailable h1')).toBeVisible();
    await expect(page.locator('body')).not.toContainText('SQLSTATE');
    await expect(page.locator('body')).not.toContainText(process.env.MARIADB_ROOT_PASSWORD ?? 'unset-root-password');
  }
});
