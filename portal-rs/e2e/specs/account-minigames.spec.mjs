import {test, expect} from '../fixtures.mjs';
import {csrfRejected, pool, submit} from '../helpers.mjs';
import {post, privatePage, refusesAnonymous, seedPlayer, signIn, strings, switchPlayer} from '../account-helpers.mjs';

const absent = '019a541b-a000-7000-8000-000000000001';

async function seedPuzzles(page, target, catalog) {
  // Local editorial fixtures, following the public-world allowlist used by the Rust puzzle tests.
  const image = Buffer.from(await page.evaluate(() => {
    const canvas = document.createElement('canvas');
    canvas.width = canvas.height = 256;
    const paint = canvas.getContext('2d');
    paint.fillStyle = 'navy';
    paint.fillRect(0, 0, 256, 256);
    return canvas.toDataURL('image/webp').split(',')[1];
  }), 'base64');
  const db = pool();
  const client = await db.connect();
  const version = '019a541b-a000-7000-8000-000000000093';
  const kind = 'account-test-creature';
  const choices = [
    {label: {pl: 'Lis', en: 'Fox'}, source: {kind, key: 'fox'}},
    {label: {pl: 'Wilk', en: 'Wolf'}, source: {kind, key: 'wolf'}},
  ];
  const puzzleIds = [];
  try {
    // Daily editorial publication has one shared slot per kind.
    await client.query("SELECT pg_advisory_lock(hashtextextended('portal-e2e-daily-puzzles', 0))");
    await privatePage(page, target);
    await strings(page.locator('main'), catalog, ['empty']);
    await client.query('BEGIN');
    await client.query("INSERT INTO world_versions(id,source_commit,status) VALUES($1,repeat('a',40),'ready') ON CONFLICT DO NOTHING", [version]);
    for (const {source: {key}} of choices) {
      await client.query("INSERT INTO world_entities(version_id,kind,key,data,publishable) VALUES($1,$2,$3,'{}',true) ON CONFLICT DO NOTHING", [version, kind, key]);
      await client.query('INSERT INTO world_visibility(kind,key,allowed) VALUES($1,$2,true) ON CONFLICT DO NOTHING', [kind, key]);
    }
    const previous = (await client.query('SELECT version_id FROM world_active_version WHERE singleton FOR UPDATE')).rows[0].version_id;
    await client.query('UPDATE world_active_version SET version_id=$1 WHERE singleton', [version]);
    for (const kind of ['monster', 'atlas', 'quiz']) {
      const definition = {
        kind, world_version: version, choices, correct_choice: 1,
        question: {pl: 'Które zwierzę żyje w stadzie?', en: 'Which animal lives in a pack?'},
        clue: {pl: 'To dzikie zwierzę.', en: 'It is a wild animal.'},
      };
      const {rows: [puzzle]} = await client.query(
        "INSERT INTO puzzles_daily(day,kind,world_version,definition,image) VALUES((now() AT TIME ZONE 'UTC')::date,$1,$2,$3,$4) RETURNING id",
        [kind, version, definition, kind === 'quiz' ? null : image],
      );
      puzzleIds.push(puzzle.id);
    }
    await client.query('COMMIT');
    return async () => {
      try {
        await client.query('BEGIN');
        await client.query('UPDATE world_active_version SET version_id=$1 WHERE singleton AND version_id=$2', [previous, version]);
        await client.query('DELETE FROM puzzles_answers WHERE puzzle_id=ANY($1)', [puzzleIds]);
        // Remove only our fixtures while both publication guards are disabled.
        // Portal connections never see either guard disabled outside this transaction.
        await client.query('ALTER TABLE puzzles_daily DISABLE TRIGGER puzzles_daily_immutable');
        await client.query('ALTER TABLE puzzles_daily DISABLE TRIGGER puzzles_daily_staged_cutoff');
        await client.query('DELETE FROM puzzles_daily WHERE id=ANY($1)', [puzzleIds]);
        await client.query('ALTER TABLE puzzles_daily ENABLE TRIGGER puzzles_daily_staged_cutoff');
        await client.query('ALTER TABLE puzzles_daily ENABLE TRIGGER puzzles_daily_immutable');
        await client.query('DELETE FROM world_visibility WHERE kind=$1 AND key=ANY($2::text[])',
          [kind, choices.map(({source}) => source.key)]);
        await client.query('COMMIT');
      } finally {
        client.release(true);
        await db.end();
      }
    };
  } catch (error) {
    client.release(true);
    await db.end();
    throw error;
  }
}

test('daily puzzles load clues, record one attempt per player, and extend the streak', async ({page}, info) => {
  test.setTimeout(240_000);
  const {locale: lang} = info.project.metadata;
  const other = lang === 'pl' ? 'en' : 'pl';
  const target = `/${lang}/puzzles`;
  const main = page.locator('main');
  const catalog = `crates/minigames/src/puzzles/${lang}.ftl`;
  await refusesAnonymous(page, target, 303, `/${lang}/login`);
  await signIn(page, lang, seedPlayer(lang));
  const restore = await seedPuzzles(page, target, catalog);
  try {
    await privatePage(page, target);
    await strings(main, catalog, [
      'title', 'rules', 'current', 'best', 'badges', 'none', 'monster', 'atlas', 'quiz', 'choose', 'submit',
    ]);
    const languageLink = locale => page.locator(`a[hreflang=${locale}][href="/${locale}/puzzles"]:visible`).first();
    await languageLink(other).click();
    await expect(page).toHaveURL(`/${other}/puzzles`);
    await expect(page.locator('html')).toHaveAttribute('lang', other);
    await languageLink(lang).click();
    await expect(page).toHaveURL(target);
    const clue = lang === 'pl' ? 'To dzikie zwierzę.' : 'It is a wild animal.';
    await expect(main).toContainText(lang === 'pl' ? 'Które zwierzę żyje w stadzie?' : 'Which animal lives in a pack?');
    const images = main.getByRole('img', {name: clue});
    await expect(images).toHaveCount(2);
    for (const image of await images.all()) {
      await expect.poll(() => image.evaluate(img => img.naturalWidth)).toBe(256);
    }
    const actions = await main.locator('form[action$="/answer"]').evaluateAll(forms => forms.map(form => form.getAttribute('action')));
    expect(actions).toHaveLength(3);
    const invalid = await post(page, actions[0], {choice: '6'});
    expect(invalid.status()).toBe(400);
    for (const action of actions) {
      const selector = `main form[action="${action}"]`;
      const form = page.locator(selector);
      await expect(form.locator('option[value="1"]')).toHaveText(lang === 'pl' ? 'Wilk' : 'Wolf');
      await form.locator('[name=choice]').selectOption('1');
      await submit(page, selector);
      await expect(form).toHaveCount(0);
    }
    await expect(main.locator('[role=status]')).toHaveCount(3);
    await strings(main, catalog, ['correct']);
    await expect(main).toContainText(lang === 'pl' ? 'Obecna seria dni: 1' : 'Current streak in days: 1');

    await switchPlayer(page, lang, seedPlayer(lang));
    await privatePage(page, target);
    const first = `main form[action="${actions[0]}"]`;
    await page.locator(first).locator('[name=choice]').selectOption('0');
    await submit(page, first);
    await strings(main, catalog, ['wrong']);
    expect((await post(page, actions[0], {choice: '0'})).status()).toBe(303);
    expect((await post(page, actions[0], {choice: '1'})).status()).toBe(409);
    await privatePage(page, `${target}/${absent}/image`, 404);
    await strings(main, catalog, ['error', 'back']);
    await main.getByRole('link').click();
    await expect(page).toHaveURL(target);
    await strings(main, catalog, ['wrong']);
  } finally {
    await restore();
  }
});

test('season pass keeps unavailable progress private and refuses missing-season purchases and unwired claims', async ({page}, info) => {
  const {locale: lang} = info.project.metadata;
  const target = `/${lang}/pass`;
  const main = page.locator('main');
  const catalog = `templates/pass/${lang}.ftl`;
  await refusesAnonymous(page, target, 303, `/${lang}/login`);
  const player = seedPlayer(lang);
  await signIn(page, lang, player);
  const unavailable = await privatePage(page, target, 503);
  expect(await unavailable.text()).not.toContain(player.email);
  await strings(main, catalog, ['title', 'unavailable']);
  await expect(main.locator('form')).toHaveCount(0);

  await page.goto(`/${lang}/account/notifications`);
  for (const [action, fields, refusedStatus] of [
    ['buy', {operation: absent}, 403], // The enabled Wallet refuses a nonexistent season.
    ['claim', {operation: absent, reward: absent}, 404],
  ]) {
    const mutation = `${target}/${absent}/${action}`;
    await csrfRejected(page, mutation, fields);
    expect((await post(page, mutation, {...fields, operation: 'invalid'})).status()).toBe(422);
    expect((await post(page, mutation, fields)).status()).toBe(refusedStatus);
  }
});

test('collection keeps the Game-dependent selector unavailable and validates deck submissions', async ({page}, info) => {
  const {locale: lang} = info.project.metadata;
  const target = `/${lang}/collection`;
  const main = page.locator('main');
  const catalog = `crates/minigames/src/collection/${lang}.ftl`;
  await refusesAnonymous(page, target, 303, `/${lang}/login`);
  await signIn(page, lang, seedPlayer(lang));
  await privatePage(page, target);
  await strings(main, catalog, ['title', 'unavailable']);
  await expect(main.locator('form')).toHaveCount(0);
  await privatePage(page, `${target}/${absent}`, 503);
  await strings(main, catalog, ['unavailable']);
  await main.getByRole('link').click();
  await expect(page).toHaveURL(target);

  await page.goto(`/${lang}/account/notifications`);
  const decks = `${target}/${absent}/decks`;
  const fields = {name: 'Browser deck', first: absent};
  await csrfRejected(page, decks, fields);
  expect((await post(page, decks, {...fields, name: ''})).status()).toBe(400);
  expect((await post(page, decks, fields)).status()).toBe(503);
  for (const [path, fields] of [
    [`decks/${absent}/battle`, {}],
    [`battles/${absent}`, {expected: '0', action: 'strike'}],
    [`battles/${absent}`, {expected: '0', action: 'guard'}],
    [`battles/${absent}/forfeit`, {}],
    [`decks/${absent}/delete`, {}],
  ]) {
    const mutation = `${target}/${path}`;
    await csrfRejected(page, mutation, fields);
    expect((await post(page, mutation, {...fields, unexpected: 'invalid'})).status()).toBe(422);
    expect((await post(page, mutation, fields)).status()).toBe(403);
  }
  await privatePage(page, `${target}/battles/${absent}`, 403);
  await privatePage(page, `${target}/${absent}/cards/${absent}/image`, 503);
});

test('collection PvP and published set controls refuse when collection and payment adapters are absent', async ({page}, info) => {
  const {locale: lang} = info.project.metadata;
  const target = `/${lang}/collection-pvp`;
  const catalog = `crates/minigames/src/collection_pvp/${lang}.ftl`;
  await refusesAnonymous(page, target, 303, `/${lang}/login`);
  await signIn(page, lang, seedPlayer(lang));
  await privatePage(page, target);
  const main = page.locator('main');
  await strings(main, catalog, ['title', 'unavailable', 'rankings', 'sets', 'tournaments', 'battles']);
  await expect(main.locator('form')).toHaveCount(0);

  await page.goto(`/${lang}/account/notifications`);
  for (const fields of [
    {op: 'challenge', opponent: absent, deck: absent},
    {op: 'accept', id: absent, deck: absent},
    {op: 'decline', id: absent},
    {op: 'play', id: absent, revision: '0', action: '{}'},
    {op: 'create', capacity: '4'},
    {op: 'join', id: absent, deck: absent},
    {op: 'start', id: absent},
  ]) {
    await csrfRejected(page, target, fields);
    expect((await post(page, target, fields)).status()).toBe(503);
  }
  expect((await post(page, target, {op: 'invalid'})).status()).toBe(422);
  expect((await post(page, target, {op: 'play', id: absent, revision: '0', action: 'invalid'})).status()).toBe(422);
  const buy = {op: 'buy', id: absent, price: '10', version: '1', request: absent};
  await csrfRejected(page, target, buy);
  expect((await post(page, target, {...buy, price: '0'})).status()).toBe(422);
  expect((await post(page, target, buy)).status()).toBe(403);
});
