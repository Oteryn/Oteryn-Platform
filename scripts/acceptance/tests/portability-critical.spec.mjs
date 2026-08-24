import { test, expect } from '@playwright/test';
import {
  allowExpectedHttpFailure,
  attachDiagnostics,
  completeMfaChallenge,
  enrollMfa,
  installDiagnostics,
  login,
  logout,
  runBinary,
  uniqueEmail,
} from './helpers.mjs';

const regularPassword = 'AcceptanceAccountOverview!234';
const browserAdminPassword = 'Acceptance-Browser-Admin-9!Pass';
const browserAdminRecoveryCode = 'BROWSER-00001';

function seedReadyAccount(email) {
  return JSON.parse(runBinary('php', [
    'scripts/acceptance/seed-account-overview-state.php',
    email,
    regularPassword,
    'ready',
  ]));
}

function seedVisualFixtures() {
  runBinary('php', ['scripts/acceptance/seed.php', 'seed']);
}

function seedBrowserAdmin(email) {
  runBinary('php', [
    'scripts/acceptance/seed-browser-admin.php',
    email,
    browserAdminPassword,
    browserAdminRecoveryCode,
  ]);
}

test.setTimeout(120_000);
test.describe.configure({ retries: 0 });

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

test('@portability public navigation and seeded game data work across browser engines', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Oteryn Platform' })).toBeVisible();

  const navigation = page.getByRole('navigation', { name: 'Public navigation' });
  await navigation.getByRole('link', { name: 'Highscores' }).click();
  await expect(page).toHaveURL(/\/highscores$/u);
  await expect(page.getByText('Acceptance Hero')).toBeVisible();

  await page.goto('/');
  await page.getByLabel('Character name').fill('Acceptance Hero');
  await page.getByRole('button', { name: 'Search' }).click();
  await expect(page).toHaveURL(/\/characters\/Acceptance%20Hero$/u);
  await expect(page.getByRole('heading', { name: 'Acceptance Hero' })).toBeVisible();
});

test('@portability Identity login logout and authenticated Account Overview remain portable', async ({ page }) => {
  const email = uniqueEmail('portability-account');
  seedReadyAccount(email);

  await login(page, email, regularPassword);
  await expect(page).toHaveURL(/\/$/u);

  await page.goto('/account');
  await expect(page.getByRole('heading', { name: 'Account overview' })).toBeVisible();
  await expect(page.getByText('Ready', { exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Create a character' })).toBeVisible();

  await logout(page);
  await page.goto('/account');
  await expect(page).toHaveURL(/\/login$/u);
});

test('@portability MFA-confirmed privileged access and seeded CMS public visibility remain portable', async ({ page }) => {
  const adminEmail = uniqueEmail('portability-admin');
  seedVisualFixtures();
  seedBrowserAdmin(adminEmail);

  await login(page, adminEmail, browserAdminPassword);
  await expect(page.getByRole('heading', { name: 'Complete your sign in' })).toBeVisible();
  await completeMfaChallenge(page, browserAdminRecoveryCode);
  await expect(page).toHaveURL(/\/$/u);

  await page.goto('/admin');
  await expect(page.getByRole('heading', { name: 'Administration' })).toBeVisible();

  const pagesResponse = await page.goto('/admin/pages');
  expect(pagesResponse?.status()).toBe(200);
  await expect(page.getByText('about-oteryn')).toBeVisible();

  await page.goto('/pages/about-oteryn');
  await expect(page.getByRole('heading', { name: 'About Oteryn' })).toBeVisible();
  await expect(page.getByText('Managed public page acceptance content.')).toBeVisible();
});

test('@portability MFA-confirmed user without administrator permission is denied fail closed', async ({ page }) => {
  const email = uniqueEmail('portability-denied');
  seedReadyAccount(email);

  await login(page, email, regularPassword);
  await expect(page).toHaveURL(/\/$/u);
  await enrollMfa(page, regularPassword);

  allowExpectedHttpFailure(page.__acceptanceDiagnostics, { status: 403, pathname: '/admin' });
  const denied = await page.goto('/admin');
  expect(denied?.status()).toBe(403);
  await expect(page.getByRole('heading', { name: 'You do not have access to this page' })).toBeVisible();
});
