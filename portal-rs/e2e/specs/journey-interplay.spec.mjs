import { test, expect } from '../fixtures.mjs';
import { uuid7 } from '../game-sim-helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import {
  clickForm, fill, follow, formPayload, newActor, postForm, provision, submitAndReplay,
} from '../journey-interplay-helpers.mjs';

const world = '0192a000-0000-7000-8000-000000000012';

test('two players trade, report harassment, appeal through support and face staff enforcement', async ({ browser, request }, info) => {
  test.setTimeout(600_000);
  const lang = info.project.metadata.locale;
  const t = (en, pl) => lang === 'pl' ? pl : en;
  const local = suffix => `/${lang}/${suffix}`;
  const actors = [];
  const evidence = `Repeated harassment witnessed during the trade ${uuid7()}`;
  const sellerCharacter = uuid7();
  const buyerCharacter = uuid7();
  const asset = uuid7();
  let seller, buyer, moderator, support, admin, listing, report, sanction, ticket;

  const main = actor => actor.page.locator('main');
  const form = (actor, suffix) => main(actor).locator(`form[action$="/${suffix}"]`);
  const card = actor => main(actor).locator(`[data-listing="${listing}"]`);
  const lastSegment = actor => new URL(actor.page.url()).pathname.split('/').at(-1);
  const heading = async (actor, label) => {
    await expect(actor.page.locator('html')).toHaveAttribute('lang', lang);
    await expect(main(actor).getByRole('heading', { level: 1 })).toHaveText(label);
  };
  const start = async (role, path, mfa) => {
    const identity = await provision(browser, info, role, mfa);
    const actor = { ...identity, ...await newActor(browser, info, identity.state) };
    actors.push(actor);
    expect((await actor.page.goto(path, { waitUntil: 'commit' })).status()).toBe(200);
    await expect(actor.page.locator('html')).toHaveAttribute('lang', lang);
    return actor;
  };

  try {
    await test.step('Five independent sessions enter the portal and their staff queues', async () => {
      seller = await start(null, local('news'), true);
      buyer = await start(null, local('news'));
      moderator = await start('moderator', '/admin/moderation/enforcement');
      support = await start('support', '/admin/moderation/reports');
      admin = await start('platform_admin', '/admin/players');
      await heading(moderator, t('Support and moderation', 'Pomoc i moderacja'));
      await heading(support, t('Support and moderation', 'Pomoc i moderacja'));
      await heading(admin, t('Players', 'Gracze'));
      expect(await query('SELECT name FROM world_status WHERE id=$1', [world]))
        .toEqual([{ name: 'Local Bazaar World' }]);
      expect((await buyer.page.request.get('/admin/moderation/enforcement')).status()).toBe(403);
      expect((await moderator.page.request.get('/admin/moderation/reports')).status()).toBe(403);
      expect((await buyer.page.request.get('/admin/console/audit')).status()).toBe(403);
      expect((await support.page.request.get('/admin/moderation/enforcement')).status()).toBe(403);
      const anonymousReport = await request.get(local('support/reports'), { maxRedirects: 0 });
      expect(anonymousReport.status()).toBe(302);
      expect(anonymousReport.headers().location).toBe(local('login'));
      const anonymousWallet = await request.get(local('shop/history'), { maxRedirects: 0 });
      expect(anonymousWallet.status()).toBe(303);
      expect(anonymousWallet.headers().location).toBe(local('login'));
    });

    await test.step('Both players check the same guilds, houses, highscores and death feeds', async () => {
      const feeds = [
        ['guilds', 'Guild information is unavailable.', 'Informacje o gildiach są niedostępne.'],
        ['houses', 'House information is unavailable. Please try again later.', 'Informacje o domach są niedostępne. Spróbuj ponownie później.'],
        ['rankings/experience', 'Temporarily unavailable. Please try again later.', 'Tymczasowo niedostępne. Spróbuj ponownie później.'],
        ['deaths', 'Death records are currently unavailable.', 'Dane o zgonach są obecnie niedostępne.'],
      ];
      for (const [route, en, pl] of feeds) {
        await Promise.all([seller, buyer].map(async actor => {
          const response = await follow(actor.page, local(route));
          expect(response.status()).toBe(503);
          expect(response.headers()['cache-control']).toContain('no-store');
          await expect(actor.page.locator('html')).toHaveAttribute('lang', lang);
          await expect(main(actor).getByText(t(en, pl), { exact: true })).toBeVisible();
        }));
      }
    });

    await test.step('A lists an item and both players see the Game open the auction', async () => {
      const seeded = await seller.page.request.post('/__bazaar_game/seed', {
        data: {
          seller: seller.id, buyer: buyer.id, kind: 'item', asset_id: asset,
          seller_character_id: sellerCharacter, buyer_character_id: buyerCharacter,
        },
      });
      expect(seeded.status()).toBe(201);
      await follow(seller.page, local('account/bazaar'));
      await follow(seller.page, local('account/bazaar/sell'));
      const create = form(seller, 'sell');
      await expect(create.getByRole('button')).toHaveText(t('List for sale', 'Wystaw na sprzedaż'));
      await expect(create.locator('[name="currency"] option')).toHaveText([
        t('Coins', 'Coiny'), t('Gold', 'Złoto'),
      ]);
      const draft = {
        kind: 'item', world_id: world, seller_character_id: sellerCharacter, asset_id: asset,
        definition_key: 'fixture-sword', starting_bid: '100', publish_snapshot: true,
      };
      await fill(create, draft);
      expect((await postForm(seller.page, local('account/bazaar/sell'), {
        ...await formPayload(create), starting_bid: '0',
      })).status()).toBe(422);
      expect((await clickForm(seller.page, create)).status()).toBe(303);
      await expect(seller.page).toHaveURL(/\/bazaar\/[a-f0-9-]+$/);
      listing = lastSegment(seller);
      await expect.poll(async () => {
        await seller.page.reload();
        return card(seller).getAttribute('data-state');
      }, { timeout: 45_000 }).toBe('active');
      await expect(card(seller).getByRole('status')).toHaveText(t('Open for bids', 'Aukcja otwarta'));
      await follow(buyer.page, local('bazaar'));
      await follow(buyer.page, local(`bazaar/${listing}`));
      await expect(card(buyer).getByRole('status')).toHaveText(t('Open for bids', 'Aukcja otwarta'));
      expect((await postForm(buyer.page, local(`account/bazaar/${listing}/control`), {
        _csrf: (await formPayload(form(buyer, 'bids')))._csrf, action: 'cancel', request_id: uuid7(),
      })).status()).toBe(403);
    });

    await test.step('B pays in the test bank and transfers coins to A exactly once', async () => {
      await follow(buyer.page, local('shop'));
      await follow(buyer.page, local('shop/coins'));
      expect((await clickForm(buyer.page, form(buyer, 'coins'))).status()).toBe(303);
      await expect(buyer.page).toHaveURL(/\/shop\/test-checkout\/[a-f0-9-]+$/);
      await heading(buyer, t('Test bank', 'Bank testowy'));
      const checkout = main(buyer).locator('form');
      await expect(checkout.locator('button')).toHaveText([
        t('Pay (test)', 'Zapłać (test)'), t('Reject payment', 'Odrzuć płatność'), t('Cancel', 'Anuluj'),
      ]);
      const checkoutPath = new URL(buyer.page.url()).pathname;
      expect((await postForm(seller.page, checkoutPath, {
        _csrf: (await formPayload(form(seller, 'control')))._csrf, action: 'pay',
      })).status()).toBe(404);
      expect((await clickForm(buyer.page, checkout, {}, checkout.locator('button[value="pay"]'))).status()).toBe(303);
      await expect(buyer.page).toHaveURL(/\/shop\/coins\?checkout=/);
      await follow(buyer.page, local('shop/history'));
      await expect.poll(async () => {
        await buyer.page.reload();
        return main(buyer).locator('dl dd').first().innerText();
      }, { timeout: 45_000 }).toBe('1234');

      await follow(buyer.page, local('shop/transfer'));
      await heading(buyer, t('Transfer coins', 'Przekaż coiny'));
      const transfer = form(buyer, 'transfer');
      await fill(transfer, { recipient: seller.id, amount: '30' });
      const fields = await formPayload(transfer);
      expect((await postForm(buyer.page, local('shop/transfer'), { ...fields, amount: '0' })).status()).toBe(422);
      const { operation } = await submitAndReplay(buyer.page, transfer, { amount: '31' });
      await expect(main(buyer).locator('dl dd').first()).toHaveText('1204');
      await expect(main(buyer).getByText(operation, { exact: true })).toHaveCount(1);
      await follow(seller.page, local('shop'));
      await follow(seller.page, local('shop/history'));
      await expect(main(seller).locator('dl dd').first()).toHaveText('30');
      await expect(main(seller).getByText(operation, { exact: true })).toHaveCount(1);
    });

    await test.step('B bids once despite concurrent submits and Game delivers the item', async () => {
      await follow(buyer.page, local('bazaar'));
      await follow(buyer.page, local(`bazaar/${listing}`));
      const bid = form(buyer, 'bids');
      await expect(bid.getByRole('button')).toHaveText(t('Place bid', 'Złóż ofertę'));
      await fill(bid, { buyer_character_id: buyerCharacter, maximum: '200' });
      const fields = await formPayload(bid);
      expect((await postForm(buyer.page, local(`account/bazaar/${listing}/bids`), {
        ...fields, maximum: '0',
      })).status()).toBe(422);
      await submitAndReplay(buyer.page, bid, { maximum: '201' });
      expect(await query('SELECT count(*)::int AS count FROM bazaar_bids WHERE listing_id=$1', [listing]))
        .toEqual([{ count: 1 }]);
      await follow(buyer.page, local('shop'));
      await follow(buyer.page, local('shop/history'));
      await expect(main(buyer).locator('dl dd').first()).toHaveText('1004');

      // Advance the disposable deadline as in bazaar.spec.mjs. The Game
      // producer still has to transfer custody and return a real receipt.
      await query("UPDATE bazaar_listings SET ends_at=clock_timestamp()-interval '1 second' WHERE id=$1", [listing]);
      await follow(seller.page, local('account/bazaar'));
      await follow(seller.page, local(`bazaar/${listing}`));
      await expect.poll(async () => {
        await seller.page.reload();
        return card(seller).getAttribute('data-state');
      }, { timeout: 45_000 }).toBe('completed');
      await follow(buyer.page, local('bazaar'));
      await follow(buyer.page, local(`bazaar/${listing}`));
      for (const actor of [seller, buyer]) {
        await expect(card(actor).getByRole('status')).toContainText(t('Sale completed.', 'Sprzedaż zakończona.'));
      }
      const game = await (await buyer.page.request.get(`/__bazaar_game/state?asset=${asset}`)).json();
      expect(game.asset).toMatchObject({ owner: buyer.id, inbox: buyer.id });
      for (const [actor, balance] of [[seller, '120'], [buyer, '1104']]) {
        await follow(actor.page, local('shop'));
        await follow(actor.page, local('shop/history'));
        await expect(main(actor).locator('dl dd').first()).toHaveText(balance);
      }
    });

    await test.step('B filters recruitment in the seeded world and cannot post without a public profile', async () => {
      await follow(buyer.page, local('party'));
      await heading(buyer, t('Looking for a team', 'Szukam drużyny'));
      const filter = main(buyer).locator('form[method="get"]');
      await fill(filter, { world, level: '8', purpose: 'hunt', profession: 'knight' });
      await filter.getByRole('button', { name: t('Filter', 'Filtruj') }).click();
      await expect(buyer.page).toHaveURL(/\/party\?world=/);
      await expect(main(buyer)).toContainText(t('No listings', 'Brak ogłoszeń'));
      const now = Math.floor(Date.now() / 1000);
      const post = form(buyer, 'party');
      const draft = {
        character: buyerCharacter, purpose: 'hunt', profession: 'any', min_level: '1',
        max_level: '100', starts_at: String(now + 60), expires_at: String(now + 3600),
        note: 'Looking for a group in the Bazaar world',
      };
      expect((await clickForm(buyer.page, post, draft)).status()).toBe(403);
      await expect(buyer.page).toHaveURL(/\/party$/);
      await buyer.page.goBack();
      await expect(main(buyer)).toContainText(t('No listings', 'Brak ogłoszeń'));
    });

    await test.step('A records the harassment and sees the submission in private report history', async () => {
      await follow(seller.page, local('support/reports'));
      await heading(seller, t('Reports', 'Zgłoszenia'));
      await expect(main(seller)).toContainText(t('No reports found.', 'Brak zgłoszeń.'));
      await follow(seller.page, local('support/reports/create'));
      const create = form(seller, 'reports');
      const fields = { report_type: 'player', category: 'harassment', target_reference: buyer.id, evidence_summary: evidence };
      await expect(create.getByRole('button')).toHaveText(t('Submit report', 'Wyślij zgłoszenie'));
      expect((await clickForm(seller.page, create, { ...fields, category: 'spam' })).status()).toBe(302);
      await expect(main(seller).getByRole('alert')).toContainText(t('Check the report type', 'Sprawdź rodzaj zgłoszenia'));
      expect((await clickForm(seller.page, create, fields)).status()).toBe(302);
      await expect(seller.page).toHaveURL(/\/support\/reports\/[a-f0-9-]+$/);
      report = lastSegment(seller);
      await expect(main(seller)).toContainText(evidence);
      await expect(main(seller)).toContainText(t('Harassment', 'Nękanie'));
      await expect(main(seller)).toContainText(t('Submitted', 'Wysłane'));
      await follow(seller.page, local('support/reports'));
      await follow(seller.page, local(`support/reports/${report}`));
      await expect(main(seller)).toContainText(evidence);
      expect((await buyer.page.request.get(local(`support/reports/${report}`))).status()).toBe(404);
      await support.page.reload();
      await follow(support.page, `/admin/moderation/reports/${report}`);
      await expect(main(support)).toContainText(evidence);
      await clickForm(support.page, main(support).locator('form:has(input[value="moderate"])'), {
        status: 'reviewing', public_outcome: '', moderator_notes: evidence,
      });
      await seller.page.reload();
      await expect(main(seller)).toContainText(t('Under review', 'W trakcie weryfikacji'));
    });

    await test.step('The moderator issues a restriction and B loses the party posting form', async () => {
      await follow(moderator.page, '/admin/moderation/enforcement/create');
      const now = Math.floor(Date.now() / 1000);
      const create = main(moderator).locator('form:has(input[value="sanction"])');
      await clickForm(moderator.page, create, {
        target: buyer.id, category: 'restriction', status: 'active', public_reason: evidence,
        moderator_notes: `Player report ${report}`, effective_at: String(now - 1), expires_at: String(now + 3600),
      });
      await expect(moderator.page).toHaveURL(/\/moderation\/enforcement\/[a-f0-9-]+$/);
      sanction = lastSegment(moderator);
      await expect(main(moderator)).toContainText(evidence);
      await expect(main(moderator)).toContainText(t('Active', 'Aktywna'));
      const outcome = t('A moderator applied a restriction.', 'Moderator nałożył ograniczenie.');
      await clickForm(support.page, main(support).locator('form:has(input[value="moderate"])'), {
        status: 'actioned', public_outcome: outcome, moderator_notes: `Sanction ${sanction}`,
      });
      await seller.page.reload();
      await expect(main(seller)).toContainText(outcome);
      await expect(main(seller)).toContainText(t('Action taken', 'Podjęto działanie'));
      expect((await buyer.page.reload()).status()).toBe(200);
      await expect(form(buyer, 'party')).toHaveCount(0);
      await expect(main(buyer)).toContainText(t(
        'Posting is unavailable until your account and character ownership can be verified.',
        'Publikacja jest niedostępna do czasu potwierdzenia konta i własności postaci.',
      ));
    });

    await test.step('B asks support to review the restriction and reads the public reply', async () => {
      await follow(buyer.page, local('support/tickets'));
      await heading(buyer, t('Support tickets', 'Zgłoszenia pomocy'));
      await follow(buyer.page, local('support/tickets/create'));
      const subject = `Review request ${sanction}`;
      await clickForm(buyer.page, form(buyer, 'tickets'), { subject, body: evidence });
      await expect(buyer.page).toHaveURL(/\/support\/tickets\/[a-f0-9-]+$/);
      ticket = lastSegment(buyer);
      await expect(main(buyer)).toContainText(subject);
      await expect(main(support).getByRole('link', { name: t('Support tickets', 'Zgłoszenia pomocy') })).toBeVisible();
      await follow(support.page, '/admin/support/tickets');
      await follow(support.page, `/admin/support/tickets/${ticket}`);
      await expect(main(support)).toContainText(evidence);
      const reply = t('Support received your request for review.', 'Pomoc otrzymała prośbę o ponowne rozpatrzenie.');
      await clickForm(support.page, main(support).locator('form:has(input[value="reply"])'), { body: reply });
      await expect(main(support)).toContainText(reply);
      await clickForm(support.page, main(support).locator('form:has(input[value="ticket_status"])'), {
        status: 'resolved',
      });
      await expect(main(support)).toContainText(t('Resolved', 'Rozwiązane'));
      await buyer.page.reload();
      await expect(main(buyer)).toContainText(reply);
      await expect(main(buyer)).toContainText(t('Resolved', 'Rozwiązane'));
      await follow(buyer.page, local('account/sessions'));
      await expect(buyer.page).toHaveURL(/\/account\/sessions$/);
    });

    await test.step('The administrator revokes B’s session and the next protected request requires login', async () => {
      await fill(main(admin).locator('form[method="get"]'), { q: buyer.email });
      await main(admin).locator('form[method="get"] button').click();
      await follow(admin.page, `/admin/players/${buyer.id}`);
      await expect(main(admin)).toContainText(buyer.id);
      const revoke = main(admin).locator('form:has(button[value="logout"])');
      expect((await clickForm(admin.page, revoke, { reason: evidence })).status()).toBe(303);
      await expect(admin.page).toHaveURL(new RegExp(`/admin/players/${buyer.id}$`));
      await buyer.page.reload();
      await expect(buyer.page).toHaveURL(/\/login(?:\?|$)/);
      await expect(main(buyer).locator('input[name="password"]')).toBeVisible();
      await seller.page.reload();
      await expect(main(seller)).toContainText(evidence);
    });

    await test.step('The administrator reads the sanction and logout entries in the audit console', async () => {
      await expect(main(admin).getByRole('link', { name: t('Audit', 'Audyt') })).toBeVisible();
      await follow(admin.page, `/admin/console/audit?lang=${lang}`);
      const filter = main(admin).locator('form[method="get"]');
      for (const [actor, operation, target, count] of [
        [moderator, 'support.enforcement_created', sanction, 1],
        [support, 'support.report_status_changed', report, 2],
        [support, 'support.ticket_replied', ticket, 1],
        [support, 'support.ticket_status_changed', ticket, 1],
        [admin, 'players.logout_all', buyer.id, 1],
      ]) {
        await fill(filter, { admin: actor.id, operation, target });
        await filter.getByRole('button').click();
        await expect(main(admin).getByText(new RegExp(`${actor.id} · ${operation} · ${target}`))).toHaveCount(count);
      }
    });
  } finally {
    await Promise.all(actors.map(actor => actor.context.close()));
  }
});
