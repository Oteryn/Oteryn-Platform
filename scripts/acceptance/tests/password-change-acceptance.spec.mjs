import { test, expect } from '@playwright/test';
import {
  attachDiagnostics,
  completeMfaChallenge,
  enrollMfa,
  installDiagnostics,
  login,
  register,
  uniqueEmail,
  waitForDifferentTotp,
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

test('@portal-account Flow 3c — authenticated password change revokes existing sessions and requires the new password', async ({ browser, page }) => {
  const email = uniqueEmail('password-change');
  const originalPassword = 'AcceptanceChange!234';
  const changedPassword = 'AcceptanceChanged!567';

  await register(page, email, originalPassword);
  await login(page, email, originalPassword);
  await expect(page).toHaveURL(/\/$/u);

  const mfa = await enrollMfa(page, originalPassword);
  let lastTotp = mfa.enrollmentCode;

  const staleContext = await browser.newContext();
  const stalePage = await staleContext.newPage();
  try {
    await login(stalePage, email, originalPassword);
    const staleCode = await waitForDifferentTotp(mfa.secret, lastTotp);
    await completeMfaChallenge(stalePage, staleCode);
    lastTotp = staleCode;
    await expect(stalePage).toHaveURL(/\/$/u);

    await page.goto('/password/change');
    await page.getByLabel('Current password').fill(originalPassword);
    await page.getByLabel('New password', { exact: true }).fill(changedPassword);
    await page.getByLabel('Confirm new password', { exact: true }).fill(changedPassword);
    await page.getByRole('button', { name: 'Change password' }).click();
    await expect(page.getByRole('status')).toContainText('Your password has been changed. Sign in again.');
    await expect(page).toHaveURL(/\/login$/u);

    await page.goto('/mfa');
    await expect(page).toHaveURL(/\/login$/u);

    const staleSessionResponse = await stalePage.goto('/mfa');
    expect(staleSessionResponse?.status()).toBe(403);
    await expect(stalePage.getByRole('heading', { name: 'You do not have access to this page' })).toBeVisible();

    await login(page, email, originalPassword);
    await expect(page.getByRole('alert')).toBeVisible();

    await login(page, email, changedPassword);
    const freshCode = await waitForDifferentTotp(mfa.secret, lastTotp);
    await completeMfaChallenge(page, freshCode);
    lastTotp = freshCode;
    await expect(page).toHaveURL(/\/mfa$/u);
    await expect(page.getByRole('heading', { name: 'Multi-factor authentication' })).toBeVisible();
    await expect(page.getByText('MFA is enabled.')).toBeVisible();
    expect(lastTotp).not.toBe(mfa.enrollmentCode);
  } finally {
    await staleContext.close();
  }
});
