import { test, expect } from '@playwright/test';
import {
  attachDiagnostics,
  installDiagnostics,
  login,
  mailhogBaseUrl,
  register,
  uniqueEmail,
} from './helpers.mjs';

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

function decodeMailBody(value) {
  return value
    .replace(/=\r?\n/gu, '')
    .replace(/=([0-9A-F]{2})/giu, (_, hex) => String.fromCharCode(Number.parseInt(hex, 16)))
    .replace(/\\\//gu, '/')
    .replace(/&amp;/gu, '&');
}

async function waitForEmailChangeLink(email, pathFragment, timeoutMs = 20_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const response = await fetch(`${mailhogBaseUrl}/api/v2/messages`);
    if (response.ok) {
      const payload = await response.json();
      for (const raw of collectStrings(payload)) {
        if (!raw.includes(email) && !raw.includes(encodeURIComponent(email))) continue;
        const decoded = decodeMailBody(raw);
        const candidates = decoded.match(/https?:\/\/[^\s<>"']+/gu) ?? [];
        for (const candidateRaw of candidates) {
          const candidate = candidateRaw.replace(/[\])}>.,;]+$/gu, '');
          try {
            const url = new URL(candidate);
            if (url.pathname.includes(pathFragment)) return url.toString();
          } catch {
            // Ignore unrelated mail fragments and keep polling the isolated SMTP service.
          }
        }
      }
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error(`MailHog did not expose the expected ${pathFragment} link for the requested identity.`);
}

async function assertResponsiveSecurityPage(page, heading = 'Security and lifecycle') {
  for (const viewport of [
    { width: 820, height: 1180 },
    { width: 390, height: 844 },
  ]) {
    await page.setViewportSize(viewport);
    await expect(page.getByRole('heading', { name: heading })).toBeVisible();
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
    expect(overflow).toBeLessThanOrEqual(1);
  }
  await page.setViewportSize({ width: 1440, height: 1000 });
}

test.setTimeout(180_000);
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

test('@portal-account account security lifecycle covers sessions, privacy, verified email recovery, recovery keys and termination grace', async ({ browser, page }) => {
  const email = uniqueEmail('account-security');
  const changedEmail = uniqueEmail('account-security-changed');
  const password = 'AcceptanceSecurity!234';
  const recoveredPassword = 'AcceptanceSecurityRecovered!567';

  await register(page, email, password);
  await login(page, email, password);
  await page.goto('/account/security');
  await expect(page.getByRole('heading', { name: 'Security and lifecycle' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Current session' })).toBeVisible();
  await assertResponsiveSecurityPage(page);

  const secondContext = await browser.newContext();
  const secondPage = await secondContext.newPage();
  try {
    await login(secondPage, email, password);
    await expect(secondPage).toHaveURL(/\/$/u);

    await page.reload();
    await expect(page.getByRole('heading', { name: 'Other session' })).toBeVisible();
    await page.getByRole('button', { name: 'Revoke all other sessions' }).click();
    await expect(page.getByRole('status')).toContainText('All other active sessions have been revoked.');

    await secondPage.goto('/account');
    await expect(secondPage).toHaveURL(/\/login$/u);
  } finally {
    await secondContext.close();
  }

  const privacy = page.locator('section[aria-labelledby="privacy-heading"]');
  await privacy.getByRole('checkbox').nth(0).check();
  await privacy.getByRole('checkbox').nth(1).check();
  await privacy.getByRole('button', { name: 'Save privacy settings' }).click();
  await expect(page.getByRole('status')).toContainText('Account privacy settings have been updated.');
  await expect(privacy.getByRole('checkbox').nth(0)).toBeChecked();
  await expect(privacy.getByRole('checkbox').nth(1)).toBeChecked();

  const termination = page.locator('section[aria-labelledby="termination-heading"]');
  await termination.getByLabel('Current password').fill(password);
  await termination.getByLabel('Type TERMINATE to confirm').fill('TERMINATE');
  await termination.getByRole('button', { name: 'Schedule account termination' }).click();
  await expect(page).toHaveURL(/\/login$/u);
  await expect(page.getByRole('status')).toContainText('Account termination is scheduled');

  await login(page, email, password);
  await page.goto('/account/security');
  await expect(page.getByRole('status')).toContainText('Termination is scheduled for');
  const pendingTermination = page.locator('section[aria-labelledby="termination-heading"]');
  await pendingTermination.getByLabel('Current password').fill(password);
  await pendingTermination.getByRole('button', { name: 'Cancel termination' }).click();
  await expect(page.getByRole('status')).toContainText('The pending account termination has been cancelled.');

  const confirmationLinkPromise = waitForEmailChangeLink(changedEmail, '/email-change/confirm/');
  const recoveryLinkPromise = waitForEmailChangeLink(email, '/email-change/recover/');
  const primaryEmail = page.locator('section[aria-labelledby="primary-email-heading"]');
  await primaryEmail.getByLabel('New email address', { exact: true }).fill(changedEmail);
  await primaryEmail.getByLabel('Confirm new email address', { exact: true }).fill(changedEmail);
  await primaryEmail.getByLabel('Current password').fill(password);
  await primaryEmail.getByRole('button', { name: 'Send confirmation links' }).click();
  await expect(page.getByRole('status').filter({ hasText: 'Check the new email address to confirm the change.' })).toBeVisible();

  const confirmationLink = await confirmationLinkPromise;
  const recoveryLink = await recoveryLinkPromise;

  await page.goto(confirmationLink);
  await expect(page.getByRole('heading', { name: 'Confirm the new email address' })).toBeVisible();
  await page.getByRole('button', { name: 'Confirm email change' }).click();
  await expect(page).toHaveURL(/\/login$/u);
  await expect(page.getByRole('status')).toContainText('Your primary email address has been changed.');

  await page.goto(recoveryLink);
  await expect(page.getByRole('heading', { name: 'Cancel or recover the email change' })).toBeVisible();
  await page.getByRole('button', { name: 'Cancel or recover email change' }).click();
  await expect(page).toHaveURL(/\/login$/u);
  await expect(page.getByRole('status')).toContainText('The previous email address has been restored.');

  await login(page, email, password);
  await page.goto('/account/security');
  const recoveryKey = page.locator('section[aria-labelledby="recovery-key-heading"]');
  await recoveryKey.getByLabel('Current password').first().fill(password);
  await recoveryKey.getByRole('button', { name: 'Generate recovery key' }).click();
  await expect(page.getByRole('heading', { name: 'Store this recovery key now' })).toBeVisible();
  await expect(page.getByRole('status', { name: 'New recovery key' }).locator('code')).toHaveText(/^OTERYN-/u);
  await page.getByRole('link', { name: 'Return to account security' }).click();

  const activeRecoveryKey = page.locator('section[aria-labelledby="recovery-key-heading"]');
  await activeRecoveryKey.getByLabel('Current password').last().fill(password);
  await activeRecoveryKey.getByRole('button', { name: 'Revoke recovery key' }).click();
  await expect(page.getByRole('status')).toContainText('The recovery key has been revoked.');

  const replacementRecoveryKey = page.locator('section[aria-labelledby="recovery-key-heading"]');
  await replacementRecoveryKey.getByLabel('Current password').first().fill(password);
  await replacementRecoveryKey.getByRole('button', { name: 'Generate recovery key' }).click();
  const rawRecoveryKey = (await page.getByRole('status', { name: 'New recovery key' }).locator('code').textContent())?.trim();
  expect(rawRecoveryKey).toMatch(/^OTERYN-/u);

  const recoveryContext = await browser.newContext();
  const recoveryPage = await recoveryContext.newPage();
  try {
    await recoveryPage.goto('/recovery-key');
    await recoveryPage.getByLabel('Email address').fill(email);
    await recoveryPage.getByLabel('Recovery key').fill(rawRecoveryKey);
    await recoveryPage.getByLabel('New password', { exact: true }).fill(recoveredPassword);
    await recoveryPage.getByLabel('Confirm new password').fill(recoveredPassword);
    await recoveryPage.getByRole('button', { name: 'Recover account' }).click();
    await expect(recoveryPage).toHaveURL(/\/login$/u);
    await expect(recoveryPage.getByRole('status')).toContainText('Account recovery completed.');

    await recoveryPage.goto('/recovery-key');
    await recoveryPage.getByLabel('Email address').fill(email);
    await recoveryPage.getByLabel('Recovery key').fill(rawRecoveryKey);
    await recoveryPage.getByLabel('New password', { exact: true }).fill('AcceptanceReplay!890');
    await recoveryPage.getByLabel('Confirm new password').fill('AcceptanceReplay!890');
    await recoveryPage.getByRole('button', { name: 'Recover account' }).click();
    await expect(recoveryPage.getByRole('alert')).toContainText('The recovery credentials are invalid.');
  } finally {
    await recoveryContext.close();
  }

  await page.goto('/account');
  await expect(page).toHaveURL(/\/login$/u);
  await login(page, email, recoveredPassword);
  await page.goto('/account/security');
  const currentSession = page.locator('article.card').filter({ has: page.getByRole('heading', { name: 'Current session' }) });
  await currentSession.getByRole('button', { name: 'Revoke and sign out' }).click();
  await expect(page).toHaveURL(/\/login$/u);
  await expect(page.getByRole('status')).toContainText('This session has been revoked.');
});

test('@portal-account account security localization covers Polish responsive, validation, authorization and expired-token states', async ({ browser, page }) => {
  const email = uniqueEmail('account-security-pl');
  const password = 'AcceptanceSecurityPl!234';

  await register(page, email, password);
  await login(page, email, password);
  await page.goto('/account/security?locale=pl');
  await expect(page.locator('html')).toHaveAttribute('lang', 'pl');
  await expect(page.getByRole('heading', { name: 'Bezpieczeństwo i cykl życia' })).toBeVisible();
  await assertResponsiveSecurityPage(page, 'Bezpieczeństwo i cykl życia');

  const primaryEmail = page.locator('section[aria-labelledby="primary-email-heading"]');
  const changedEmail = uniqueEmail('account-security-pl-changed');
  await primaryEmail.getByLabel('Nowy adres e-mail', { exact: true }).fill(changedEmail);
  await primaryEmail.getByLabel('Potwierdź nowy adres e-mail', { exact: true }).fill(changedEmail);
  await primaryEmail.getByLabel('Bieżące hasło').fill('wrong-password');
  await primaryEmail.getByRole('button', { name: 'Wyślij łącza potwierdzające' }).click();
  await expect(page.getByRole('alert')).toContainText('Bieżące hasło jest nieprawidłowe.');

  await page.goto('/email-change/confirm/expired-token?locale=pl');
  await expect(page.getByRole('heading', { name: 'Potwierdź nowy adres e-mail' })).toBeVisible();
  await page.getByRole('button', { name: 'Potwierdź zmianę adresu e-mail' }).click();
  await expect(page.getByRole('alert')).toContainText('Łącze potwierdzające adres e-mail jest nieprawidłowe lub wygasło.');

  const guestContext = await browser.newContext();
  const guestPage = await guestContext.newPage();
  try {
    await guestPage.goto('/account/security?locale=pl');
    await expect(guestPage).toHaveURL(/\/login$/u);
    await expect(guestPage.getByRole('heading', { name: 'Zaloguj się do Oteryn Platform' })).toBeVisible();

    await guestPage.goto('/recovery-key?locale=pl');
    await expect(guestPage.getByRole('heading', { name: 'Odzyskaj konto za pomocą klucza odzyskiwania' })).toBeVisible();
  } finally {
    await guestContext.close();
  }
});
