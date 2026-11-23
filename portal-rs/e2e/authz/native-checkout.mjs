import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';

const pattern = '/native-account/coin-checkout/{order_id}';
export const nativeCheckoutRoutes = [['GET', pattern], ['HEAD', pattern], ['POST', pattern]];

async function coinOrder(owner, state, code) {
  const [row] = await query(`
    WITH quote AS (
      INSERT INTO coin_purchase_quotes
        (id,payer_account_id,package_id,coins,minor_units,currency,exponent,environment,expires_at,consumed_at)
      VALUES(uuidv7(),$1,uuidv7(),1234,250,'PLN',2,'testing',now()+interval '1 minute',now())
      RETURNING id,payer_account_id,package_id,coins,minor_units,currency,exponent,environment
    ), draft AS (SELECT uuidv7() AS id)
    INSERT INTO coin_purchase_orders
      (id,payer_account_id,request_id,quote_id,package_id,coins,minor_units,currency,exponent,environment,
       payment_intent_id,state,code,checkout_url)
    SELECT draft.id,payer_account_id,uuidv7(),quote.id,package_id,coins,minor_units,currency,exponent,environment,
           uuidv7(),$2,$3,CASE WHEN $2='awaiting_payment' THEN $4 || draft.id::text END
    FROM quote CROSS JOIN draft
    RETURNING id
  `, [owner, state, code, new URL('/native-account/coin-checkout/', process.env.PORTAL_E2E_ORIGIN ||
    'https://localhost:18443').href]);
  return row.id;
}

export function closedOrder(owner) {
  return coinOrder(owner, 'failed', 'COIN_ORDER_FAILED');
}

export async function exerciseNativeCheckout(request, locale, token, accountId, restricted, routes, snapshot, verifyRefusal) {
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  const [{ id: escrow }] = await query("SELECT id FROM accounts WHERE email='bazaar-escrow@local.test'");
  const foreign = await closedOrder(escrow);
  const owned = accountId ? await coinOrder(accountId, 'awaiting_payment', 'COIN_ORDER_AWAITING_PAYMENT')
    : foreign;
  const orderState = async id => (await query('SELECT state FROM coin_purchase_orders WHERE id=$1', [id]))[0].state;
  for (const [method, routePath] of nativeCheckoutRoutes) {
    const route = routes.find(r => r.listener === 'public' && r.method === method && r.path === routePath);
    const { guard, source } = route.registrations.find(row => row.profile === 'configured');
    expect(guard.kind).toBe('logged-in');
    for (const [order, ownership] of accountId ? [[owned, true], [foreign, false]] : [[foreign, false]]) {
      await test.step(`${method} ${routePath}${accountId && !ownership ? ' as nonowner' : ''}`, async () => {
        const target = routePath.replace('{order_id}', order);
        if (method === 'POST' && ownership) {
          for (const [description, headers] of [
            ['missing', {}], ['invalid', { 'x-csrf-token': '0'.repeat(64) }],
          ]) {
            await test.step(`owner POST with ${description} CSRF`, async () => {
              const initial = await orderState(order);
              const before = await snapshot();
              const rejected = await request.post(target, {
                form: { action: 'fail' }, headers: { origin, ...headers }, maxRedirects: 0,
              });
              try {
                expect(rejected.status()).toBe(403);
                expect(await orderState(order)).toBe(initial);
                await verifyRefusal(before);
              } finally {
                await rejected.dispose();
              }
            });
          }
          expect(await orderState(order)).toBe('awaiting_payment');
        }
        const blocked = ownership && restricted;
        const before = blocked || (method === 'POST' && !ownership) ? await snapshot() : null;
        const response = await request.fetch(target, {
          method, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
          ...(method === 'POST' && { form: { action: 'fail' } }),
        });
        try {
          const status = !accountId ? 303 : !ownership ? 404 : blocked ? 409 : method === 'POST' ? 303 : 200;
          expect(response.status(), source).toBe(status);
          if (!accountId) expect(response.headers().location).toBe(`/${locale}/login`);
          if (method === 'HEAD') expect((await response.body()).length).toBe(0);
          if (ownership && !blocked && method === 'GET') {
            const html = await response.text();
            expect(/<html\b[^>]*\blang="(pl|en)"/.exec(html)?.[1], "checkout document locale").toBe(locale);
            const heading = locale === 'pl' ? 'Zakup monet w trybie testowym' : 'Sandbox coin checkout';
            expect(html.includes(`<h1>${heading}</h1>`), 'localized checkout heading').toBe(true);
            const payment = locale === 'pl' ? 'Zakończ płatność testową' : 'Complete test payment';
            expect(html.includes(payment), 'awaiting order exposes the localized payment action').toBe(true);
          }
          if (ownership && !blocked && method === 'POST') {
            expect(response.headers().location).toBe(target);
            expect(await orderState(order)).toBe('failed');
            const [intent] = await query(`
              SELECT p.state, o.code FROM coin_purchase_orders o
              JOIN payment_intents p ON p.id=o.payment_intent_id WHERE o.id=$1
            `, [order]);
            expect(intent).toEqual({ state: 'failed', code: 'COIN_ORDER_FAILED' });
          }
          if (blocked) expect(await orderState(order)).toBe('awaiting_payment');
          if (before) await verifyRefusal(before, blocked ? ['coin_purchase.payment_refused'] : []);
        } finally {
          await response.dispose();
        }
      });
    }
  }
}
