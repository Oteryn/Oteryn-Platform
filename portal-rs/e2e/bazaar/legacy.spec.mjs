import { execFileSync } from 'node:child_process';
import { test, expect } from '../fixtures.mjs';
import { csrfRejected, login, password, submit } from '../helpers.mjs';
import { loginWithRecoveryCode, query } from '../staff-rbac-helpers.mjs';
import { listing, post, uuid7 } from '../game-sim-helpers.mjs';

const legacyActions = ['renew', 'cancel', 'bid', 'accept', 'reject', 'offer', 'publish', 'buy-now'];

function seed(entity, payload) {
  const binary = process.env.PORTAL_E2E_BINARY;
  if (!binary) throw new Error('Set PORTAL_E2E_BINARY to the local oteryn-portal binary');
  try {
    return JSON.parse(execFileSync(binary, ['seed', entity, JSON.stringify(payload)], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    }));
  } catch (error) {
    // The command arguments and MFA result contain disposable credentials.
    throw new Error(`Seed CLI failed for ${entity} (${error.code ?? error.status ?? 'invalid result'})`);
  }
}

async function seedPlayer(page, lang, { mfa = false } = {}) {
  const id = uuid7();
  const account = { email: `bazaar-${id}@example.invalid`, password: password() };
  seed('account', { ...account, accountId: id, locale: lang });
  if (mfa) {
    const { recoveryCodes: [code] } = seed('account-mfa', { accountId: id, password: account.password });
    await loginWithRecoveryCode(page, lang, account, code);
    await page.goto(`/${lang}/account/sudo`);
    await submit(page, 'main form', { current_password: account.password });
  } else {
    await login(page, lang, account);
  }
  return { page, id };
}

function legacyPosts(lang, id) {
  return legacyActions.map(name => `/${lang}/bazaar/items/${id}/${name}`);
}

async function send(page, path, form) {
  return page.request.post(path, {
    form,
    headers: { origin: new URL(page.url()).origin },
    maxRedirects: 0,
  });
}

test('Game runtime replaces the legacy item forms', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const otherContext = await browser.newContext({
    ignoreHTTPSErrors: true,
    locale: lang,
    viewport: info.project.use.viewport,
    baseURL: process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443',
  });
  try {
    const seller = await seedPlayer(page, lang, { mfa: true });
    const buyer = await seedPlayer(await otherContext.newPage(), lang);

    const lot = await listing(seller, buyer, lang);
    const itemIndex = `/${lang}/bazaar/items`;
    const create = `/${lang}/account/bazaar/sell`;
    expect((await buyer.page.goto(itemIndex)).status()).toBe(200);
    await expect(buyer.page.locator(`[data-listing="${lot.id}"] h2 a`)).toHaveAttribute('href', lot.url);
    await expect(buyer.page.locator(`a[href="${itemIndex}/new"]`)).toHaveCount(0);
    await expect(buyer.page.locator(`a[href="${create}"]`)).toHaveCount(1);
    expect((await buyer.page.goto(`${itemIndex}/new`)).status()).toBe(200);
    await expect(buyer.page.locator('main form')).toHaveAttribute('action', create);

    const item = `${itemIndex}/${lot.id}`;
    const bids = `/${lang}/account/bazaar/${lot.id}/bids`;
    const control = `/${lang}/account/bazaar/${lot.id}/control`;
    expect((await buyer.page.goto(item)).status()).toBe(200);
    expect((await page.goto(item)).status()).toBe(200);
    for (const [viewer, target] of [[page, control], [buyer.page, bids]]) {
      await expect(viewer.locator(`main form[action="${target}"]`)).toHaveCount(1);
      await expect(viewer.locator(`main form[action^="${itemIndex}"]`)).toHaveCount(0);
      const token = await viewer.locator('main input[name=_csrf]').inputValue();
      const fields = { request_id: uuid7(), _csrf: token };
      const createResponse = await send(viewer, itemIndex, fields);
      expect(createResponse.status(), `POST ${itemIndex}`).toBe(405);
      for (const path of legacyPosts(lang, lot.id)) {
        expect((await send(viewer, path, fields)).status(), `POST ${path}`).toBe(404);
      }
    }

    const invalid = await buyer.page.request.get(`${itemIndex}/malformed`);
    expect(invalid.status()).toBe(400);
    expect((await buyer.page.request.get(`${itemIndex}/${uuid7()}`)).status()).toBe(404);
    expect((await post(buyer.page, control, { request_id: uuid7(), action: 'cancel' })).status()).toBe(403);
    expect((await post(page, control, { request_id: uuid7(), action: 'renew' })).status()).toBe(422);
    expect(await query('SELECT state FROM bazaar_listings WHERE id=$1', [lot.id])).toEqual([{ state: 'active' }]);
    expect(await query('SELECT id FROM bazaar_bids WHERE listing_id=$1', [lot.id])).toEqual([]);
    expect(await query('SELECT id FROM bazaar_offers WHERE listing_id=$1', [lot.id])).toEqual([]);

    await csrfRejected(page, control, { request_id: uuid7(), action: 'cancel' });
    await page.locator(`form[action="${control}"] button[value="cancel"]`).click();
    await expect.poll(async () => {
      await page.reload();
      return page.locator(`[data-listing="${lot.id}"]`).getAttribute('data-state');
    }, { timeout: 45_000 }).toBe('cancelled');
    expect((await buyer.page.request.get(item)).status()).toBe(404);
  } finally {
    await otherContext.close();
  }
});

test('fallback mounts the legacy POSTs as unavailable', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const itemIndex = `/${lang}/bazaar/items`;
  const message = lang === 'pl' ? 'Handel przedmiotami jest tymczasowo niedostępny.'
    : 'Item trading is temporarily unavailable.';

  const listingId = uuid7();
  for (const path of [itemIndex, `${itemIndex}/new`, `${itemIndex}/${listingId}`]) {
    expect((await page.goto(path)).status(), `GET ${path}`).toBe(503);
    await expect(page.locator('main')).toContainText(message);
    await expect(page.locator('main form')).toHaveCount(0);
    await expect(page.locator(`main a[href^="${itemIndex}"]`)).toHaveCount(0);
  }

  const requestId = uuid7();
  for (const path of [itemIndex, ...legacyPosts(lang, listingId)]) {
    const response = await send(page, path, { request_id: requestId });
    expect(response.status(), `POST ${path}`).toBe(503);
    expect(response.headers()['cache-control']).toContain('no-store');
    expect(await response.text()).toContain(message);
  }
  expect((await send(page, `/${lang}/bazaar/items/malformed/unknown`, {})).status()).toBe(503);
  expect((await send(page, '/xx/bazaar/items', {})).status()).toBe(404);
  expect(await query('SELECT request_id FROM bazaar_item_requests WHERE request_id=$1', [requestId])).toEqual([]);
});
