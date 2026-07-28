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

const mailhogBaseUrl = process.env.ACCEPTANCE_MAILHOG_URL ?? 'http://127.0.0.1:8025';

function collectStrings(value, output = []) {
  if (typeof value === 'string') {
    output.push(value);
    return output;
  }
  if (Array.isArray(value)) {
    for (const item of value) collectStrings(item, output);
    return output;
  }
  if (value && typeof value === 'object') {
    for (const item of Object.values(value)) collectStrings(item, output);
  }
  return output;
}

function decodeQuotedPrintable(value) {
  return value
    .replace(/=\r?\n/gu, '')
    .replace(/=([0-9A-F]{2})/giu, (_, hex) => String.fromCharCode(Number.parseInt(hex, 16)))
    .replace(/&amp;/gu, '&');
}

async function waitForExactResetLink(email, timeoutMs = 20_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const response = await fetch(`${mailhogBaseUrl}/api/v2/messages`);
    if (response.ok) {
      const payload = await response.json();
      for (const raw of collectStrings(payload)) {
        if (!raw.includes(email) && !raw.includes(encodeURIComponent(email))) continue;
        const decoded = decodeQuotedPrintable(raw);
        const candidates = decoded.match(/https?:\/\/[^\s<>"']+/gu) ?? [];
        for (const candidateRaw of candidates) {
          const candidate = candidateRaw.replace(/[\])}>.,;]+$/gu, '');
          try {
            const url = new URL(candidate);
            if (!url.pathname.includes('/reset-password/')) continue;
            const candidateEmail = url.searchParams.get('email');
            if (candidateEmail === null || candidateEmail === email) return url.toString();
          } catch {
            // Ignore non-URL text fragments and continue polling MailHog.
          }
        }
      }
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error('Password reset message did not expose a valid reset URL for the requested identity.');
}

test.setTimeout(120_000);

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

test('@portal-account Flow 3b — password recovery uses real SMTP, revokes old sessions and rejects token replay', async ({ browser, page }) => {
  const email = uniqueEmail('password-recovery');
  const originalPassword = 'AcceptanceRecovery!234';
  const changedPassword = 'AcceptanceRecovered!567';

  await register(page, email, originalPassword);
  await login(page, email, originalPassword);
  await expect(page).toHaveURL(/\/$/u);

  const mfa = await enrollMfa(page, originalPassword);
  let lastTotp = mfa.enrollmentCode;
  await page.goto('/mfa');
  await expect(page.getByRole('heading', { name: 'Multi-factor authentication' })).toBeVisible();
  await expect(page.getByText('MFA is enabled.')).toBeVisible();

  const resetContext = await browser.newContext();
  const resetPage = await resetContext.newPage();
  try {
    await resetPage.goto('/forgot-password');
    await resetPage.getByLabel('Email').fill(email);
    await resetPage.getByRole('button', { name: 'Send reset link' }).click();
    await expect(resetPage.getByRole('status')).toBeVisible();

    const resetLink = await waitForExactResetLink(email);
    await resetPage.goto(resetLink);
    await resetPage.getByLabel('Email').fill(email);
    await resetPage.getByLabel('New password', { exact: true }).fill(changedPassword);
    await resetPage.getByLabel('Confirm new password', { exact: true }).fill(changedPassword);
    await resetPage.getByRole('button', { name: 'Reset password' }).click();
    await expect(resetPage.getByRole('status')).toContainText('Your password has been reset. Sign in again.');

    await page.goto('/mfa');
    await expect(page).toHaveURL(/\/login$/u);
    await expect(page.getByRole('heading', { name: 'Sign in to Oteryn Platform' })).toBeVisible();

    await resetPage.getByLabel('Email').fill(email);
    await resetPage.getByLabel('Password').fill(originalPassword);
    await resetPage.getByRole('button', { name: 'Sign in' }).click();
    await expect(resetPage.getByRole('alert')).toBeVisible();

    await resetPage.goto(resetLink);
    await resetPage.getByLabel('Email').fill(email);
    await resetPage.getByLabel('New password', { exact: true }).fill('AcceptanceReplay!890');
    await resetPage.getByLabel('Confirm new password', { exact: true }).fill('AcceptanceReplay!890');
    await resetPage.getByRole('button', { name: 'Reset password' }).click();
    await expect(resetPage.getByRole('alert')).toContainText('This password reset link is invalid or expired.');

    await login(resetPage, email, changedPassword);
    const code = await waitForDifferentTotp(mfa.secret, lastTotp);
    await completeMfaChallenge(resetPage, code);
    lastTotp = code;
    await expect(resetPage).toHaveURL(/\/$/u);
    expect(lastTotp).not.toBe(mfa.enrollmentCode);
  } finally {
    await resetContext.close();
  }
});
