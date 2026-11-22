import { test, expect } from '../fixtures.mjs';
import { csrfRejected, pool, submit } from '../helpers.mjs';
import { loginWithRecoveryCode } from '../staff-rbac-helpers.mjs';
import { uuid7 } from '../support/bazaar-game.mjs';
import { postAs, readForm, seedMfa, seedPlayer, signInPlayer } from '../commerce-helpers.mjs';

async function bazaarSeller(page, lang) {
  const seller = await seedPlayer(lang);
  await loginWithRecoveryCode(page, lang, seller, await seedMfa(seller));
  await page.goto(`/${lang}/account/sudo`);
  await submit(page, 'main form', { current_password: seller.password });
  return seller;
}

async function gameLot(page, seller, buyer, kind) {
  const sellerCharacter = uuid7();
  const buyerCharacter = uuid7();
  const asset = kind === 'character' ? sellerCharacter : uuid7();
  const seeded = await page.request.post('/__bazaar_game/seed', { data: {
    seller: seller.id, buyer: buyer.id, kind, asset_id: asset,
    seller_character_id: sellerCharacter, buyer_character_id: buyerCharacter,
  } });
  expect(seeded.status()).toBe(201);
  return {
    asset, buyerCharacter, values: {
      world_id: '0192a000-0000-7000-8000-000000000012',
      seller_character_id: sellerCharacter, asset_id: asset, expected_revision: '7',
      quantity: '1', definition_key: 'fixture-sword', definition_revision: 'v1',
      starting_bid: '10000', duration_days: '1',
    },
  };
}

test('Bazaar seller lists and safely cancels an item after an unfunded bidder is refused', async ({ page, browser }, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const l = lang === 'pl' ? {
    title: 'Bazar', own: 'Twoje aukcje', sell: 'Wystaw na sprzedaż',
    coins: 'Coiny', gold: 'Złoto', item: 'Przedmiot', active: 'Aukcja otwarta',
    bid: 'Złóż ofertę', resume: 'Wznów z aktualnym uprawnieniem',
    cancel: 'Poproś o bezpieczny zwrot', cancelled: 'Anulowano po potwierdzonym bezpiecznym zwrocie',
  } : {
    title: 'Bazaar', own: 'Your listings', sell: 'List for sale',
    coins: 'Coins', gold: 'Gold', item: 'Item', active: 'Open for bids',
    bid: 'Place bid', resume: 'Resume with current authorization',
    cancel: 'Request safe return', cancelled: 'Cancelled after verified safe return',
  };
  const buyerContext = await browser.newContext({ ...info.project.use });
  try {
    const path = `/${lang}/bazaar`;
    const sellURL = `/${lang}/account/bazaar/sell`;
    const guest = await page.request.get(sellURL, { maxRedirects: 0 });
    expect(guest.status()).toBe(303);
    expect(guest.headers().location).toBe(`/${lang}/login`);
    const seller = await bazaarSeller(page, lang);
    const buyerPage = await buyerContext.newPage();
    const buyer = await signInPlayer(buyerPage, lang);
    const { asset, buyerCharacter, values } = await gameLot(page, seller, buyer, 'item');

    await page.goto(path);
    await expect(page.getByRole('heading', { level: 1 })).toHaveText(l.title);
    await page.getByRole('link', { name: l.own, exact: true }).click();
    await expect(page).toHaveURL(new RegExp(`/${lang}/account/bazaar$`));
    await page.getByRole('link', { name: l.sell, exact: true }).click();
    await expect(page).toHaveURL(new RegExp(`${sellURL}$`));
    await expect(page.locator('select[name="currency"] option')).toHaveText([l.coins, l.gold]);
    await page.locator('select[name="kind"]').selectOption('item');
    await page.locator('select[name="currency"]').selectOption('COINS');
    const create = await readForm(page.locator('main form'));
    expect((await postAs(page, create.url, {
      ...create.fields, ...values, duration_days: '15', publish_snapshot: 'true',
    })).status()).toBe(422);
    await submit(page, 'main form', { ...values, publish_snapshot: true });
    await expect(page).toHaveURL(new RegExp(`${path}/[a-f0-9-]+$`));
    const id = new URL(page.url()).pathname.split('/').at(-1);
    const card = page.locator(`article[data-listing="${id}"]`);
    const waitState = state => expect.poll(async () => {
      await page.reload();
      return card.getAttribute('data-state', { timeout: 1500 });
    }, { timeout: 45_000 }).toBe(state);
    await waitState('active');
    await expect(card.getByText(l.active, { exact: true })).toBeVisible();
    await expect(card.getByText(`${l.item} · 10000 ${l.coins}`, { exact: true })).toBeVisible();

    await buyerPage.goto(`${path}/items/${id}`);
    const bid = buyerPage.locator('main form[action$="/bids"]');
    await expect(bid.getByRole('button')).toHaveText(l.bid);
    const denied = buyerPage.waitForNavigation();
    await submit(buyerPage, 'main form[action$="/bids"]', {
      buyer_character_id: buyerCharacter, maximum: '10000',
    });
    expect((await denied).status()).toBe(403);

    const control = card.locator('form[action$="/control"]');
    const resume = await readForm(control);
    const resumeButton = control.getByRole('button', { name: l.resume });
    await csrfRejected(page, resume.url, { ...resume.fields, action: 'resume' });
    await resumeButton.click();
    await expect(card).toHaveAttribute('data-state', 'active');
    expect((await postAs(page, resume.url, { ...resume.fields, action: 'resume' })).status()).toBe(303);
    expect((await postAs(page, resume.url, { ...resume.fields, action: 'cancel' })).status()).toBe(409);
    const cancel = await readForm(control);
    await csrfRejected(page, cancel.url, { ...cancel.fields, action: 'cancel' });
    await control.getByRole('button', { name: l.cancel }).click();
    await waitState('cancelled');
    await expect(card.getByText(l.cancelled, { exact: true })).toBeVisible();
    const game = await (await page.request.get(`/__bazaar_game/state?asset=${asset}`)).json();
    expect(game.asset.owner).toBe(seller.id);
    expect(game.decisions.some(entry => entry.command === 'ReturnBazaarItem' && entry.outcome === 'COMMITTED')).toBe(true);
  } finally {
    await buyerContext.close();
  }
});

test('funded character bid waits for verified unlock and credits the seller once', async ({ page, browser }, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const buyerContext = await browser.newContext({ ...info.project.use });
  const db = pool();
  try {
    const seller = await bazaarSeller(page, lang);
    const buyerPage = await buyerContext.newPage();
    const buyer = await signInPlayer(buyerPage, lang);
    const { asset, buyerCharacter, values } = await gameLot(page, seller, buyer, 'character');
    const funding = uuid7();
    await db.query('INSERT INTO wallet_accounts(account_id) VALUES($1) ON CONFLICT DO NOTHING', [buyer.id]);
    await db.query(
      "INSERT INTO wallet_operations(id,kind,account_id,amount,reason) VALUES($1,'adjustment',$2,15000,'Disposable funded Bazaar bid')",
      [funding, buyer.id],
    );
    await db.query(
      'INSERT INTO wallet_entries(id,operation_id,account_id,available_delta,reserved_delta) VALUES($1,$2,$3,15000,0)',
      [uuid7(), funding, buyer.id],
    );

    await page.goto(`/${lang}/account/bazaar/sell`);
    await page.locator('select[name="kind"]').selectOption('character');
    await submit(page, 'main form', { ...values, publish_snapshot: true });
    await expect(page).toHaveURL(new RegExp(`/${lang}/bazaar/[a-f0-9-]+$`));
    const listing = new URL(page.url()).pathname.split('/').at(-1);
    const card = page.locator(`article[data-listing="${listing}"]`);
    await expect.poll(async () => {
      await page.reload();
      return card.getAttribute('data-state', { timeout: 1500 });
    }, { timeout: 45_000 }).toBe('active');
    await expect(card.getByText(lang === 'pl' ? 'Postać · 10000 Coiny' : 'Character · 10000 Coins', { exact: true })).toBeVisible();

    await buyerPage.goto(page.url());
    const bid = 'main form[action$="/bids"]';
    await submit(buyerPage, bid, { buyer_character_id: buyerCharacter, maximum: '15000' });
    expect((await db.query('SELECT amount,completed FROM bazaar_coin_holds WHERE id=$1 AND account_id=$2', [listing, buyer.id])).rows)
      .toEqual([{ amount: '15000', completed: false }]);
    // Advance only this disposable auction after its real browser bid.
    await db.query("UPDATE bazaar_listings SET ends_at=clock_timestamp()-interval '1 second' WHERE id=$1", [listing]);
    const completed = lang === 'pl'
      ? 'Sprzedaż zakończona. Kupujący otrzymał obiekt, a sprzedający otrzymał środki.'
      : 'Sale completed. The buyer received the asset and the seller was credited.';
    await expect.poll(async () => {
      await buyerPage.reload();
      return buyerPage.locator(`article[data-listing="${listing}"] [role="status"]`).innerText({ timeout: 1500 });
    }, { timeout: 45_000 }).toBe(completed);
    const sale = (await db.query('SELECT id FROM bazaar_settlements WHERE listing_id=$1', [listing])).rows[0].id;
    expect((await db.query(
      "SELECT count(*)::int AS count FROM wallet_operations WHERE reference_id=$1 AND kind='capture'", [sale],
    )).rows[0].count).toBe(1);
    expect((await db.query('SELECT sum(available_delta)::bigint AS credit FROM wallet_entries WHERE account_id=$1', [seller.id])).rows)
      .toEqual([{ credit: '9000' }]);
    const game = await (await buyerPage.request.get(`/__bazaar_game/state?asset=${asset}`)).json();
    expect(game.asset.owner).toBe(buyer.id);
    expect(game.asset.locked).toBe(false);
  } finally {
    await buyerContext.close();
    await db.end();
  }
});
