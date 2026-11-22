import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';

export const shopRoutes = [
  ['POST', '/{lang}/shop'],
  ['POST', '/{lang}/shop/coins'],
  ['GET', '/{lang}/shop/test-checkout/{intent}'],
  ['POST', '/{lang}/shop/test-checkout/{intent}'],
  ['POST', '/{lang}/shop/transfer'],
  ['GET', '/{lang}/panel/payments/receipts/{id}'],
];
const fixture = '019a541b-a000-7000-8000-000000000001';

export async function exerciseShop(request, locale, token, accountId, restricted, routes, snapshot, verifyRefusal) {
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  const [{ id: recipient }] = await query("SELECT id FROM accounts WHERE email='bazaar-escrow@local.test'");
  const uuid = async () => (await query('SELECT uuidv7() AS id'))[0].id;
  const topUp = await uuid();
  const transfer = await uuid();
  const walletAllowed = Boolean(accountId) && !restricted;
  let intent = fixture;
  let receipt = fixture;

  if (walletAllowed) {
    await query('INSERT INTO wallet_accounts(account_id) VALUES($1) ON CONFLICT DO NOTHING', [accountId]);
    await query(`
      WITH adjustment AS (
        INSERT INTO wallet_operations(id,kind,account_id,amount,reason)
        VALUES(uuidv7(),'adjustment',$1,20,'Disposable authorization fixture') RETURNING id,account_id
      )
      INSERT INTO wallet_entries(id,operation_id,account_id,available_delta,reserved_delta)
      SELECT uuidv7(),id,account_id,20,0 FROM adjustment
    `, [accountId]);
  } else if (accountId) {
    const [invoice] = await query(`
      WITH intent AS (
        INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state)
        VALUES(uuidv7(),$1,'authz-local',uuidv7()::text,250,'PLN','succeeded')
        RETURNING id,account_id,amount,currency
      )
      INSERT INTO invoices(id,intent_id,account_id,amount,currency)
      SELECT uuidv7(),id,account_id,amount,currency FROM intent RETURNING id
    `, [accountId]);
    receipt = invoice.id;
  }
  const forms = {
    '/{lang}/shop': { operation: await uuid(), character: fixture, world: fixture, revision: '1',
      offer: 'local-fixture', price: '1', quantity: '1' },
    '/{lang}/shop/coins': { operation: topUp, package: 'e2e-coins' },
    '/{lang}/shop/test-checkout/{intent}': { action: 'pay' },
    '/{lang}/shop/transfer': { operation: transfer, recipient, amount: '1' },
  };
  for (const [method, path] of shopRoutes) {
    const { guard, source } = routes.find(r => r.listener === 'public' && r.method === method && r.path === path)
      .registrations[0];
    const purchasing = path === '/{lang}/shop';
    const receiving = path.endsWith('/receipts/{id}');
    const mutation = method === 'POST';
    const allowed = purchasing || (receiving ? Boolean(accountId) : walletAllowed);
    const status = purchasing ? 503 : allowed ? mutation ? 303 : 200 : accountId ? 403 : 303;
    const url = path.replace('{lang}', locale).replace('{intent}', intent).replace('{id}', receipt);
    await test.step(`${method} ${path}`, async () => {
      expect(guard.kind).toBe(purchasing ? 'anonymous' : 'logged-in');
      const before = mutation && !allowed ? await snapshot() : null;
      const response = await request.fetch(url, {
        method, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
        ...(mutation && { form: { ...forms[path], _csrf: token } }),
      });
      try {
        expect(response.status(), source).toBe(status);
        if (before) await verifyRefusal(before);
        if (!allowed && !accountId) expect(response.headers().location).toBe(`/${locale}/login`);
        if (allowed && path.endsWith('/coins')) {
          const [row] = await query('SELECT payment_intent_id FROM store_web_topups WHERE operation_id=$1 AND account_id=$2',
            [topUp, accountId]);
          intent = row.payment_intent_id;
          expect(response.headers().location).toBe(`/${locale}/shop/test-checkout/${intent}`);
        }
        if (allowed && mutation && path.endsWith('/test-checkout/{intent}')) {
          const [row] = await query('SELECT id FROM invoices WHERE intent_id=$1 AND account_id=$2', [intent, accountId]);
          receipt = row.id;
          expect(response.headers().location).toBe(`/${locale}/shop/coins?checkout=${intent}`);
        }
        if (allowed && path.endsWith('/transfer')) {
          expect((await query('SELECT 1 FROM wallet_transfers WHERE id=$1 AND sender_id=$2 AND recipient_id=$3',
            [transfer, accountId, recipient])).length).toBe(1);
        }
      } finally {
        await response.dispose();
      }
    });
  }
}
