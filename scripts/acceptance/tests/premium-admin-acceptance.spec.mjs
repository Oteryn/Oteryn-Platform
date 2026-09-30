import { test, expect } from '@playwright/test';
import {
  allowExpectedHttpFailure,
  assertAccessibilitySmoke,
  attachDiagnostics,
  completeMfaChallenge,
  installDiagnostics,
  login,
  runBinary,
  uniqueEmail,
} from './helpers.mjs';

const adminPassword = 'Acceptance-Premium-Admin-9!Pass';
const adminRecoveryCode = 'PREMIUM-0001';
const responsiveViewports = [
  { width: 1440, height: 1000 },
  { width: 820, height: 1180 },
  { width: 390, height: 844 },
];

function seedAdmin(email) {
  return JSON.parse(runBinary('php', [
    'scripts/acceptance/seed-browser-admin.php',
    email,
    adminPassword,
    adminRecoveryCode,
  ]));
}

function seedPremium(adminEmail, targetEmail) {
  return JSON.parse(runBinary('php', [
    'scripts/acceptance/seed-premium.php',
    adminEmail,
    targetEmail,
  ]));
}

async function assertNoHorizontalOverflow(page) {
  const dimensions = await page.evaluate(() => ({
    viewport: document.documentElement.clientWidth,
    content: document.documentElement.scrollWidth,
  }));
  expect(dimensions.content).toBeLessThanOrEqual(dimensions.viewport + 1);
}

async function assertResponsiveLayout(page) {
  for (const viewport of responsiveViewports) {
    await page.setViewportSize(viewport);
    await expect(page.locator('main')).toBeVisible();
    await assertNoHorizontalOverflow(page);
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

test('@portal-premium-admin MFA permission Premium time grant extend revoke and audit surface', async ({ page }) => {
  const adminEmail = uniqueEmail('premium-admin');
  const targetEmail = uniqueEmail('premium-target');
  seedAdmin(adminEmail);

  await login(page, adminEmail, adminPassword);
  await completeMfaChallenge(page, adminRecoveryCode);
  allowExpectedHttpFailure(page.__acceptanceDiagnostics, { status: 403, pathname: '/admin/premium' });
  const denied = await page.goto('/admin/premium');
  expect(denied?.status()).toBe(403);

  seedPremium(adminEmail, targetEmail);
  await page.goto('/admin/premium');
  await expect(page.getByRole('heading', { name: 'Premium time', level: 1 })).toBeVisible();
  await page.getByLabel('Platform Identity email').fill(targetEmail);
  await page.getByRole('button', { name: 'Find account' }).click();
  await expect(page.getByText(targetEmail, { exact: true })).toBeVisible();
  await expect(page.getByText('NONE', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Revoke Premium time' })).toHaveCount(0);

  await page.getByLabel('Days to grant').fill('30');
  await page.getByLabel('Grant reason').fill('Closed test cohort Premium allocation for acceptance.');
  await page.getByRole('button', { name: 'Grant Premium time' }).click();
  await expect(page.getByRole('status')).toContainText('Premium time granted.');
  await expect(page.getByText('ACTIVE', { exact: true })).toBeVisible();
  await expect(page.getByText('GRANT', { exact: true })).toBeVisible();

  await page.getByLabel('Revocation reason').fill('Acceptance revocation of the test allocation.');
  await page.getByRole('button', { name: 'Revoke Premium time' }).click();
  await expect(page.getByRole('status')).toContainText('Premium time revoked.');
  await expect(page.getByText('REVOKED', { exact: true }).first()).toBeVisible();
  await expect(page.getByRole('button', { name: 'Revoke Premium time' })).toHaveCount(0);

  await page.getByLabel('Days to grant').fill('400');
  await page.getByLabel('Grant reason').fill('Out of range grant must be refused.');
  await page.getByRole('button', { name: 'Grant Premium time' }).evaluate((button) => button.form.noValidate = true);
  await page.getByRole('button', { name: 'Grant Premium time' }).click();
  await expect(page.getByRole('alert')).toContainText('366');
  await expect(page.getByText('REVOKED', { exact: true }).first()).toBeVisible();

  await assertResponsiveLayout(page);
  await assertAccessibilitySmoke(page);
  expect(page.__acceptanceDiagnostics.pageErrors).toEqual([]);
});
