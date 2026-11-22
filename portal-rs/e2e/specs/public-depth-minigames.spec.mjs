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
  const routes = [
    ...collectionRoutes,
    ['get', '/puzzles'],
    ['get', `/puzzles/${missingId}/image`],
    ['post', `/puzzles/${missingId}/answer`, { choice: '0' }],
    ['post', '/collection-pvp', { op: 'challenge' }],
  ];
  const pvp = messages('crates/minigames/src/collection_pvp', lang);
  await visit(page, `${prefix}/login`, 200);
  const csrf = await page.locator('main form input[name=_csrf]').inputValue();
  expect(csrf.length).toBe(64);
  const headers = { origin: new URL(page.url()).origin, 'x-csrf-token': csrf };
  for (const [method, route, fields] of routes) {
    const path = prefix + route;
    if (method === 'get') {
      await redirect(page.request, path, `${prefix}/login`);
      continue;
    }
    await csrfRejected(page, path, fields);
    const response = await page.request.post(path, { headers, form: fields ?? {}, maxRedirects: 0 });
    if (route === '/collection-pvp') {
      expect(response.status(), path).toBe(403);
      // This localized response comes from the actor check, after CSRF validation.
      expect(await response.text()).toContain(pvp.error);
    } else {
      expect(response.status(), path).toBe(303);
      expect(response.headers().location, path).toBe(`${prefix}/login`);
    }
  }

  await login(page, lang, await seedPlayer(lang));
  await expect(page).not.toHaveURL(new RegExp(`/${lang}/login$`));
  const puzzle = messages('crates/minigames/src/puzzles', lang);
  const collection = messages('crates/minigames/src/collection', lang);
  const puzzles = `${prefix}/puzzles`;
  await visit(page, puzzles, 200, { lang, message: puzzle.empty });
  for (const [method, route, fields] of routes) {
    if (method === 'post') await csrfRejected(page, prefix + route, fields);
  }
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
});

test('league withholds outcomes and standings when the producer watermark expires', async ({ page, request }, info) => {
  const lang = info.project.metadata.locale;
  const t = messages('crates/minigames/src/league', lang);
  const path = `/${lang}/league`;
  const control = process.env.PORTAL_E2E_LEAGUE_CONTROL;
  expect(control, 'existing league harness control').toBeTruthy();
  test.setTimeout(150_000);
  const expectPublished = async () => {
    await expect.poll(async () => (await request.get(path)).status(), {
      timeout: 45_000, intervals: [1000, 2000],
    }).toBe(200);
    await visit(page, path, 200, { lang });
    await expect(page.locator('main h2')).toHaveText([t.outcomes, t.standings]);
    await expect(page.locator('main article').filter({ hasText: 'timed-out' })).toBeVisible();
    await expect(page.getByRole('cell', { name: 'Aldric Guild', exact: true })).toBeVisible();
    await expect(page.getByRole('cell', { name: 'Second Guild', exact: true })).toBeVisible();
    await expect(page.locator('main tbody tr')).toHaveCount(2);
  };
  expect((await request.post(`${control}/resume`)).status()).toBe(200);
  await expectPublished();
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
  await expectPublished();
});
