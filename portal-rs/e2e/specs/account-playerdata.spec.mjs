import {test, expect} from '../fixtures.mjs';
import {csrfRejected, pool, submit} from '../helpers.mjs';
import {post, privatePage, refusesAnonymous, seedPlayer, signIn, strings, switchPlayer} from '../account-helpers.mjs';

async function seedHunts(db, account) {
  const {rows: [ids]} = await db.query(
    'SELECT uuidv7() AS character, uuidv7() AS generation, uuidv7() AS first, uuidv7() AS second',
  );
  const ended = Math.floor(Date.now() / 1000) - 60;
  for (const [id, place, experience] of [[ids.first, 'place:forest', 600], [ids.second, 'place:cave', 1200]]) {
    const facts = {
      session_id: id, character_id: ids.character, started_at: ended - 600, ended_at: ended,
      duration_seconds: 600, experience_gained: experience, experience_lost: 20,
      places: {[place]: 600}, loot: {'item:gold': 10}, supplies: {'item:potion': 2}, monsters_killed: {'monster:rat': 3},
      damage_dealt: [{damage_type_key: 'damage:fire', source_kind: 'SELF', source_key: null, amount: 42}],
      damage_taken: [{damage_type_key: 'damage:physical', source_kind: 'MONSTER', source_key: 'monster:rat', amount: 10}],
      healing_done: 12, healing_received: 4, party: null,
    };
    await db.query(
      'INSERT INTO pd_hunt_sessions VALUES ($1,$2,$3,1,$4,$5,$6,$7,false,$8)',
      [account, id, ids.generation, ids.character, facts.started_at, ended, ended + 3600, facts],
    );
    await db.query('INSERT INTO pd_hunt_access VALUES ($1,$2,$3,1,true,true)', [account, id, ids.generation]);
  }
  return ids;
}

test('hunt owner filters, clears, opens, and compares sessions while sharing stays unavailable', async ({page}, info) => {
  test.setTimeout(240_000);
  const {locale: lang} = info.project.metadata;
  const target = `/${lang}/account/hunts`;
  const catalog = `templates/playerdata/${lang}.ftl`;
  const main = page.locator('main');
  await refusesAnonymous(page, target, 303, `/${lang}/login`);
  const player = seedPlayer(lang);
  const foreign = seedPlayer(lang);
  const db = pool();
  let ids;
  try {
    ids = await seedHunts(db, player.id);
  } finally {
    await db.end();
  }

  await signIn(page, lang, foreign);
  await privatePage(page, target);
  await strings(main, catalog, ['empty']);
  await privatePage(page, `${target}/${ids.first}`, 404);
  await privatePage(page, `${target}/compare?ids=${ids.first},${ids.second}`, 404);

  await switchPlayer(page, lang, player);
  await privatePage(page, target);
  await strings(main, catalog, [
    'title', 'derived', 'character', 'place', 'from', 'until', 'party', 'offset', 'apply', 'all',
    'details', 'compare', 'share-disabled', 'split-note', 'session', 'duration', 'experience',
    'loss', 'experience-rate', 'loot', 'supplies', 'kills', 'dealt', 'taken', 'healing-done',
    'healing-received', 'balance', 'unavailable', 'self', 'monster',
  ]);
  await expect(main.locator('article')).toHaveCount(2);
  const filter = main.locator(`form[action="${target}"]`);
  await expect(filter.locator('[name=party] option')).toHaveText(lang === 'pl'
    ? ['Wyczyść filtry', 'Nie', 'Tak'] : ['Clear filters', 'No', 'Yes']);
  await filter.locator('[name=place]').fill('place:forest');
  await filter.locator('[name=party]').selectOption('false');
  await filter.getByRole('button').click();
  await expect(main.locator('article')).toHaveCount(1);
  await expect(main.locator('article')).toContainText(ids.first);
  await filter.getByRole('link').click();
  await expect(main.locator('article')).toHaveCount(2);

  await main.locator(`a[href="${target}/${ids.first}"]`).click();
  await expect(page).toHaveURL(`${target}/${ids.first}`);
  await expect(main.locator('article')).toHaveCount(1);
  await expect(main.locator('article')).toContainText('3600');

  const compare = main.locator(`form[action="${target}/compare"]`);
  await compare.locator('[name=ids]').fill(`${ids.first},${ids.second}`);
  await compare.getByRole('button').click();
  await expect(page).toHaveURL(/\/account\/hunts\/compare\?ids=/);
  await expect(main.locator('article')).toHaveCount(2);
  await expect(main.locator('article').filter({hasText: ids.second})).toContainText('7200');

  await privatePage(page, `${target}/compare?ids=${ids.first}`, 400);
  await privatePage(page, `${target}?from=3&until=2`, 400);
  await privatePage(page, `${target}/${ids.first}/share`, 503);
  const share = await page.request.post(`${target}/${ids.first}/share`);
  expect(share.status()).toBe(503);
  expect(share.headers()['cache-control']).toContain('no-store');
});

test('player can save and clear an earned badge while unknown selections are refused', async ({page}, info) => {
  const {locale: lang} = info.project.metadata;
  const target = `/${lang}/account/badges`;
  await refusesAnonymous(page, target, 303, `/${lang}/login`);
  await signIn(page, lang, seedPlayer(lang));
  await privatePage(page, target);
  const main = page.locator('main');
  await strings(main, `templates/badges/${lang}.ftl`, [
    'title', 'introduction', 'badges', 'achievements', 'profile', 'ranking', 'none', 'save',
    'unavailable', 'account-age', 'loyalty', 'total-levels', 'all-professions',
  ]);
  const badge = main.locator('[name=badge][value=account-age]');
  for (const key of ['loyalty', 'total-levels', 'all-professions']) {
    await expect(main.locator(`[name=badge][value=${key}]`)).toBeDisabled();
  }
  const invalid = await post(page, target, {profile: 'invalid', ranking: ''});
  expect(invalid.status()).toBe(422);
  await badge.check();
  await submit(page);
  await privatePage(page, target);
  await expect(badge).toBeChecked();
  await badge.uncheck();
  await submit(page);
  await privatePage(page, target);
  await expect(badge).not.toBeChecked();
});

test('hirelings display only the owner’s localized roster and expedition history', async ({page}, info) => {
  const {locale: lang} = info.project.metadata;
  const target = `/${lang}/panel/hirelings`;
  await refusesAnonymous(page, target, 303, `/${lang}/login`);
  const owner = seedPlayer(lang);
  const visitor = seedPlayer(lang);
  const db = pool();
  try {
    const {rows: [helper]} = await db.query(
      "INSERT INTO hirelings(id,account_id,house_id,entitlement_id,profession,slot) VALUES(uuidv7(),$1,uuidv7(),uuidv7(),'miner',1) RETURNING id",
      [owner.id],
    );
    for (const state of ['pending', 'refused', 'claimed']) {
      await db.query(
        "INSERT INTO hireling_expeditions(id,account_id,hireling_id,destination,reward_table,duration_hours,state) VALUES(uuidv7(),$1,$2,'qualification','{}',1,$3)",
        [owner.id, helper.id, state],
      );
    }
  } finally {
    await db.end();
  }

  const catalog = `crates/hirelings/locales/${lang}.ftl`;
  const main = page.locator('main');
  await signIn(page, lang, visitor);
  await privatePage(page, target);
  await strings(main, catalog, ['title', 'limit', 'waiting', 'helpers', 'history', 'rewards', 'empty']);
  await expect(main.locator('ul')).toHaveCount(2);
  for (const list of await main.locator('ul').all()) await expect(list.locator('li')).toHaveCount(1);
  await switchPlayer(page, lang, owner);
  await privatePage(page, target);
  await strings(main, catalog, ['miner', 'pending', 'refused', 'claimed']);
  await expect(main.locator('form')).toHaveCount(0);
});

test('journal report and notes enforce Game access and email preferences persist', async ({page}, info) => {
  const {locale: lang} = info.project.metadata;
  const account = `/${lang}/account`;
  const reports = `${account}/journal/reports`;
  const notes = `${reports}/notes`;
  const target = `${notes}?kind=session&id=019a541b-a000-7000-8000-000000000001`;
  await refusesAnonymous(page, reports, 303, `/${lang}/login`);
  await refusesAnonymous(page, `${notes}/export`, 303, `/${lang}/login`);

  const player = seedPlayer(lang);
  await signIn(page, lang, player);
  for (const path of [`${account}/dashboard`, `${account}/game-logins`, reports,
    `${reports}?format=csv&dataset=drops`, `${reports}?format=json`]) {
    const response = await privatePage(page, path, 503);
    expect(await response.text()).not.toContain(player.email);
  }
  await privatePage(page, target, 404);
  const world = '0192a000-0000-7000-8000-000000000012';
  const character = '019a541b-a000-7000-8000-000000000001';
  for (const path of [
    `/${lang}/panel/items/${world}/${character}?q=local`,
    `/${lang}/panel/history/${world}/bank`,
    `/${lang}/panel/history/${world}/market/${character}`,
    `/${lang}/panel/history/${world}/trades/${character}`,
  ]) {
    const unavailable = await page.request.get(path);
    expect(unavailable.status(), path).toBe(503);
    expect(unavailable.headers()['cache-control']).toContain('no-store');
    expect(await unavailable.text()).not.toContain(player.email);
  }
  const noteExport = await page.request.get(`${notes}/export`);
  expect(noteExport.status()).toBe(200);
  expect(noteExport.headers()['content-disposition']).toBe('attachment; filename="journal-notes.json"');
  expect(await noteExport.json()).toEqual([]);

  await privatePage(page, `${account}/notifications`);
  await csrfRejected(page, target, {body: 'Local journal note'});
  expect((await post(page, target, {body: 'x'.repeat(4001)})).status()).toBe(422);
  expect((await post(page, target, {body: 'Local journal note'})).status()).toBe(404);
  const preference = `${reports}/email`;
  await csrfRejected(page, preference, {enabled: 'true'});
  expect((await post(page, preference, {enabled: 'invalid'})).status()).toBe(422);
  const db = pool();
  try {
    for (const enabled of [true, false]) {
      const fields = enabled ? {enabled: 'true'} : {};
      expect((await post(page, preference, fields)).status()).toBe(204);
      const result = await db.query('SELECT email_enabled FROM pd_report_preferences WHERE account_id=$1', [player.id]);
      expect(result.rows).toEqual([{email_enabled: enabled}]);
    }
  } finally {
    await db.end();
  }
});
