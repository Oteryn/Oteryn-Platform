import { createHmac, randomUUID } from 'node:crypto';
import { test, expect } from '../fixtures.mjs';
import { login, pool, register, submit } from '../helpers.mjs';
import { paidTopup, staffContext, uuid7 } from '../staff-commerce-helpers.mjs';

const webhook = '/api/v1/payments/provider-webhook';

async function fitsViewport(page) {
  expect(await page.locator('main').evaluate(
    main => main.getBoundingClientRect().right <= document.documentElement.clientWidth
      && document.documentElement.scrollWidth <= document.documentElement.clientWidth,
  )).toBe(true);
}

test('payer follows checkout and staff looks up the settlement evidence', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const checkout = `/${lang}/panel/payments/coin-checkout`;
  const pl = lang === 'pl';
  const account = await register(page, lang);
  await login(page, lang, account);
  const staff = await staffContext(browser, info);
  const db = pool();
  try {
    const admin = await staff.newPage();
    await admin.setViewportSize(info.project.use.viewport);
    await admin.setExtraHTTPHeaders({ 'Accept-Language': lang });
    expect((await admin.goto('/admin/commerce')).status()).toBe(200);
    await admin.locator('a[href="#payment-qualification"]').click();
    const title = admin.locator('#payment-qualification');
    await expect(title).toBeVisible();
    const evidence = admin.locator('section[aria-labelledby="payment-qualification"]');
    await expect(evidence).toContainText(pl ? 'oczekuje na wybór dostawcy' : 'provider selection pending');
    await fitsViewport(admin);

    let intent;
    let eventId = '';
    if (!process.env.PORTAL_COIN_PACKAGES && !process.env.PORTAL_PAYMENTS_PROVIDER_WEBHOOK_SECRET) {
      // The standard launcher has the Store test bank but no native sandbox
      // package or webhook signing material.
      expect((await page.goto(checkout)).status()).toBe(503);
      expect((await page.request.post(webhook, { data: '{}' })).status()).toBe(503);
      intent = (await paidTopup(page, lang, db)).payment_intent_id;
    } else {
      // The opt-in rehearsal configures both prerequisites on the stock launcher.
      const secret = process.env.PORTAL_PAYMENTS_PROVIDER_WEBHOOK_SECRET;
      expect(secret?.length).toBeGreaterThanOrEqual(32);
      expect(process.env.PORTAL_COIN_PACKAGES).toBeTruthy();
      expect((await page.goto(`/${lang}/panel/payments`)).status()).toBe(200);
      await page.locator(`main nav a[href="${checkout}"]`).click();
      await expect(page).toHaveURL(new RegExp(`${checkout}$`));
      await fitsViewport(page);
      expect((await page.request.post(checkout, {
        headers: { origin: new URL(page.url()).origin },
        form: { action: 'quote', package: uuid7() },
      })).status()).toBe(403);

      const quote = 'form:has(input[name=action][value=quote])';
      const order = 'form:has(input[name=action][value=order])';
      await submit(page, quote);
      await expect(page.locator(order)).toBeVisible();
      await submit(page, order);
      await expect(page).toHaveURL(/\/native-account\/coin-checkout\/[0-9a-f-]+\?lang=(en|pl)$/);
      const checkoutUrl = page.url();
      const orderId = new URL(checkoutUrl).pathname.split('/').at(-1);
      const { rows: [payment] } = await db.query(
        'SELECT p.id,p.provider_reference,p.amount,p.currency FROM coin_purchase_orders c JOIN payment_intents p ON p.id=c.payment_intent_id WHERE c.id=$1',
        [orderId],
      );
      intent = payment.id;
      const pay = 'form:has(input[name=action][value=pay])';
      await submit(page, pay);
      await expect(page.locator('[data-payment-state]')).toHaveText(pl ? 'Opłacona' : 'Paid');
      await expect(page.locator(pay)).toHaveCount(0);
      expect((await db.query('SELECT count(*)::int AS n FROM wallet_entries WHERE operation_id=$1', [intent])).rows[0].n).toBe(1);

      const refund = {
        version: 1, event_id: randomUUID(), type: 'refund',
        order_reference: payment.provider_reference,
        amount: { minor_units: Number(payment.amount), currency: payment.currency },
      };
      const deliver = (event, signed = true) => {
        const body = JSON.stringify(event);
        const timestamp = String(Math.floor(Date.now() / 1000));
        const signature = createHmac('sha256', secret).update(`${timestamp}.${body}`).digest('hex');
        const headers = { 'content-type': 'application/json' };
        if (signed) {
          headers['oteryn-webhook-timestamp'] = timestamp;
          headers['oteryn-webhook-signature'] = `v1=${signature}`;
        }
        return page.request.post(webhook, { headers, data: body });
      };
      expect((await deliver(refund, false)).status()).toBe(401);
      for (const bad of [
        { order_reference: 'missing-order-reference' },
        { amount: { ...refund.amount, minor_units: refund.amount.minor_units + 1 } },
        { amount: { ...refund.amount, currency: refund.amount.currency === 'USD' ? 'EUR' : 'USD' } },
      ]) {
        expect((await deliver({ ...refund, ...bad, event_id: randomUUID() })).status()).toBe(422);
      }
      for (const replay of [false, true]) {
        const response = await deliver(refund);
        expect(response.status()).toBe(200);
        expect(await response.json()).toMatchObject({ outcome: 'applied', replay });
      }
      expect((await deliver({ ...refund, event_id: randomUUID() })).status()).toBe(422);
      expect((await db.query(
        'SELECT count(*)::int AS n FROM coin_purchase_reversals WHERE order_id=$1 AND outcome=\'debited\'',
        [orderId],
      )).rows[0].n).toBe(1);
      await page.reload();
      await expect(page.locator('[data-payment-state]')).toHaveText(pl ? 'Zwrócona' : 'Refunded');
      await fitsViewport(page);
      eventId = refund.event_id;
    }

    const form = evidence.locator('form[method=get]');
    await form.locator('[name=intent]').fill(intent);
    await form.locator('[name=event]').fill(eventId);
    await form.getByRole('button').click();
    await expect(evidence).toContainText(eventId
      ? (pl ? 'Zastosowano' : 'Applied')
      : (pl ? 'Opłacona' : 'Paid'));
    await expect(evidence).toContainText(pl ? 'Identyfikator rachunku' : 'Receipt ID');
    await fitsViewport(admin);
    expect((await admin.request.get('/admin/commerce?intent=invalid')).status()).toBe(422);
    const refused = await page.request.get(`/admin/commerce?intent=${intent}`, { maxRedirects: 0 });
    expect([303, 403, 404]).toContain(refused.status());
    expect(await refused.text()).not.toContain(intent);
  } finally {
    await staff.close();
    await db.end();
  }
});
