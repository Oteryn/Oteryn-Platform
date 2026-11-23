import { test, expect } from '../fixtures.mjs';
import { csrfRejected, login, pool } from '../helpers.mjs';
import { messages, seedPlayer } from '../public-depth-helpers.mjs';
import { paidTopup, uuid7 } from '../staff-commerce-helpers.mjs';

test('players buy fixed cards and finish a collection tournament', async ({ page, browser }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const path = `/${lang}/collection-pvp`;
  const labels = messages('crates/minigames/src/collection_pvp', lang);
  const context = await browser.newContext({
    baseURL: process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443',
    ignoreHTTPSErrors: true,
    viewport: info.project.use.viewport,
  });
  const opponent = await context.newPage();
  const db = pool();
  const sku = uuid7();
  const card = uuid7();
  const key = `gap62-${card}`;
  const names = { en: 'Test collection card', pl: 'Testowa karta kolekcji' };
  const form = (scope, operation) => scope.locator(`form:has([name=op][value="${operation}"])`);
  const fields = locator => locator.evaluate(node => Object.fromEntries(new FormData(node)));
  const post = (actor, payload) => actor.request.post(path, {
    form: payload,
    headers: { origin: new URL(actor.url()).origin },
    maxRedirects: 0,
  });
  let temporaryWorld;
  let publishedWorld;
  let fixtureReady = false;

  try {
    const fixture = await db.connect();
    try {
      await fixture.query('BEGIN');
      let { rows: [{ version_id: world }] } = await fixture.query(
        'SELECT version_id FROM world_active_version WHERE singleton FOR UPDATE',
      );
      if (!world) {
        temporaryWorld = world = uuid7();
        await fixture.query(
          "INSERT INTO world_versions(id,source_commit,status) VALUES($1,repeat('a',40),'ready')", [world],
        );
        await fixture.query('UPDATE world_active_version SET version_id=$1 WHERE singleton', [world]);
      }
      publishedWorld = world;
      await fixture.query(
        "INSERT INTO world_entities(version_id,kind,key,data,publishable) VALUES($1,'Creature',$2,$3,true)",
        [world, key, {
          definition: { identity: { family: 'Creature', key }, client_projection: 'ClientSafe' },
          authoring: { profile: { details: { display_name: names.en, bestiary: {} } } },
        }],
      );
      await fixture.query("INSERT INTO world_visibility(kind,key,allowed) VALUES('Creature',$1,true)", [key]);
      await fixture.query(
        "INSERT INTO collection_cards(id,definition,sprite) VALUES($1,$2,decode('01','hex'))",
        [card, {
          id: card, world_version: world, kind: 'Creature', key, race_key: key,
          race_revision: '1', final_kill_threshold: 1, name: names, appearance_id: 1,
        }],
      );
      await fixture.query(
        `INSERT INTO collection_cards(id,definition,sprite)
         SELECT extra.id,jsonb_set(c.definition,'{id}',to_jsonb(extra.id)),c.sprite
         FROM collection_cards c
         CROSS JOIN (SELECT uuidv7() AS id FROM generate_series(1,1000)) extra
         WHERE c.id=$1`,
        [card],
      );
      await fixture.query(
        'INSERT INTO collection_pvp_sets(id,version,price,cards,published) VALUES($1,1,100,$2,true)',
        [sku, { [card]: 2 }],
      );
      await fixture.query('COMMIT');
      fixtureReady = true;
    } finally {
      if (!fixtureReady) await fixture.query('ROLLBACK');
      fixture.release();
    }

    const players = [];
    for (const actor of [page, opponent]) {
      const account = await seedPlayer(lang);
      const { rows: [{ id }] } = await db.query('SELECT id FROM accounts WHERE email=$1', [account.email]);
      await login(actor, lang, account);
      if (info.project.use.viewport.width === 390) {
        await actor.locator('#portal-menu-toggle').click();
      }
      await actor.locator('#portal-world-button').click();
      await actor.locator(`#portal-navigation a[href="${path}"]`).click();
      await expect(actor).toHaveURL(new RegExp(`/${lang}/collection-pvp$`));
      await expect(actor.locator(`#portal-navigation a[href="${path}"]`)).toHaveAttribute('aria-current', 'page');
      await expect(actor.locator('main')).toContainText(labels['journal-unavailable']);
      const offer = actor.locator(`[data-set="${sku}"]`);
      await expect(offer).toContainText(`${names[lang]} × 2`);
      const buy = form(offer, 'buy');
      const pending = await fields(buy);
      await csrfRejected(actor, path, { op: 'buy', id: sku });
      expect((await post(actor, pending)).status()).toBe(403);
      await paidTopup(actor, lang, db);
      await actor.goto(path);
      const purchase = await fields(buy);
      expect((await post(actor, { ...purchase, id: 'invalid' })).status()).toBe(422);
      expect((await post(actor, { ...purchase, id: uuid7() })).status()).toBe(403);
      const [purchaseResponse] = await Promise.all([
        actor.waitForResponse(response =>
          response.request().method() === 'POST' && new URL(response.url()).pathname === path),
        buy.getByRole('button', { name: labels.buy, exact: true }).click(),
      ]);
      expect(purchaseResponse.status()).toBe(303);
      await expect(offer).toContainText(labels.purchased);
      expect((await post(actor, purchase)).status()).toBe(303);
      expect((await post(actor, { ...purchase, request: uuid7() })).status()).toBe(403);
      const { rows: receipts } = await db.query(
        `SELECT p.cards, e.available_delta::int AS debit FROM collection_pvp_purchases p
         JOIN wallet_entries e ON e.operation_id=p.id JOIN wallet_operations o ON o.id=p.id
         WHERE p.account_id=$1 AND p.set_id=$2 AND o.kind='card_set_purchase'`, [id, sku],
      );
      expect(receipts).toEqual([{ cards: { [card]: 2 }, debit: -100 }]);
      const save = form(actor.locator('main'), 'save-deck');
      await save.locator('[name=name]').fill(names[lang]);
      await save.locator('[name=card1]').selectOption(card);
      await save.getByRole('button').click();
      const { rows: [{ id: deck }] } = await db.query(
        'SELECT id FROM collection_pvp_owned_decks WHERE account_id=$1', [id],
      );
      players.push({ actor, id, deck });
    }

    const [left, right] = players;
    const challenge = form(page.locator('main'), 'challenge');
    const draft = await fields(challenge);
    expect((await post(page, { ...draft, opponent: right.id, deck: right.deck })).status()).toBe(403);
    await form(page.locator('main'), 'create').getByRole('button').click();
    const { rows: [{ id: tournament }] } = await db.query(
      'SELECT id FROM collection_pvp_tournaments WHERE organizer=$1', [left.id],
    );
    for (const { actor, deck } of players) {
      await actor.goto(path);
      const join = form(actor.locator(`[data-tournament="${tournament}"]`), 'join');
      await join.locator('[name=deck]').selectOption(deck);
      await join.getByRole('button').click();
    }
    await page.reload();
    await form(page.locator(`[data-tournament="${tournament}"]`), 'start').getByRole('button').click();
    const { rows: [{ id: battle }] } = await db.query(
      'SELECT id FROM collection_pvp_battles WHERE tournament_id=$1', [tournament],
    );
    const play = actor => form(actor.locator(`[data-battle="${battle}"]`), 'play');
    const bad = await fields(play(page));
    expect((await post(page, { ...bad, action: '"teleport"' })).status()).toBe(422);
    for (let round = 0; round < 5; round++) {
      if (!(await play(page).count())) break;
      await play(page).getByRole('button').click();
      await expect(page.locator(`[data-battle="${battle}"]`)).toContainText(labels.waiting);
      await opponent.goto(path);
      await play(opponent).getByRole('button').click();
      await page.reload();
    }
    const tournamentView = page.locator(`[data-tournament="${tournament}"]`);
    await expect(tournamentView).toContainText(labels.completed);
    await tournamentView.getByRole('link', { name: labels['tournament-rankings'] }).click();
    await expect(page.locator('#rankings + p')).toContainText(tournament);
    await expect(page.locator('section[aria-labelledby=rankings] li')).toHaveCount(2);
    await expect.poll(() => page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(
      info.project.use.viewport.width,
    );

    await db.query(
      `INSERT INTO collection_pvp_tournaments(id,organizer,capacity)
       SELECT id,$1,2 FROM unnest($2::uuid[]) AS id`,
      [right.id, Array.from({ length: 51 }, uuid7)],
    );
    await db.query(
      `INSERT INTO collection_pvp_battles(id,left_account,right_account,left_deck)
       SELECT id,$1,$2,'{}'::jsonb FROM unnest($3::uuid[]) AS id`,
      [right.id, left.id, Array.from({ length: 51 }, uuid7)],
    );
    await page.reload();
    await expect(page.locator('section[aria-labelledby=rankings] li')).toHaveCount(2);
    await expect(tournamentView).toHaveCount(0);
    await expect(page.locator(`[data-battle="${battle}"]`)).toHaveCount(0);
    await page.getByRole('link', { name: labels['older-tournaments'] }).click();
    await expect(tournamentView).toContainText(labels.completed);
    await page.getByRole('link', { name: labels['older-battles'] }).click();
    await expect(page.locator(`[data-battle="${battle}"]`)).toBeVisible();
    const additionalSets = Array.from({ length: 11 }, uuid7);
    try {
      await db.query(
        `INSERT INTO collection_pvp_sets(id,version,price,cards,published)
         SELECT id,1,100,$1,true FROM unnest($2::uuid[]) AS id`,
        [{ [card]: 2 }, additionalSets],
      );
      await page.goto(path);
      await expect(page.locator(`[data-set="${sku}"]`)).toHaveCount(0);
      await page.getByRole('link', { name: labels['more-sets'] }).click();
      await expect(page.locator(`[data-set="${sku}"]`)).toContainText(labels.purchased);
    } finally {
      await db.query('UPDATE collection_pvp_sets SET published=false WHERE id=ANY($1)', [additionalSets]);
    }
  } finally {
    try {
      if (fixtureReady) {
        await db.query('UPDATE collection_pvp_sets SET published=false WHERE id=$1', [sku]);
        await db.query("DELETE FROM collection_cards WHERE definition->>'key'=$1", [key]);
        await db.query("DELETE FROM world_visibility WHERE kind='Creature' AND key=$1", [key]);
        await db.query("DELETE FROM world_entities WHERE version_id=$1 AND kind='Creature' AND key=$2", [publishedWorld, key]);
        if (temporaryWorld) {
          await db.query('UPDATE world_active_version SET version_id=NULL WHERE singleton AND version_id=$1', [temporaryWorld]);
        }
      }
    } finally {
      await context.close();
      await db.end();
    }
  }
});

test('i18n-parity for collection battles and fixed sets', () => {
  const directory = 'crates/minigames/src/collection_pvp';
  const en = messages(directory, 'en');
  const pl = messages(directory, 'pl');
  expect(Object.keys(pl).sort()).toEqual(Object.keys(en).sort());
  for (const key of [
    'title', 'unavailable', 'journal-unavailable', 'try-later', 'older-battles',
    'latest-battles', 'older-tournaments', 'latest-tournaments', 'more-sets',
    'latest-sets', 'older-purchases', 'latest-purchases', 'pending', 'registration',
    'completed', 'won', 'lost', 'draw',
  ]) {
    expect(en[key]).toBeTruthy();
    expect(pl[key]).toBeTruthy();
  }
});
