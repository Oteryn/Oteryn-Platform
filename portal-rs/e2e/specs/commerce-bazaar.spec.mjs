import { test, expect } from '../fixtures.mjs';
import { csrfRejected, submit } from '../helpers.mjs';
import { loginWithRecoveryCode } from '../staff-rbac-helpers.mjs';
import { uuid7 } from '../support/bazaar-game.mjs';
import { postAs, readForm, seedMfa, seedPlayer, signInPlayer } from '../commerce-helpers.mjs';

test('Bazaar seller lists and safely cancels an item after an unfunded bidder is refused', async ({ page, browser }, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const l = lang === 'pl' ? {
    title: 'Bazar', own: 'Twoje aukcje', sell: 'Wystaw na sprzedaż',
    coins: 'Coiny', gold: 'Złoto', active: 'Aukcja otwarta',
    bid: 'Złóż ofertę', resume: 'Wznów z aktualnym uprawnieniem',
    cancel: 'Poproś o bezpieczny zwrot', cancelled: 'Anulowano po potwierdzonym bezpiecznym zwrocie',
  } : {
    title: 'Bazaar', own: 'Your listings', sell: 'List for sale',
    coins: 'Coins', gold: 'Gold', active: 'Open for bids',
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
    const seller = await seedPlayer(lang);
    await loginWithRecoveryCode(page, lang, seller, await seedMfa(seller));
    await page.goto(`/${lang}/account/sudo`);
    await submit(page, 'main form', { current_password: seller.password });

    const buyerPage = await buyerContext.newPage();
    const buyer = await signInPlayer(buyerPage, lang);
    const asset = uuid7();
    const sellerCharacter = uuid7();
    const buyerCharacter = uuid7();
    const seeded = await page.request.post('/__bazaar_game/seed', { data: {
      seller: seller.id, buyer: buyer.id, kind: 'item', asset_id: asset,
      seller_character_id: sellerCharacter, buyer_character_id: buyerCharacter,
    } });
    expect(seeded.status()).toBe(201);

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
    const values = {
      world_id: '0192a000-0000-7000-8000-000000000012',
      seller_character_id: sellerCharacter, asset_id: asset, expected_revision: '7',
      quantity: '1', definition_key: 'fixture-sword', definition_revision: 'v1',
      starting_bid: '10000', duration_days: '1',
    };
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
