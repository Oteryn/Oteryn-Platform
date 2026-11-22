import { expect, test } from '../fixtures.mjs';
import { submit } from '../helpers.mjs';
import { enrollMfa, loginWithRecoveryCode, query } from '../staff-rbac-helpers.mjs';
import { listing } from '../game-sim-helpers.mjs';
import {
  balance, creditEvidence, expectStatuses, loginSeeded, paymentEvent, postForm, seedAccount, topUp, webhook,
} from './fixtures.mjs';

async function bazaarActor(browser, info, { mfa = false, funded = false } = {}) {
  const lang = info.project.metadata.locale;
  const account = seedAccount(lang);
  const context = await browser.newContext({
    baseURL: info.project.use.baseURL,
    ignoreHTTPSErrors: true,
    locale: lang,
    viewport: info.project.use.viewport,
  });
  const page = await context.newPage();
  try {
    await loginSeeded(page, lang, account);
    if (mfa) {
      await enrollMfa(page, lang, account);
      const recoveryCode = page.locator('main textarea[readonly]').first();
      await expect(recoveryCode).toBeVisible();
      await loginWithRecoveryCode(page, lang, account, await recoveryCode.inputValue());
      expect((await page.goto(`/${lang}/account/sudo`)).status()).toBe(200);
      await submit(page, 'main form', { current_password: account.password });
    }
    if (funded) {
      const intent = await topUp(page, lang);
      expect((await webhook(page, paymentEvent(intent))).status).toBe(204);
      await creditEvidence(intent.id, account.id);
    }
    return { page, context, id: account.id };
  } catch (error) {
    await context.close();
    throw error;
  }
}


test('parallel Bazaar bids fund one hold and settle the auction once', async ({ browser }, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const seller = await bazaarActor(browser, info, { mfa: true });
  try {
    const buyer = await bazaarActor(browser, info, { funded: true });
    try {
      const lot = await listing(seller, buyer, lang);
      const action = `/${lang}/account/bazaar/${lot.id}/bids`;
      expect((await buyer.page.goto(lot.url)).status()).toBe(200);
      const form = await buyer.page.locator(`main form[action="${action}"]`)
        .evaluate(element => Object.fromEntries(new FormData(element)));
      const replies = await Promise.all(Array.from({ length: 20 }, async (_, index) => {
        const maximum = index % 2 === 0 ? '200' : '300';
        return { maximum, ...await postForm(buyer.page, action, {
          ...form, maximum, buyer_character_id: lot.destination,
        }) };
      }));
      const rows = await query(
        'SELECT request_id, buyer, maximum FROM bazaar_authority_bid_requests WHERE listing_id=$1', [lot.id],
      );
      expect(rows).toHaveLength(1);
      const [{ maximum }] = rows;
      expect(['200', '300']).toContain(maximum);
      expect(rows).toEqual([{ request_id: form.request_id, buyer: buyer.id, maximum }]);
      for (const reply of replies) {
        expect(reply.status).toBe(reply.maximum === maximum ? 303 : 409);
      }
      expect(await query('SELECT amount, completed FROM bazaar_coin_holds WHERE id=$1', [lot.id]))
        .toEqual([{ amount: maximum, completed: false }]);
      await balance(buyer.page, lang, buyer.id, 1234 - Number(maximum), Number(maximum));

      // Advance only this fixture's auction deadline after the real funded bid.
      await query("UPDATE bazaar_listings SET ends_at=clock_timestamp()-interval '1 second' WHERE id=$1", [lot.id]);
      await expect.poll(() => query('SELECT state FROM bazaar_listings WHERE id=$1', [lot.id]),
        { timeout: 45_000 }).toEqual([{ state: 'completed' }]);
      const sales = await query(
        'SELECT id, buyer, seller, amount, commission, seller_net FROM bazaar_settlements WHERE listing_id=$1', [lot.id],
      );
      expect(sales).toHaveLength(1);
      expect(sales[0]).toMatchObject({
        buyer: buyer.id, seller: seller.id, amount: '100', commission: '10', seller_net: '90',
      });
      const deliveries = await query(
        "SELECT operation_id FROM bazaar_game_outbox WHERE listing_id=$1 AND command='DeliverBazaarItem'", [lot.id],
      );
      expect(deliveries).toHaveLength(1);
      const gameReply = await seller.page.request.get(`/__bazaar_game/state?asset=${lot.asset}`);
      expect(gameReply.status()).toBe(200);
      const game = await gameReply.json();
      expect(game.asset).toMatchObject({ owner: buyer.id, inbox: buyer.id });
      expect(game.decisions.filter(decision => decision.operation === deliveries[0].operation_id))
        .toEqual([{ operation: deliveries[0].operation_id, command: 'DeliverBazaarItem', outcome: 'COMMITTED' }]);
      expect(await query(
        "SELECT count(*)::int AS count FROM wallet_operations WHERE reference_id=$1 AND kind='capture'", [sales[0].id],
      )).toEqual([{ count: 1 }]);
      await balance(buyer.page, lang, buyer.id, 1134);
      await balance(seller.page, lang, seller.id, 90);
    } finally {
      await buyer.context.close();
    }
  } finally {
    await seller.context.close();
  }
});

test('parallel Bazaar cancellations cannot reuse one key across listings', async ({ browser }, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const seller = await bazaarActor(browser, info, { mfa: true });
  try {
    const viewer = await bazaarActor(browser, info);
    try {
      const lots = [];
      for (let index = 0; index < 2; index++) {
        const lot = await listing(seller, viewer, lang);
        const action = `/${lang}/account/bazaar/${lot.id}/control`;
        const form = await seller.page.locator(`main form[action="${action}"]`)
          .evaluate(element => Object.fromEntries(new FormData(element)));
        lots.push({ ...lot, action, form });
      }
      const ids = lots.map(lot => lot.id);
      const key = lots[0].form.request_id;
      const replies = await Promise.all(Array.from({ length: 20 }, async (_, index) => {
        const lot = lots[index % 2];
        return {
          id: lot.id,
          ...await postForm(seller.page, lot.action, { ...lot.form, request_id: key, action: 'cancel' }),
        };
      }));
      expectStatuses(replies, { 303: 10, 409: 10 });
      const requests = await query(
        'SELECT listing_id, actor, request_id, action FROM bazaar_authority_browser_requests WHERE listing_id=ANY($1::uuid[])',
        [ids],
      );
      expect(requests).toHaveLength(1);
      const [{ listing_id: winner }] = requests;
      expect(ids).toContain(winner);
      expect(requests).toEqual([{ listing_id: winner, actor: seller.id, request_id: key, action: 'cancel' }]);
      for (const reply of replies) expect(reply.status).toBe(reply.id === winner ? 303 : 409);
      const lot = lots.find(entry => entry.id === winner);
      const other = lots.find(entry => entry.id !== winner);
      await expect.poll(() => query('SELECT state FROM bazaar_listings WHERE id=$1', [lot.id]),
        { timeout: 45_000 }).toEqual([{ state: 'cancelled' }]);
      expect(await query('SELECT state FROM bazaar_listings WHERE id=$1', [other.id]))
        .toEqual([{ state: 'active' }]);
      const returns = await query(
        "SELECT listing_id, operation_id FROM bazaar_game_outbox WHERE listing_id=ANY($1::uuid[]) AND command='ReturnBazaarItem'", [ids],
      );
      expect(returns).toHaveLength(1);
      expect(returns[0].listing_id).toBe(lot.id);
      const gameReply = await seller.page.request.get(`/__bazaar_game/state?asset=${lot.asset}`);
      expect(gameReply.status()).toBe(200);
      const game = await gameReply.json();
      expect(game.asset).toMatchObject({ owner: seller.id, consumed: true });
      expect(game.decisions.filter(decision => decision.operation === returns[0].operation_id))
        .toEqual([{ operation: returns[0].operation_id, command: 'ReturnBazaarItem', outcome: 'COMMITTED' }]);
      expect(await query('SELECT id FROM bazaar_settlements WHERE listing_id=ANY($1::uuid[])', [ids])).toEqual([]);
      await balance(seller.page, lang, seller.id, 0);
    } finally {
      await viewer.context.close();
    }
  } finally {
    await seller.context.close();
  }
});
