import {test, expect} from '../fixtures.mjs';
import {actor, listing, post, uuid7} from '../game-sim-helpers.mjs';
import {query} from '../staff-rbac-helpers.mjs';
import {submit} from '../helpers.mjs';

test('Game-backed buyer can edit the owned profile and seller promotion fails closed', async ({browser}, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const seller = await actor(browser, info, {mfa: true});
  try {
    const buyer = await actor(browser, info);
    try {
      const lot = await listing(seller, buyer, lang);
      const name = `Fixture ${lot.destination.slice(-12)}`;
      const profile = `/account/characters/${encodeURIComponent(name)}/profile`;
      expect((await buyer.page.goto(profile)).status()).toBe(200);
      await submit(buyer.page, 'main form', {comment: 'Local buyer profile.'});
      expect((await buyer.page.goto(profile)).status()).toBe(200);
      await expect(buyer.page.locator('main [name=comment]')).toHaveValue('Local buyer profile.');

      const path = `/${lang}/account/bazaar/${lot.id}/promote`;
      const response = await post(seller.page, path, {
        id: uuid7(), fee_revision: uuid7(), amount: '1',
      });
      expect(response.status()).toBe(403);
      expect(await query('SELECT id FROM bazaar_promotions WHERE listing_id=$1', [lot.id])).toEqual([]);
    } finally {
      await buyer.context.close();
    }
  } finally {
    await seller.context.close();
  }
});

// The configured Game Bazaar owns the item routes. It creates listings via
// /account/bazaar/sell and uses /bids and /control. The old item forms are
// unreachable in the composed portal.
// F-bazaar-46767873991968, R-bazaar-13638167717685 (legacy create).
// F-bazaar-23275926065244, F-bazaar-86148115874390, F-bazaar-231195288605257
// (legacy renew, cancel and publish).
// F-bazaar-140015055205774, F-bazaar-221617554685096, F-bazaar-250676990346223
// (legacy bid, offer and buy-now).
// F-bazaar-160340630540621, F-bazaar-187660078956952 (legacy accept and reject).
// F-bazaar-249147474695525 needs a signed snapshot with an item list. The
// pinned character fixture omits items.
// R-bazaar-137999873732047 has the same shape as the live {listing} route.
// The recorder resolves both to R-bazaar-82908904463903.
