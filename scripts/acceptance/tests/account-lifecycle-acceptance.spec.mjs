import { test, expect } from '@playwright/test';
import {
  attachDiagnostics,
  installDiagnostics,
  login,
  logout,
  register,
  uniqueEmail,
} from './helpers.mjs';

test.setTimeout(120_000);
test.describe.configure({ retries: 0 });

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);

  if (testInfo.status !== testInfo.expectedStatus && !page.isClosed()) {
    const screenshot = await page.screenshot({
      fullPage: true,
      mask: [page.locator('input'), page.locator('textarea'), page.locator('code')],
    });
    await testInfo.attach('sanitized-failure-screenshot', {
      body: screenshot,
      contentType: 'image/png',
    });
  }
});

test('@portal-account registration, duplicate identity, invalid login and protected account access', async ({ page }) => {
  const email = uniqueEmail('account-lifecycle');
  const password = 'AcceptanceAccountLifecycle!234';

  await page.goto('/register');
  await page.getByRole('button', { name: 'Register' }).click();
  await expect(page.getByRole('alert')).toBeVisible();
  await expect(page).toHaveURL(/\/register$/u);

  await page.getByLabel('Email').fill('not-an-email');
  await page.getByLabel('Password', { exact: true }).fill('short');
  await page.getByLabel('Confirm password').fill('different');
  await page.getByRole('button', { name: 'Register' }).click();
  await expect(page.getByRole('alert')).toBeVisible();

  await register(page, email, password);

  await page.goto('/register');
  await page.getByLabel('Email').fill(email);
  await page.getByLabel('Password', { exact: true }).fill(password);
  await page.getByLabel('Confirm password').fill(password);
  await page.getByRole('button', { name: 'Register' }).click();
  await expect(page.getByRole('alert')).toBeVisible();
  await expect(page).toHaveURL(/\/register$/u);

  await page.goto('/account');
  await expect(page).toHaveURL(/\/login$/u);

  await login(page, email, 'DefinitelyWrong!234');
  await expect(page.getByRole('alert')).toBeVisible();
  await expect(page).toHaveURL(/\/login$/u);

  await login(page, email, password);
  await expect(page).toHaveURL(/\/$/u);

  await page.goto('/account');
  await expect(page.getByRole('heading', { name: 'Account overview' })).toBeVisible();
  await expect(page.getByText(email, { exact: true })).toBeVisible();
  await expect(page.getByText('Ready', { exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Create a character' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Manage security' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Change password' })).toBeVisible();

  await logout(page);
  await page.goto('/account');
  await expect(page).toHaveURL(/\/login$/u);
});
