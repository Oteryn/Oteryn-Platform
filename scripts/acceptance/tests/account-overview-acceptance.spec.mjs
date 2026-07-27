import { test, expect } from '@playwright/test';
import {
  assertAccessibilitySmoke,
  attachDiagnostics,
  evidenceScreenshot,
  installDiagnostics,
  login,
  register,
  runBinary,
  runPhpState,
  uniqueEmail,
} from './helpers.mjs';

const password = 'AcceptanceAccountOverview!234';
const desktopViewport = { width: 1440, height: 1000 };
const mobileViewport = { width: 390, height: 844 };

function seedState(email, state) {
  return JSON.parse(runBinary('php', [
    'scripts/acceptance/seed-account-overview-state.php',
    'seed',
    email,
    state,
  ]));
}

async function assertState(page, state) {
  await expect(page.getByRole('heading', { name: 'Account overview' })).toBeVisible();

  if (state === 'ready') {
    await expect(page.getByText('Ready', { exact: true })).toBeVisible();
    await expect(page.getByRole('link', { name: 'Create a character' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Retry game account setup' })).toHaveCount(0);
  } else if (state === 'pending') {
    await expect(page.getByText('Setup in progress', { exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Retry game account setup' })).toHaveCount(0);
  } else if (state === 'recoverable') {
    await expect(page.getByText('Setup interrupted', { exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Retry game account setup' })).toBeVisible();
  } else {
    await expect(page.getByText('Support required', { exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Retry game account setup' })).toHaveCount(0);
  }

  await assertAccessibilitySmoke(page);
}

async function assertInternalAccountIdHidden(page, accountId) {
  const body = await page.locator('body').innerText();
  expect(body).not.toContain(`Canary account ID: ${accountId}`);
  expect(body).not.toContain(`Game account ID: ${accountId}`);
  await expect(page.getByText(String(accountId), { exact: true })).toHaveCount(0);
}

test.setTimeout(120_000);
test.describe.configure({ retries: 0 });

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

test('@portal-account Account Overview — authorization, status matrix, responsive evidence and recoverable retry', async ({ page }) => {
  const email = uniqueEmail('account-overview');

  await page.goto('/account');
  await expect(page).toHaveURL(/\/login$/u);

  await register(page, email, password);
  await login(page, email, password);
  await expect(page).toHaveURL(/\/account$/u);

  for (const state of ['ready', 'pending', 'recoverable', 'conflict', 'missing']) {
    const fixture = seedState(email, state);

    await page.setViewportSize(desktopViewport);
    await page.goto('/account');
    await assertState(page, state);
    await evidenceScreenshot(page, `account-overview-${state}-desktop`);

    if (fixture.canary_account_id) {
      await assertInternalAccountIdHidden(page, fixture.canary_account_id);
    }
    if (fixture.provisioning_name) {
      expect(await page.locator('body').innerText()).not.toContain(fixture.provisioning_name);
    }

    await page.setViewportSize(mobileViewport);
    await page.reload();
    await assertState(page, state);
    await evidenceScreenshot(page, `account-overview-${state}-mobile`);
  }

  seedState(email, 'recoverable');
  await page.setViewportSize(desktopViewport);
  await page.goto('/account');
  await page.getByRole('button', { name: 'Retry game account setup' }).click();
  await expect(page).toHaveURL(/\/account$/u);
  await expect(page.getByRole('status')).toContainText('Game account setup completed.');
  await expect(page.getByText('Ready', { exact: true })).toBeVisible();

  const binding = runPhpState('binding', email);
  expect(binding.status).toBe('ready');
  expect(binding.canary_account_id).toBeGreaterThan(0);
  await assertInternalAccountIdHidden(page, binding.canary_account_id);
  await evidenceScreenshot(page, 'account-overview-retry-success-desktop');
});
