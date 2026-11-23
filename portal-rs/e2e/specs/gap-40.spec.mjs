// Uses the local launcher's disposable outbox database and test-bank worker.
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { test as base, expect } from '../fixtures.mjs';
import { pool } from '../helpers.mjs';
import { uuid7 } from '../game-sim-helpers.mjs';
import { gameJson, gamePeer, portalCommand, seededAccount, seededPlayer } from '../outbox-helpers.mjs';
import { paidTopup } from '../staff-commerce-helpers.mjs';

const outboxEnv = process.env.PORTAL_E2E_OUTBOX_ENV
  ? JSON.parse(await fs.readFile(process.env.PORTAL_E2E_OUTBOX_ENV, 'utf8'))
  : null;
const test = base.extend({
  outboxEnvironment: [async ({}, use) => {
    const previous = Object.fromEntries(Object.keys(outboxEnv ?? {}).map(key => [key, process.env[key]]));
    Object.assign(process.env, outboxEnv);
    try {
      await use();
    } finally {
      for (const [key, value] of Object.entries(previous)) {
        if (value === undefined) delete process.env[key];
        else process.env[key] = value;
      }
    }
  }, { auto: true }],
});
if (outboxEnv) test.use({ baseURL: outboxEnv.PORTAL_E2E_ORIGIN });

test('Premium expiry and receipts after a settled Store purchase', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const polish = lang === 'pl';
  const account = await seededPlayer(page, lang);
  const db = pool();
  const other = await browser.newContext({
    baseURL: process.env.PORTAL_E2E_ORIGIN,
    locale: lang,
    ignoreHTTPSErrors: true,
  });
  try {
    const topup = await paidTopup(page, lang, db);
    const [[invoice], operator] = await Promise.all([
      db.query('SELECT id FROM invoices WHERE intent_id=$1 AND account_id=$2', [topup.payment_intent_id, account.id])
        .then(result => result.rows),
      seededAccount(lang),
    ]);
    expect(invoice).toBeDefined();
    await db.query(`INSERT INTO account_roles(account_id,role_id)
      SELECT $1,id FROM roles WHERE key='platform_admin' AND built_in`, [operator.id]);
    const fixture = new URL('../../../docs/contracts/fixtures/store-price-list-v1/price-list.valid.json', import.meta.url);
    const catalog = JSON.parse(await fs.readFile(fixture, 'utf8'));
    const item = catalog.offers[0];
    const offer = {
      ...item,
      offer_key: 'gap40-premium',
      units: [{ ...item.units[0], product_id: 'oteryn.test_premium', unit_kind: 'PREMIUM_TIME' }],
    };
    catalog.offers.push(offer);
    const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'portal-e2e-premium-'));
    let revision;
    try {
      const file = path.join(directory, 'catalog.json');
      await fs.writeFile(file, JSON.stringify(catalog));
      revision = await portalCommand(['catalog', 'import', file, '--actor', operator.id]);
      expect(await portalCommand(['catalog', 'activate', revision, '--actor', operator.id])).toBe(revision);
    } finally {
      await fs.rm(directory, { recursive: true, force: true });
    }

    await page.goto(`/${lang}/account`);
    await page.locator(`main a[href="/${lang}/panel/premium"]`).click();
    const status = page.locator('main > dl');
    await expect(status).toContainText(polish ? 'Brak premium' : 'No premium');
    const purchase = {
      contract_version: 1,
      operation: 'PurchaseStoreOfferV1',
      source_authority: process.env.PRODUCTS_ENTITLEMENTS_STORE_SOURCE_AUTHORITY,
      purchase_request_id: uuid7(),
      account_id: account.id,
      character_id: uuid7(),
      world_id: uuid7(),
      catalog_revision: revision,
      offer_key: offer.offer_key,
      currency: 'COINS',
      unit_price: offer.unit_price,
      quantity: 1,
    };
    const peer = gamePeer('store');
    const route = '/internal/v1/products-entitlements/store/purchases';
    const result = await gameJson(peer, route, purchase);
    expect(result).toMatchObject({ result: 'ACCEPTED', total_price: offer.unit_price });
    expect(await gameJson(peer, route, purchase)).toEqual(result);
    const grants = await db.query(
      'SELECT effective_until FROM premium_ledger WHERE account_id=$1 AND request_id=$2',
      [account.id, result.units[0].entitlement_id],
    );
    expect(grants.rows).toHaveLength(1);
    const expiry = grants.rows[0].effective_until.toISOString().replace('.000Z', 'Z');

    expect((await page.reload()).status()).toBe(200);
    await expect(status).toContainText(polish ? 'Aktywne' : 'Active');
    await expect(status.locator('time')).toHaveAttribute('datetime', expiry);
    await expect(status).toContainText(polish ? 'Tymczasowo niedostępne' : 'Temporarily unavailable');
    const receiptLink = page.getByRole('link', { name: polish ? 'Potwierdzenie zakupu premium' : 'Premium purchase receipt' });
    const receiptPath = `/${lang}/panel/premium?receipt=${result.order_id}`;
    await expect(receiptLink).toHaveAttribute('href', receiptPath);
    await receiptLink.click();
    await expect(page.locator('main > dl')).toContainText(result.order_id);
    await expect(page.locator('main > dl')).toContainText(`${offer.unit_price} ${polish ? 'monet' : 'coins'}`);
    await expect(page.locator('article time').last()).toHaveAttribute('datetime', expiry);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);

    await page.getByRole('link', { name: polish ? 'Płatności' : 'Payments' }).click();
    const invoicePath = `/${lang}/panel/payments/receipts/${invoice.id}`;
    await page.locator(`main a[href="${invoicePath}"]`).click();
    await expect(page.locator('main article')).toContainText(topup.payment_intent_id);
    await expect(page.locator('main article')).toContainText(polish ? 'Opłacona' : 'Paid');
    const visitor = await other.newPage();
    await seededPlayer(visitor, lang);
    expect((await visitor.goto(receiptPath)).status()).toBe(404);
    expect((await visitor.goto(invoicePath)).status()).toBe(404);
    expect((await page.request.get(`/${lang}/panel/premium?receipt=invalid`)).status()).toBe(400);
  } finally {
    await other.close();
    await db.end();
  }
});
