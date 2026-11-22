import { createHmac, randomUUID } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect } from '../fixtures.mjs';
import { password, waitForLoginBudget } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';

export const portal = fileURLToPath(new URL('../../', import.meta.url));
export const parallel = 24;

export function portalCommand(args) {
  const database = new URL(process.env.PORTAL_E2E_DATABASE_URL);
  if (!['localhost', '127.0.0.1', '[::1]'].includes(database.hostname) ||
      !database.pathname.startsWith('/portal_e2e_')) {
    throw new Error('Fixture CLI requires a disposable local E2E database');
  }
  const binary = process.env.PORTAL_E2E_BINARY ||
    path.resolve(portal, process.env.CARGO_TARGET_DIR || 'target', 'debug/oteryn-portal');
  try {
    return execFileSync(binary, args, {
      cwd: portal,
      env: { ...process.env, OTERYN_ENV: 'testing', OTERYN_DATABASE_URL: database.href },
      stdio: ['ignore', 'pipe', 'pipe'],
      encoding: 'utf8',
    }).trim();
  } catch {
    throw new Error(`Fixture CLI failed: ${args[0]} ${args[1]}`);
  }
}

export function seedAccount(lang) {
  const account = { email: `money-${randomUUID()}@example.invalid`, password: password() };
  const output = portalCommand(['seed', 'account', JSON.stringify({ ...account, locale: lang })]);
  return { ...account, id: JSON.parse(output).accountId };
}

export async function formAt(page, url) {
  expect((await page.goto(url, { waitUntil: 'domcontentloaded' })).status()).toBe(200);
  const form = page.locator(`main form[action="${url}"]`).first();
  await expect(form).toBeVisible();
  return form.evaluate(element => Object.fromEntries(new FormData(element)));
}

export async function postForm(page, url, form) {
  const response = await page.request.post(url, {
    form,
    headers: { origin: new URL(page.url()).origin },
    maxRedirects: 0,
  });
  const result = { status: response.status(), location: response.headers().location };
  await response.dispose();
  return result;
}

// Seeded actors begin in a fresh browser context. Use its rendered sign-in form
// and let the real response set that context's session cookie.
export async function loginSeeded(page, lang, account) {
  const url = `/${lang}/login`;
  const form = await formAt(page, url);
  await waitForLoginBudget();
  const { status } = await postForm(page, url, {
    ...form, email: account.email, password: account.password,
  });
  expect(status).toBe(302);
}

export async function burst(action) {
  return Promise.all(Array.from({ length: parallel }, (_, index) => action(index)));
}

export function expectStatuses(results, counts) {
  const expected = Object.entries(counts).flatMap(([status, count]) => Array(count).fill(Number(status)));
  expect(results.map(({ status }) => status).sort()).toEqual(expected.sort());
}

export async function topUp(page, lang) {
  const url = `/${lang}/shop/coins`;
  const form = await formAt(page, url);
  const response = await postForm(page, url, form);
  expect(response.status).toBe(303);
  const rows = await query(
    `SELECT t.payment_intent_id AS id, p.provider_reference, p.state
     FROM store_web_topups t JOIN payment_intents p ON p.id=t.payment_intent_id
     WHERE t.operation_id=$1`,
    [form.operation],
  );
  expect(rows).toHaveLength(1);
  expect(response.location).toBe(`/${lang}/shop/test-checkout/${rows[0].id}`);
  expect(rows[0].state).toBe('pending');
  return rows[0];
}

export function paymentEvent(intent, state = 'succeeded') {
  return {
    event_id: randomUUID(),
    intent_id: intent.id,
    provider_reference: intent.provider_reference,
    amount: { minor_units: 250, currency: 'PLN' },
    state,
  };
}

export async function webhook(page, event) {
  const data = JSON.stringify(event);
  const signature = createHmac('sha256', process.env.OTERYN_SHOP_TEST_PAYMENT_KEY)
    .update(data).digest('base64');
  const response = await page.request.post('/__test/payments/webhook', {
    data,
    headers: { 'content-type': 'application/json', 'x-test-payment-signature': signature },
  });
  const status = response.status();
  await response.dispose();
  return { status };
}

export async function creditEvidence(id, account) {
  await expect.poll(() => query(
    "SELECT state FROM outbox WHERE kind='wallet.top_up' AND idempotency_key=$1", [id],
  ), { timeout: 30_000 }).toEqual([{ state: 'done' }]);
  expect(await query(
    'SELECT account_id, available_delta, reserved_delta FROM wallet_entries WHERE operation_id=$1', [id],
  )).toEqual([{ account_id: account, available_delta: '1234', reserved_delta: '0' }]);
  expect(await query('SELECT account_id FROM invoices WHERE intent_id=$1', [id]))
    .toEqual([{ account_id: account }]);
}

export async function balance(page, lang, account, available, reserved = 0) {
  const expected = { available: String(available), reserved: String(reserved) };
  expect(await query(
    `SELECT coalesce(sum(available_delta),0)::text AS available,
            coalesce(sum(reserved_delta),0)::text AS reserved
     FROM wallet_entries WHERE account_id=$1`, [account],
  )).toEqual([expected]);
  expect((await page.goto(`/${lang}/shop/history`, { waitUntil: 'domcontentloaded' })).status()).toBe(200);
  await expect(page.locator('main dl dd')).toHaveText([expected.available, expected.reserved]);
}
