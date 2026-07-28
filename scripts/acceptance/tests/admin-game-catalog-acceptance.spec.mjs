import { test, expect } from '@playwright/test';
import {
  assertAccessibilitySmoke,
  attachDiagnostics,
  completeMfaChallenge,
  installDiagnostics,
  login,
  runBinary,
  uniqueEmail,
} from './helpers.mjs';

const adminPassword = 'Acceptance-Game-Catalog-Admin-9!Pass';
const adminRecoveryCode = 'CATALOG-00001';
const responsiveViewports = [
  { width: 1440, height: 1000 },
  { width: 820, height: 1180 },
  { width: 390, height: 844 },
];

function seedGameCatalog() {
  return JSON.parse(runBinary('php', ['scripts/acceptance/seed-game-catalog.php']));
}

function seedAdmin(email) {
  return JSON.parse(runBinary('php', [
    'scripts/acceptance/seed-browser-admin.php',
    email,
    adminPassword,
    adminRecoveryCode,
  ]));
}

async function assertResponsiveLayout(page) {
  for (const viewport of responsiveViewports) {
    await page.setViewportSize(viewport);
    await expect(page.locator('main')).toBeVisible();
    const dimensions = await page.evaluate(() => ({
      viewport: document.documentElement.clientWidth,
      content: document.documentElement.scrollWidth,
    }));
    expect(dimensions.content).toBeLessThanOrEqual(dimensions.viewport + 1);
  }

  await page.setViewportSize(responsiveViewports[0]);
}

test.setTimeout(120_000);
test.describe.configure({ retries: 0 });

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

test('@portal-game-catalog-admin MFA RBAC snapshot profile finding diff responsive and accessibility inspection', async ({ page }) => {
  const fixture = seedGameCatalog();
  const adminEmail = uniqueEmail('game-catalog-admin');
  seedAdmin(adminEmail);

  await login(page, adminEmail, adminPassword);
  await completeMfaChallenge(page, adminRecoveryCode);
  await page.goto('/admin/game-catalog');
  await expect(page.getByRole('heading', { level: 1, name: 'Game Catalog' })).toBeVisible();
  await expect(page.getByText('Public Game Catalog', { exact: true })).toBeVisible();
  await expect(page.locator('input[type="file"]')).toHaveCount(0);
  await expect(page.getByRole('button', { name: /activate snapshot/iu })).toHaveCount(0);
  await assertResponsiveLayout(page);
  await assertAccessibilitySmoke(page);

  await page.goto('/admin/game-catalog/snapshots');
  await expect(page.getByRole('heading', { level: 1, name: 'Snapshots' })).toBeVisible();
  await page.getByRole('link', { name: `#${fixture.snapshot_id}`, exact: true }).click();
  await expect(page.getByRole('heading', { level: 1, name: `Snapshot #${fixture.snapshot_id}` })).toBeVisible();
  await expect(page.getByText('item:fixture-sword', { exact: true })).toBeVisible();
  await expect(page.getByText('future_release', { exact: true })).toBeVisible();
  await expect(page.getByText('partial', { exact: true })).toBeVisible();
  await assertResponsiveLayout(page);
  await assertAccessibilitySmoke(page);

  await page.goto(`/admin/game-catalog/profiles/${fixture.profile_id}`);
  await expect(page.getByRole('heading', { level: 1, name: 'Public Game Catalog' })).toBeVisible();
  await expect(page.getByText('item:fixture-sword', { exact: true })).toBeVisible();
  await expect(page.getByText('visible', { exact: true }).first()).toBeVisible();
  await assertResponsiveLayout(page);
  await assertAccessibilitySmoke(page);

  await page.goto(`/admin/game-catalog/findings?snapshot_id=${fixture.snapshot_id}`);
  await expect(page.getByRole('heading', { level: 1, name: 'Findings' })).toBeVisible();
  await expect(page.getByText('No validation findings match these filters.', { exact: true })).toBeVisible();
  await assertResponsiveLayout(page);
  await assertAccessibilitySmoke(page);

  await page.goto('/admin/game-catalog/diff');
  await expect(page.getByRole('heading', { level: 1, name: 'Snapshot diff' })).toBeVisible();
  await expect(page.getByLabel('Snapshot A')).toBeVisible();
  await expect(page.getByLabel('Snapshot B')).toBeVisible();
  await assertResponsiveLayout(page);
  await assertAccessibilitySmoke(page);
});
