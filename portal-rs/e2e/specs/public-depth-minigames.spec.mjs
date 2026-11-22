import { test, expect } from '../fixtures.mjs';
import { csrfRejected, login } from '../helpers.mjs';
import { messages, missingId, redirect, seedPlayer, visit } from '../public-depth-helpers.mjs';

const collectionRoutes = [
  ['get', '/collection'],
  ['get', `/collection/${missingId}`],
  ['get', `/collection/${missingId}/cards/${missingId}/image`],
  ['get', `/collection/battles/${missingId}`],
  ['post', `/collection/${missingId}/decks`],
  ['post', `/collection/decks/${missingId}/battle`],
  ['post', `/collection/battles/${missingId}`],
  ['post', `/collection/battles/${missingId}/forfeit`],
  ['post', `/collection/decks/${missingId}/delete`],
  ['get', '/collection-pvp'],
];

test('puzzles and collection protect every player route and show localized empty states', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const prefix = `/${lang}`;
  for (const [method, route] of [
    ...collectionRoutes,
    ['get', '/puzzles'],
    ['get', `/puzzles/${missingId}/image`],
    ['post', `/puzzles/${missingId}/answer`],
  ]) {
    if (method === 'get') {
      await redirect(page.request, prefix + route, `${prefix}/login`);
    } else {
      await page.goto(`${prefix}/login`);
      await csrfRejected(page, prefix + route);
    }
  }

  await login(page, lang, await seedPlayer(lang));
  await expect(page).not.toHaveURL(new RegExp(`/${lang}/login$`));
  const puzzle = messages('crates/minigames/src/puzzles', lang);
  const collection = messages('crates/minigames/src/collection', lang);
  const pvp = messages('crates/minigames/src/collection_pvp', lang);
  const puzzles = `${prefix}/puzzles`;
  await visit(page, puzzles, 200, { lang, message: puzzle.empty });
  await expect(page.locator('main h1')).toHaveText(puzzle.title);
  await expect(page.locator('main')).toContainText(puzzle.rules);
  await expect(page.locator('main section p')).toHaveText([
    `${puzzle.current}: 0`, `${puzzle.best}: 0`, puzzle.none,
  ]);
  await expect(page.locator('main h2')).toHaveText(puzzle.badges);
  await expect(page.locator('main form')).toHaveCount(0);
  const other = lang === 'en' ? 'pl' : 'en';
  const switchLanguage = page.locator(`header.site-header a[hreflang=${other}][href='/${other}/puzzles']`);
  await expect(switchLanguage).toHaveText(puzzle['other-language']);
  await switchLanguage.click();
  await expect(page).toHaveURL(new RegExp(`/${other}/puzzles$`));
  await expect(page.locator('main h1')).toHaveText(messages('crates/minigames/src/puzzles', other).title);
  await visit(page, `${puzzles}/${missingId}/image`, 404, { lang, message: puzzle.error });
  await page.getByRole('link', { name: puzzle.back, exact: true }).click();
  await expect(page).toHaveURL(new RegExp(`${puzzles}$`));

  await visit(page, `${prefix}/collection`, 200, { lang, message: collection.unavailable });
  await expect(page.locator('main h1')).toHaveText(collection.title);
  await expect(page.locator('main form')).toHaveCount(0);
  await visit(page, `${prefix}/collection/${missingId}`, 503, { lang, message: collection.unavailable });
  await visit(page, `${prefix}/collection/${missingId}/cards/${missingId}/image`, 503,
    { lang, message: collection.unavailable });
  await visit(page, `${prefix}/collection-pvp`, 200, { lang, message: pvp.unavailable });
  await expect(page.locator('main h1')).toHaveText(pvp.title);
  await expect(page.locator('main form')).toHaveCount(0);
  await csrfRejected(page, `${prefix}/collection-pvp`, { op: 'challenge' });
});

test('league withholds outcomes and standings when the producer watermark expires', async ({ page, request }, info) => {
  const lang = info.project.metadata.locale;
  const t = messages('crates/minigames/src/league', lang);
  const path = `/${lang}/league`;
  const control = process.env.PORTAL_E2E_LEAGUE_CONTROL;
  expect(control, 'existing league harness control').toBeTruthy();
  expect((await request.post(`${control}/resume`)).status()).toBe(200);
  await expect.poll(async () => (await request.get(path)).status(), { timeout: 45_000 }).toBe(200);
  try {
    expect((await request.post(`${control}/pause`)).status()).toBe(200);
    await expect.poll(async () => (await request.get(path)).status(), {
      timeout: 45_000, intervals: [1000, 2000],
    }).toBe(503);
    await visit(page, path, 503, { lang, message: t.unavailable });
    await expect(page.locator('main h1')).toHaveText(t.title);
    await expect(page.locator('main h2')).toHaveText([t.outcomes, t.standings]);
    await expect(page.locator('main article, main table, main form')).toHaveCount(0);
  } finally {
    expect((await request.post(`${control}/resume`)).status()).toBe(200);
  }
});
