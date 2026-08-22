import crypto from 'node:crypto';
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

const customerPassword = 'Acceptance-Payments-9!Pass';
const adminPassword = 'Acceptance-Payments-Admin-9!Pass';
const adminRecoveryCode = 'PAYMENT-00001';
const testSecret = 'acceptance-payments-test-secret-not-a-secret';
const syntheticAmountMinor = 1_234;

function seedCustomer(email) {
  return JSON.parse(runBinary('php', [
    'scripts/acceptance/seed-payment-foundation.php',
    email,
    customerPassword,
  ]));
}

function seedAdmin(email) {
  return JSON.parse(runBinary('php', [
    'scripts/acceptance/seed-browser-admin.php',
    email,
    adminPassword,
    adminRecoveryCode,
  ]));
}
function providerPayload(eventId, type, orderPublicId, amountMinor = syntheticAmountMinor, currency = 'PLN') {
  const created = Math.floor(Date.now() / 1000);
  return JSON.stringify({
    id: eventId,
    type,
    created,
    data: {
      order_public_id: orderPublicId,
      currency,
      amount_minor: amountMinor,
      provider_object_reference: null,
    },
  });
}

async function sendProviderEvent(page, payload) {
  const timestamp = Math.floor(Date.now() / 1000);
  const signature = crypto
    .createHmac('sha256', testSecret)
    .update(`${timestamp}.${payload}`)
    .digest('hex');
  return page.request.post('/api/v1/payments/test/events', {
    data: payload,
    headers: {
      'content-type': 'application/json',
      'x-oteryn-test-timestamp': String(timestamp),
      'x-oteryn-test-signature': signature,
    },
  });
}

async function assertNoHorizontalOverflow(page) {
  const dimensions = await page.evaluate(() => ({
    viewport: document.documentElement.clientWidth,
    content: document.documentElement.scrollWidth,
  }));
  expect(dimensions.content).toBeLessThanOrEqual(dimensions.viewport + 1);
}
test.setTimeout(120_000);
test.describe.configure({ retries: 0 });

test.beforeEach(async ({ page }) => {
  page.__acceptanceDiagnostics = installDiagnostics(page);
});

test.afterEach(async ({ page }, testInfo) => {
  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);
});

test('@portal-payments-account owner history browser return and signed test-provider truth', async ({ page }) => {
  const email = uniqueEmail('payments-customer');
  seedCustomer(email);

  await page.goto('/account/payments');
  await expect(page).toHaveURL(/\/login/u);
  await login(page, email, customerPassword);

  await page.goto('/account/payments');
  await expect(page.getByRole('heading', { name: 'Payments', exact: true })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Payment history', exact: true })).toBeVisible();
  await expect(page.getByText('No payment orders have been recorded for this account.')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Non-production checkout exercise' })).toBeVisible();
  await assertNoHorizontalOverflow(page);
  await assertAccessibilitySmoke(page);

  await page.getByLabel('Test currency').selectOption('PLN');
  await page.getByRole('button', { name: 'Create synthetic checkout' }).click();
  await expect(page.getByRole('heading', { name: 'Payment status' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Checkout created / awaiting provider evidence', exact: true })).toBeVisible();

  const returnUrl = page.url();
  const orderPublicId = new URL(returnUrl).pathname.split('/').filter(Boolean).at(-1);
  expect(orderPublicId).toMatch(/^[0-9a-f-]{36}$/u);

  await page.goto(`${returnUrl}?provider_return=succeeded`);
  await expect(page.getByRole('heading', { name: 'Checkout created / awaiting provider evidence', exact: true })).toBeVisible();
  await expect(page.getByText('Succeeded', { exact: true })).toHaveCount(0);
  const successPayload = providerPayload(
    crypto.randomUUID(),
    'payment.succeeded',
    orderPublicId,
  );
  const successResponse = await sendProviderEvent(page, successPayload);
  expect(successResponse.status()).toBe(202);
  expect(await successResponse.json()).toEqual({
    status: 'processed',
    reconciliation_reason: null,
  });

  await page.goto(returnUrl);
  await expect(page.getByText('Succeeded', { exact: true })).toBeVisible();
  await page.getByRole('link', { name: 'Back to payment history' }).click();
  await expect(page.getByText(orderPublicId, { exact: true })).toBeVisible();
  await expect(page.getByText('12.34 PLN', { exact: true })).toBeVisible();
  await expect(page.getByText('Succeeded', { exact: true })).toBeVisible();

  await page.goto('/account/payments?locale=pl');
  await expect(page.getByRole('heading', { name: 'Płatności', exact: true })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Historia płatności', exact: true })).toBeVisible();
  await expect(page.getByText('Zakończona sukcesem', { exact: true })).toBeVisible();
  await assertNoHorizontalOverflow(page);
});

test('@portal-payments-admin exact permission MFA reconciliation review never changes payment state', async ({ page }) => {
  const customerEmail = uniqueEmail('payments-reconciliation-customer');
  seedCustomer(customerEmail);
  await login(page, customerEmail, customerPassword);
  await page.goto('/account/payments');
  await page.getByLabel('Test currency').selectOption('PLN');
  await page.getByRole('button', { name: 'Create synthetic checkout' }).click();

  const returnUrl = page.url();
  const orderPublicId = new URL(returnUrl).pathname.split('/').filter(Boolean).at(-1);
  expect(orderPublicId).toMatch(/^[0-9a-f-]{36}$/u);

  const mismatchPayload = providerPayload(
    crypto.randomUUID(),
    'payment.succeeded',
    orderPublicId,
    syntheticAmountMinor - 1,
  );
  const mismatchResponse = await sendProviderEvent(page, mismatchPayload);
  expect(mismatchResponse.status()).toBe(202);
  expect(await mismatchResponse.json()).toEqual({
    status: 'reconciliation',
    reconciliation_reason: 'settlement_integrity_mismatch',
  });

  const denied = await page.request.get('/admin/payments/reconciliation');
  expect(denied.status()).toBe(403);

  await page.goto('/account/payments');
  await page.getByRole('button', { name: 'Sign out' }).click();

  const adminEmail = uniqueEmail('payments-reconciliation-admin');
  seedAdmin(adminEmail);
  await login(page, adminEmail, adminPassword);
  await completeMfaChallenge(page, adminRecoveryCode);
  await page.goto('/admin/payments/reconciliation');

  await expect(page.getByRole('heading', { name: 'Payment reconciliation', exact: true })).toBeVisible();
  const row = page.getByRole('row').filter({ hasText: orderPublicId });
  await expect(row).toContainText('settlement_integrity_mismatch');
  await expect(row).toContainText('Open');
  await assertNoHorizontalOverflow(page);
  await assertAccessibilitySmoke(page);

  await row.getByRole('button', { name: 'Mark test evidence reviewed' }).click();
  await expect(page.getByRole('status')).toContainText(
    'The reconciliation evidence was marked reviewed without changing payment state.',
  );
  const resolvedRow = page.getByRole('row').filter({ hasText: orderPublicId });
  await expect(resolvedRow).toContainText('Resolved');
  await expect(resolvedRow).toContainText('Reviewed — no payment state change');

  await page.goto('/admin/payments/reconciliation?locale=pl');
  await expect(page.getByRole('heading', { name: 'Uzgadnianie płatności', exact: true })).toBeVisible();

  const foreignReturn = await page.goto(returnUrl);
  expect(foreignReturn?.status()).toBe(404);
});
