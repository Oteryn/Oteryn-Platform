import { login } from '../helpers.mjs';
import { uuid7 } from '../staff-commerce-helpers.mjs';
import { after, before, expect, one, test } from './fixtures.mjs';
import { authorize, tokens } from './oauth.mjs';

test('expired native checkout quote creates no order or payment intent', async ({ page, lang, db, seed }) => {
  const account = await seed();
  await login(page, lang, account);
  const issued = await tokens(page, await authorize(page, lang, 'game:account-services'));
  const headers = { authorization: `Bearer ${issued.access_token}` };
  const api = '/api/v1/native-account/coin-purchases';
  const quote = async () => {
    const response = await page.request.get(`${api}/catalogue`, { headers });
    expect(response.status()).toBe(200);
    const payload = await response.json();
    expect(payload.sandbox).toBe(true);
    expect(payload.packages).toHaveLength(1);
    const item = payload.packages[0];
    expect(item.valid_for_seconds).toBe(60);
    return item;
  };
  const order = item => page.request.post(`${api}/orders`, {
    headers, data: { version: 1, request_id: uuid7(), quote_id: item.quote_id,
      package_id: item.id, character_id: null },
  });
  const move = (id, seconds) => one(db, `UPDATE coin_purchase_quotes SET expires_at=clock_timestamp()+make_interval(secs=>$3)
    WHERE id=$1 AND payer_account_id=$2 RETURNING id`, [id, account.id, seconds]);
  const state = () => one(db, `SELECT
    (SELECT count(*)::int FROM coin_purchase_orders WHERE payer_account_id=$1) AS orders,
    (SELECT count(*)::int FROM payment_intents WHERE account_id=$1) AS intents`, [account.id]);
  const live = await quote();
  const expired = await quote();

  await move(live.quote_id, before);
  const accepted = await order(live);
  expect(accepted.status()).toBe(200);
  const created = await accepted.json();
  expect(created).toMatchObject({ quote_id: live.quote_id, state: 'awaiting_payment' });
  expect(await state()).toEqual({ orders: 1, intents: 1 });

  await move(expired.quote_id, after);
  const refused = await order(expired);
  expect(refused.status()).toBe(409);
  expect(refused.headers()['cache-control']).toBe('no-store');
  expect(await refused.text()).toBe('');
  expect(await state()).toEqual({ orders: 1, intents: 1 });
  expect(await one(db, 'SELECT consumed_at FROM coin_purchase_quotes WHERE id=$1', [expired.quote_id]))
    .toEqual({ consumed_at: null });

  // The catalogue prunes old unconsumed quotes on access, after the retention hour.
  await one(db, `UPDATE coin_purchase_quotes SET created_at=expires_at-interval '61 minutes'
    WHERE id=$1 AND payer_account_id=$2 RETURNING id`, [expired.quote_id, account.id]);
  await quote();
  expect((await db.query('SELECT id FROM coin_purchase_quotes WHERE id IN ($1,$2)', [live.quote_id, expired.quote_id])).rows)
    .toEqual([{ id: live.quote_id }]);
});
