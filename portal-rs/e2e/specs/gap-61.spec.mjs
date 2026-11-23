import { test, expect } from '../fixtures.mjs';
import { csrfRejected } from '../helpers.mjs';
import { messages } from '../public-depth-helpers.mjs';
import { query, seededAccount, seededLogin, seededStaff } from '../staff-rbac-helpers.mjs';

const localeDirectory = 'crates/minigames/src/collection';

test('an owner opens Collection from navigation and sees the current progress state', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const t = messages(localeDirectory, lang);
  const path = `/${lang}/collection`;
  const owner = await seededAccount(lang);
  const visitor = await seededAccount(lang);
  const [{ character, world, card }] = await query('SELECT uuidv7() AS character, uuidv7() AS world, uuidv7() AS card');
  // The existing synthetic portfolio producer retains epoch 3 and the first
  // snapshot for each fresh account starts at revision 1.
  await query(
    `INSERT INTO pd_ownership_baselines(character_id,account_id,world_id,character_revision,
       from_projection_revision,to_projection_revision,projection_epoch,recovery_generation,accepted_at)
     VALUES($1,$2,$3,1,1,1,3,3,now()-interval '1 hour')`,
    [character, owner.id, world],
  );
  const publication = await fetch(`${process.env.PORTAL_E2E_GAME_CLOSURE_CONTROL_URL}/portfolio`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ account_id: owner.id, character_id: character, world_id: world }),
  });
  expect(publication.status).toBe(200);
  const { name } = await publication.json();
  await seededLogin(page, lang, owner);
  await page.goto(`/${lang}/news`);
  if (info.project.use.viewport.width <= 960) {
    await page.locator('.om-fallback-menu > summary').click();
  }
  const worldMenu = page.locator('#portal-world-panel');
  await worldMenu.locator('..').locator('summary').click();
  await worldMenu.locator(`a[href="${path}"]`).click();
  await expect(page.locator('main h1')).toHaveText(t.title);
  await expect(page.getByRole('region', { name: t['operator-status'] })).toHaveCount(0);
  await page.getByRole('link', { name, exact: true }).click();
  await expect(page).toHaveURL(new RegExp(`/${character}$`));
  const cards = page.getByRole('region', { name: t.cards });
  await expect(cards).toContainText(t.unavailable);
  await expect(page.getByRole('region', { name: t.bestiary })).toContainText(t['source-unavailable']);
  await expect(page.locator('main form')).toHaveCount(0);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);

  const save = `${path}/${character}/decks`;
  const form = { name: 'W'.repeat(40), first: card };
  await csrfRejected(page, save, form);
  expect((await page.goto(`/${lang}/account/sudo`)).status()).toBe(200);
  const csrf = await page.locator('main form input[name=_csrf]').inputValue();
  const headers = { origin: new URL(page.url()).origin };
  const missing = await page.request.post(save, { headers, form: { ...form, _csrf: csrf } });
  expect(missing.status()).toBe(503);
  const invalid = await page.request.post(save, { headers, form: { ...form, second: card, _csrf: csrf } });
  expect(invalid.status()).toBe(400);
  expect(await query('SELECT id FROM collection_decks WHERE account_id=$1', [owner.id])).toEqual([]);

  const [{ id: deck }] = await query(
    'INSERT INTO collection_decks(id,account_id,character_id,name,cards) VALUES(uuidv7(),$1,$2,$3,ARRAY[$4::uuid]) RETURNING id',
    [owner.id, character, form.name, card],
  );
  const [{ id: battle }] = await query(
    `INSERT INTO collection_battles(id,account_id,character_id,deck_id,state)
     VALUES(uuidv7(),$1,$2,$3,'{"outcome":"active"}') RETURNING id`,
    [owner.id, character, deck],
  );
  expect((await page.goto(`${path}/${character}`)).status()).toBe(200);
  await expect(page.getByRole('region', { name: t.cards })).toContainText(t.unavailable);
  await expect(page.getByRole('heading', { name: form.name })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
  await expect(page.getByRole('button', { name: t.save })).toHaveCount(0);
  await expect(page.getByRole('button', { name: t.fight })).toHaveCount(0);
  await page.getByRole('button', { name: t.forfeit }).click();
  await expect(page).toHaveURL(new RegExp(`${path}$`));
  await expect(page.getByRole('button', { name: t.forfeit })).toHaveCount(0);
  expect(await query("SELECT state->>'outcome' AS outcome FROM collection_battles WHERE id=$1", [battle]))
    .toEqual([{ outcome: 'defeat' }]);
  expect((await page.goto(`${path}/${character}`)).status()).toBe(200);
  await page.getByRole('button', { name: t.delete }).click();
  await expect(page.getByRole('region', { name: t.cards })).toContainText(t.unavailable);
  await expect(page.getByRole('button', { name: t.delete })).toHaveCount(0);
  expect(await query('SELECT id FROM collection_decks WHERE id=$1', [deck])).toEqual([]);

  const [{ id: archivedBattle }] = await query(
    `INSERT INTO collection_battles(id,account_id,character_id,state)
     VALUES(uuidv7(),$1,uuidv7(),'{"outcome":"active"}') RETURNING id`,
    [owner.id],
  );
  expect((await page.goto(path)).status()).toBe(200);
  await page.getByRole('button', { name: t.forfeit }).click();
  await expect(page.getByRole('button', { name: t.forfeit })).toHaveCount(0);
  expect(await query("SELECT state->>'outcome' AS outcome FROM collection_battles WHERE id=$1", [archivedBattle]))
    .toEqual([{ outcome: 'defeat' }]);

  const context = await browser.newContext({
    baseURL: info.project.use.baseURL, ignoreHTTPSErrors: true, viewport: info.project.use.viewport,
  });
  try {
    const other = await context.newPage();
    await seededLogin(other, lang, visitor);
    expect((await other.goto(`${path}/${character}`)).status()).toBe(403);
    await expect(other.getByRole('region', { name: t.bestiary })).toHaveCount(0);
  } finally {
    await context.close();
  }
});

test('a world operator sees the collection readiness checks', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const t = messages(localeDirectory, lang);
  const operator = await seededStaff(page, lang, 'game_admin');
  expect((await page.goto(`/${lang}/collection`)).status()).toBe(200);
  const status = page.getByRole('region', { name: t['operator-status'] });
  await expect(status).toContainText(t['operator-journal']);
  await expect(status.getByRole('definition').nth(0)).toHaveText(t['operator-enabled']);
  await expect(status).toContainText(t['operator-source']);
  await expect(status.getByRole('definition').nth(1)).toHaveText(t[
    process.env.PORTAL_PLAYERDATA_PROGRESS_SOURCE_AUTHORITY ? 'operator-configured' : 'source-unavailable'
  ]);
  await expect(status).toContainText(t['operator-awaiting-mapping']);
  await expect(status).toContainText(t['operator-publication']);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);

  await query("UPDATE web_sessions SET data=data-'mfa_grant' WHERE account_id=$1", [operator]);
  expect((await page.reload()).status()).toBe(200);
  await expect(status).toHaveCount(0);
});

test('i18n-parity for Collection', () => {
  const en = Object.keys(messages(localeDirectory, 'en')).sort();
  expect(en.length).toBeGreaterThan(0);
  expect(Object.keys(messages(localeDirectory, 'pl')).sort()).toEqual(en);
});
