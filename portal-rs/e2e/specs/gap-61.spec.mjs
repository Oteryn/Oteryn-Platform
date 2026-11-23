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
  const form = { name: 'First deck', first: card };
  await csrfRejected(page, save, form);
  expect((await page.goto(`/${lang}/account/sudo`)).status()).toBe(200);
  const csrf = await page.locator('main form input[name=_csrf]').inputValue();
  const headers = { origin: new URL(page.url()).origin };
  const missing = await page.request.post(save, { headers, form: { ...form, _csrf: csrf } });
  expect(missing.status()).toBe(503);
  const invalid = await page.request.post(save, { headers, form: { ...form, second: card, _csrf: csrf } });
  expect(invalid.status()).toBe(400);
  expect(await query('SELECT id FROM collection_decks WHERE account_id=$1', [owner.id])).toEqual([]);

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
  await seededStaff(page, lang, 'game_admin');
  expect((await page.goto(`/${lang}/collection`)).status()).toBe(200);
  const status = page.getByRole('region', { name: t['operator-status'] });
  await expect(status).toContainText(t['operator-journal']);
  await expect(status).toContainText(t['operator-source']);
  await expect(status).toContainText(t['operator-awaiting-mapping']);
  await expect(status).toContainText(t['operator-publication']);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
});
