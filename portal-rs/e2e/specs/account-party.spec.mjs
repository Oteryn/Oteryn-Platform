import {test, expect} from '../fixtures.mjs';
import {csrfRejected, pool, submit} from '../helpers.mjs';
import {post, privatePage, seedPlayer, signIn, strings, switchPlayer} from '../account-helpers.mjs';

test('party filters, reports, and owner removal honor post ownership', async ({page}, info) => {
  test.setTimeout(240_000);
  const {locale: lang} = info.project.metadata;
  const target = `/${lang}/party`;
  const owner = seedPlayer(lang);
  const reporter = seedPlayer(lang);
  const db = pool();
  const name = `Qualification ${owner.id.slice(-12)}`;
  try {
    // Existing Platform-owned listing, with a local read-model character marker.
    const {rows: [fixture]} = await db.query(
      'SELECT uuidv7() AS id, uuidv7() AS character, uuidv7() AS world',
    );
    await db.query(
      "INSERT INTO account_characters(account_id,character_id,world_id,name,availability) VALUES ($1,$2,$3,$4,'AVAILABLE')",
      [owner.id, fixture.character, fixture.world, name],
    );
    await db.query(
      `INSERT INTO pd_party_posts (id,account_id,character_id,character_name,world_id,purpose,
       min_level,max_level,profession,note,starts_at,expires_at)
       VALUES ($1,$2,$3,$4,$5,'hunt',10,100,'knight','Local board fixture',now(),now()+interval '1 hour')`,
      [fixture.id, owner.id, fixture.character, name, fixture.world],
    );
    const report = `${target}/${fixture.id}/report`;
    const remove = `${target}/${fixture.id}/remove`;
    const reportForm = `main form[action="${report}"]`;
    const removeForm = `main form[action="${remove}"]`;
    const main = page.locator('main');
    const listing = main.locator('article').filter({hasText: name});
    const catalog = `crates/community/src/party/party_${lang}.ftl`;

    await privatePage(page, target);
    await expect(listing).toBeVisible();
    await strings(main, catalog, ['title', 'hint', 'unavailable']);
    expect((await post(page, remove, {}, page.locator(removeForm))).status()).toBe(403);

    await signIn(page, lang, reporter);
    await privatePage(page, target);
    const filter = main.locator('form[method=get]');
    await filter.locator('[name=world]').fill(fixture.world);
    await filter.locator('[name=level]').fill('101');
    await filter.locator('[name=profession]').selectOption('knight');
    await filter.getByRole('button').click();
    await strings(main, catalog, ['empty']);
    await expect(listing).toHaveCount(0);

    await filter.locator('[name=world]').fill(fixture.world);
    await filter.locator('[name=level]').fill('50');
    await filter.locator('[name=profession]').selectOption('knight');
    await filter.getByRole('button').click();
    await expect(listing).toContainText('Local board fixture');
    await strings(main, catalog, [
      'world', 'purpose', 'level', 'profession', 'filter', 'create', 'character', 'min', 'max',
      'starts', 'expires', 'note', 'contact', 'reason', 'report', 'remove',
      'hunt', 'boss', 'quest', 'any', 'knight', 'paladin', 'sorcerer', 'druid', 'monk',
    ]);

    expect((await post(page, remove, {}, page.locator(removeForm))).status()).toBe(404);
    expect((await post(page, report, {reason: ' '}, page.locator(reportForm))).status()).toBe(422);
    await submit(page, reportForm, {reason: 'First local report'});
    await expect(page).toHaveURL(target);
    await submit(page, reportForm, {reason: 'Duplicate local report'});
    const reports = await db.query(
      'SELECT account_id, reason, status FROM pd_party_reports WHERE post_id=$1',
      [fixture.id],
    );
    expect(reports.rows).toEqual([{account_id: reporter.id, reason: 'First local report', status: 'open'}]);

    await switchPlayer(page, lang, owner);
    await privatePage(page, target);
    expect((await post(page, report, {reason: 'Self report'}, page.locator(reportForm))).status()).toBe(403);
    await submit(page, removeForm);
    await expect(listing).toHaveCount(0);
    expect((await db.query('SELECT status FROM pd_party_posts WHERE id=$1', [fixture.id])).rows[0].status)
      .toBe('removed');
  } finally {
    await db.end();
  }
});

test('party creation validates form input and refuses an unproven Game character', async ({page}, info) => {
  const {locale: lang} = info.project.metadata;
  const target = `/${lang}/party`;
  const player = seedPlayer(lang);
  await signIn(page, lang, player);
  await privatePage(page, target);
  const formSelector = `main form[action="${target}"]`;
  const form = page.locator(formSelector);
  const now = Math.floor(Date.now() / 1000);
  const draft = {
    character: '019a541b-a000-7000-8000-000000000001',
    purpose: 'hunt', profession: 'any', min_level: '1', max_level: '100',
    starts_at: String(now + 300), expires_at: String(now + 3600), note: 'Unproven Game character',
  };
  expect((await post(page, target, {...draft, max_level: '0'}, form)).status()).toBe(422);
  for (const name of ['purpose', 'profession']) {
    await form.locator(`[name=${name}]`).selectOption(draft[name]);
  }
  for (const name of ['character', 'min_level', 'max_level', 'starts_at', 'expires_at', 'note']) {
    await form.locator(`[name=${name}]`).fill(draft[name]);
  }
  const payload = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  expect(payload).toMatchObject(draft);
  await csrfRejected(page, target, payload);
  const refused = await post(page, target, payload, form);
  expect(refused.status()).toBe(403);
  expect(refused.headers()['cache-control']).toContain('no-store');

  const [navigation] = await Promise.all([
    page.waitForResponse(response => response.request().isNavigationRequest()
      && response.request().method() === 'POST' && new URL(response.url()).pathname === target),
    form.getByRole('button').click(),
  ]);
  expect(navigation.status()).toBe(403);
  await expect(page).toHaveURL(url => url.pathname === target);
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  await expect(page.getByRole('heading', {level: 1})).toHaveText(lang === 'pl' ? 'Brak dostępu' : 'Access denied');

  const db = pool();
  try {
    expect((await db.query('SELECT id FROM pd_party_posts WHERE account_id=$1', [player.id])).rows).toEqual([]);
  } finally {
    await db.end();
  }
});
