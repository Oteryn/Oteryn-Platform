import { test, expect } from '../fixtures.mjs';
import { pool, submit } from '../helpers.mjs';
import { query, seededAccount, seededLogin } from '../staff-rbac-helpers.mjs';
import { messages } from '../public-depth-helpers.mjs';

async function completedSale(seller, buyer, price) {
  const db = pool();
  const client = await db.connect();
  try {
    await client.query('BEGIN');
    const { rows: [{ id }] } = await client.query(
      `INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,state,ends_at)
       VALUES(uuidv7(),$1,'item','fixed_price','coins',$2,false,'completed',now()) RETURNING id`,
      [seller, price],
    );
    await client.query(
      `INSERT INTO bazaar_settlements(id,listing_id,buyer,seller,currency,amount,commission,seller_net,reservation,idempotency_key,completed_at)
       VALUES(uuidv7(),$1,$2,$3,'coins',$4,$4::bigint/10,$4::bigint-$4::bigint/10,uuidv7(),$1::uuid::text,now())`,
      [id, buyer, seller, price],
    );
    await client.query('COMMIT');
    return id;
  } finally {
    client.release();
    await db.end();
  }
}

test('completed sales, saved searches and Bazaar account controls', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const market = `/${lang}/bazaar/history`;
  const own = `/${lang}/account/bazaar/history`;
  const seller = await seededAccount(lang);
  const buyer = await seededAccount(lang);
  const shopper = await seededAccount(lang);
  const options = {
    baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, viewport: info.project.use.viewport,
  };
  const sellerContext = await browser.newContext(options);
  const buyerContext = await browser.newContext(options);
  try {
    await seededLogin(page, lang, shopper);
    await page.goto(`/${lang}/bazaar`);
    await page.locator(`a[href="${market}"]`).click();
    await expect(page).toHaveURL(new URL(market, info.project.use.baseURL).href);
    await page.locator(`main a[href="${own}"]`).click();
    await submit(page, 'form[data-action="enable-alerts"]');
    // Keep each project isolated while all four exercise the same database.
    const price = 735100 + ['pl-1440', 'pl-390', 'en-1440', 'en-390'].indexOf(info.project.name);
    await page.goto(market);
    await page.locator('form[data-filter] input[name=min_price]').fill(String(price));
    await page.locator('form[data-filter] input[name=max_price]').fill(String(price));
    await page.locator('form[data-filter] button').click();
    await submit(page, 'form[data-action="save"]', { name: 'My <search>' });
    await expect(page.locator('main')).toContainText('My <search>');
    const id = await completedSale(seller.id, buyer.id, price);
    expect((await page.request.get(`${own}?sale=${id}`)).status()).toBe(404);
    const csrf = await page.locator('form[data-action="disable-alerts"] input[name="_csrf"]').inputValue();
    const denied = await page.request.post(own, {
      headers: { origin: new URL(page.url()).origin },
      form: { action: 'share', sale: id, _csrf: csrf },
      maxRedirects: 0,
    });
    expect(denied.status()).toBe(404);
    expect((await page.request.get(`${market}?min_price=-1`)).status()).toBe(422);
    expect((await page.request.get(`${market}?from=2026-02-30`)).status()).toBe(422);
    const sellerPage = await sellerContext.newPage();
    const buyerPage = await buyerContext.newPage();
    for (const [actorPage, account] of [[sellerPage, seller], [buyerPage, buyer]]) {
      await seededLogin(actorPage, lang, account);
      expect((await actorPage.goto(`${own}?sale=${id}`)).status()).toBe(200);
      await expect(actorPage.locator('main')).toContainText(String(price));
      await submit(actorPage, 'form[data-action="share"]');
    }
    await page.goto(own);
    const alerts = page.locator('[data-alert]');
    await expect(alerts).toHaveCount(1);
    await page.getByRole('link', { name: 'My <search>', exact: true }).click();
    await expect(page.locator('article[data-sale]')).toHaveCount(1);
    await page.goto(own);
    await alerts.getByRole('link').click();
    const published = page.locator('article[data-sale]');
    await expect(published).toHaveCount(1);
    await expect(published).toContainText(String(price));
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
    for (const privateValue of [id, seller.id, buyer.id]) {
      await expect(page.locator('main')).not.toContainText(privateValue);
    }
    await submit(page, 'form[data-action="watch"]');
    await expect(page.locator('[data-watched]')).toHaveCount(1);
    await expect(page.locator('[data-alert]')).toHaveCount(1);
    await submit(page, 'form[data-action="dismiss"]');
    await expect(page.locator('[data-alert]')).toHaveCount(0);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
    expect((await query('SELECT count(*)::int AS n FROM bazaar_history_watches WHERE account=$1', [shopper.id]))[0].n).toBe(1);
    // Revoking consent also removes the sale from watches.
    await submit(sellerPage, 'form[data-action="unshare"]');
    await page.reload();
    await expect(page.locator('[data-watched]')).toHaveCount(0);
  } finally {
    await Promise.all([sellerContext.close(), buyerContext.close()]);
  }
});

test('i18n-parity for Bazaar history', () => {
  const en = messages('crates/bazaar/src/history', 'en');
  const pl = messages('crates/bazaar/src/history', 'pl');
  expect(Object.keys(pl).sort()).toEqual(Object.keys(en).sort());
  expect(en['history-title']).toBe('Bazaar history');
  expect(pl['history-title']).toBe('Historia bazaru');
});
