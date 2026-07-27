import { test, expect } from '@playwright/test';
import {
  attachDiagnostics,
  completeMfaChallenge,
  enrollMfa,
  installDiagnostics,
  login,
  logout,
  register,
  runPhpState,
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
});

test('@portal-account Flow 4 — MFA valid, invalid, replay, recovery single-use, disable and session invalidation', async ({ browser, page }) => {
  const email = uniqueEmail('mfa-security');
  const password = 'AcceptanceMfa!234';

  await register(page, email, password);
  await login(page, email, password);
  const mfa = await enrollMfa(page, password);
  let lastTotp = mfa.enrollmentCode;

  await logout(page);
  await login(page, email, password);
  const validTotp = await waitForDifferentTotp(mfa.secret, lastTotp);
  await completeMfaChallenge(page, validTotp);
  lastTotp = validTotp;
  await expect(page).toHaveURL(/\/$/u);

  await logout(page);
  await login(page, email, password);
  await completeMfaChallenge(page, 'not-a-code');
  await expect(page.getByRole('alert')).toBeVisible();

  await completeMfaChallenge(page, lastTotp);
  await expect(page.getByRole('alert')).toBeVisible();

  const firstRecoveryCode = mfa.recoveryCodes[0];
  await completeMfaChallenge(page, firstRecoveryCode);
  await expect(page).toHaveURL(/\/$/u);
  expect(runPhpState('recovery-code-consumed', email, firstRecoveryCode).recovery_code_consumed).toBe(true);

  const staleContext = await browser.newContext();
  const stalePage = await staleContext.newPage();
  try {
    await login(stalePage, email, password);
    const staleTotp = await waitForDifferentTotp(mfa.secret, lastTotp);
    await completeMfaChallenge(stalePage, staleTotp);
    lastTotp = staleTotp;
    await expect(stalePage).toHaveURL(/\/$/u);

    await page.goto('/mfa');
    await page.getByLabel('Current password').fill(password);
    await page.getByLabel('Fresh authenticator or recovery code').fill(mfa.recoveryCodes[1]);
    await page.getByRole('button', { name: 'Disable MFA and sign out everywhere' }).click();
    await expect(page).toHaveURL(/\/$/u);

    await page.goto('/mfa');
    await expect(page).toHaveURL(/\/login$/u);

    const staleResponse = await stalePage.goto('/mfa');
    expect(staleResponse?.status()).toBe(403);
    await expect(stalePage.getByRole('heading', { name: 'You do not have access to this page' })).toBeVisible();

    await login(page, email, password);
    await expect(page).toHaveURL(/\/mfa$/u);
    await expect(page.getByRole('heading', { name: 'Multi-factor authentication' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'MFA is not enabled' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Complete your sign in' })).toHaveCount(0);
  } finally {
    await staleContext.close();
  }
});
