import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';

const pattern = '/native-account/coin-checkout/{order_id}';
export const nativeCheckoutRoutes = [['GET', pattern], ['POST', pattern]];

async function closedOrder(owner) {
  const [row] = await query(`
    WITH quote AS (
      INSERT INTO coin_purchase_quotes
        (id,payer_account_id,package_id,coins,minor_units,currency,exponent,environment,expires_at,consumed_at)
      VALUES(uuidv7(),$1,uuidv7(),1234,250,'PLN',2,'testing',now()+interval '1 minute',now())
      RETURNING id,payer_account_id,package_id,coins,minor_units,currency,exponent,environment
    )
    INSERT INTO coin_purchase_orders
      (id,payer_account_id,request_id,quote_id,package_id,coins,minor_units,currency,exponent,environment,
       payment_intent_id,state,code)
    SELECT uuidv7(),payer_account_id,uuidv7(),id,package_id,coins,minor_units,currency,exponent,environment,
           uuidv7(),'failed','COIN_ORDER_FAILED' FROM quote
    RETURNING id
  `, [owner]);
  return row.id;
}

export async function exerciseNativeCheckout(request, token, accountId, routes, snapshot, verifyRefusal) {
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  const [{ id: escrow }] = await query("SELECT id FROM accounts WHERE email='bazaar-escrow@local.test'");
  const foreign = await closedOrder(escrow);
  const owned = accountId ? await closedOrder(accountId) : foreign;
  for (const [method, routePath] of nativeCheckoutRoutes) {
    const route = routes.find(r => r.listener === 'public' && r.method === method && r.path === routePath);
    const { guard, source } = route.registrations[0];
    expect(guard.kind).toBe('logged-in');
    for (const [order, ownership] of accountId ? [[owned, true], [foreign, false]] : [[foreign, false]]) {
      await test.step(`${method} ${routePath}${accountId && !ownership ? ' as nonowner' : ''}`, async () => {
        const target = routePath.replace('{order_id}', order);
        const refused = !ownership;
        const before = method === 'POST' && refused ? await snapshot() : null;
        const response = await request.fetch(target, {
          method, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
          ...(method === 'POST' && { form: { action: 'fail' } }),
        });
        try {
          const status = accountId ? ownership ? method === 'GET' ? 200 : 303 : 404 : 303;
          expect(response.status(), source).toBe(status);
          if (!accountId) expect(response.headers().location).toBe('/pl/login');
          if (ownership && method === 'POST') {
            expect(response.headers().location).toBe(target);
            expect((await query('SELECT state FROM coin_purchase_orders WHERE id=$1', [order]))[0].state).toBe('failed');
          }
          if (before) await verifyRefusal(before);
        } finally {
          await response.dispose();
        }
      });
    }
  }
}
