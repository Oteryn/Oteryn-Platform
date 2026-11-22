import { test, expect } from '../fixtures.mjs';
import { messages, missingId, probe, redirect, seedWorld, visit } from '../public-depth-helpers.mjs';

test('world registry refreshes browser rows and conditionally validates API snapshots', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const t = messages('crates/ui/i18n', lang);
  await redirect(page.request, '/servers', '/pl/worlds', 301);
  const api = '/api/v1/worlds';
  const first = await page.request.get(api);
  expect(first.status()).toBe(200);
  const stale = first.headers().etag;
  expect(stale).toMatch(/^W\/"[0-9a-f]{64}"$/);

  const world = await seedWorld();
  const fresh = await page.request.get(api, { headers: { 'if-none-match': stale } });
  expect(fresh.status()).toBe(200);
  expect(fresh.headers()['cache-control']).toContain('no-store');
  expect(fresh.headers().etag).not.toBe(stale);
  const payload = await fresh.json();
  expect(payload.schema).toBe(1);
  expect(payload.worlds.find(row => row.id === world.id)).toEqual({
    ...world, status: 'unknown', players_online: null,
  });
  const unchanged = await page.request.get(api, { headers: { 'if-none-match': fresh.headers().etag } });
  expect(unchanged.status()).toBe(304);
  expect(unchanged.headers().etag).toBe(fresh.headers().etag);
  expect((await unchanged.body()).length).toBe(0);

  for (const route of ['/worlds', `/${lang}/worlds`]) {
    await visit(page, route, 200, { lang, message: t['worlds-description'] });
    await expect(page.locator('main h1')).toHaveText(t['worlds-title']);
    const card = page.locator('main article').filter({ has: page.getByRole('heading', { name: world.name, exact: true }) });
    await expect(card.locator('dt')).toHaveText(['location', 'status', 'players', 'as-of'].map(key => t[`worlds-${key}`]));
    await expect(card.locator('dd')).toHaveText(['local', ...Array(3).fill(t['worlds-unknown'])]);
  }
  const other = lang === 'pl' ? 'en' : 'pl';
  await page.locator(`header.op-panel a[hreflang=${other}][href="/${other}/worlds"]`).click();
  await expect(page.locator('main h1')).toHaveText(messages('crates/ui/i18n', other)['worlds-title']);
});

test('rankings validate categories, filters and pagination before reading Game snapshots', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const t = messages('crates/ui/i18n', lang);
  const growth = messages('templates/stats/rankings', lang);
  const scores = `/${lang}/rankings`;
  const stats = `/${lang}/stats/rankings`;
  await redirect(page.request, '/highscores?page=2&vocation=1&sort=name',
    '/pl/rankings/level?page=2&vocation=1', 301);
  await visit(page, `${scores}/experience`, 503, { lang, message: t['rankings-unavailable'] });
  await expect(page.getByRole('table')).toHaveCount(0);
  for (const kind of ['level', 'experience', 'magic', 'fist', 'club', 'sword', 'axe', 'distance', 'shielding', 'fishing']) {
    await probe(page, `${scores}/${kind}`, 503, { lang, message: t['rankings-unavailable'], empty: 'table' });
  }
  for (const kind of ['yesterday', 'week', 'month', 'record', 'oldest', 'anniversaries', 'name-changes', 'transfers', 'sold']) {
    await probe(page, `${stats}/${kind}`, 503, { lang, message: growth.unavailable, empty: 'main tbody tr' });
  }
  await probe(page, `${stats}/yesterday?world=${missingId}&vocation=5&pvp=hardcore&page=20000`, 503,
    { lang, message: growth.unavailable });
  for (const query of ['page=0', 'page=01', 'page=1&page=2', 'sort=name', 'page=%GG']) {
    await probe(page, `${scores}/experience?${query}`, 400, { lang, message: t['rankings-bad-query'] });
  }
  for (const query of ['world=bad', 'vocation=6']) {
    await probe(page, `${scores}/experience?${query}`, 404, { lang, message: t['rankings-not-found'] });
  }
  await visit(page, `${scores}/unknown`, 404, { lang, message: t['rankings-not-found'] });
  await visit(page, `${stats}/unknown`, 404, { lang, message: growth['not-found'] });
  for (const query of ['world=bad', 'vocation=6', 'pvp=closed', 'page=0', 'page=20001', 'page=1&page=2', 'sort=name']) {
    await probe(page, `${stats}/yesterday?${query}`, 400, { lang, message: growth.invalid });
  }
});

test('statistics exercise the world picker and bounded guild, character and comparison filters', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const world = await seedWorld();
  const dir = 'templates/stats';
  const prefix = `/${lang}/stats`;
  const worldText = messages(`${dir}/world`, lang);
  const guildText = messages(`${dir}/guilds`, lang);
  const characterText = messages(`${dir}/character`, lang);
  await visit(page, `${prefix}/world`, 200, { lang });
  await page.locator('select[name=world]').selectOption({ label: world.name });
  await page.locator('input[name=start]').fill('2026-10-01');
  await page.locator('input[name=end]').fill('2026-10-31');
  const [response] = await Promise.all([
    page.waitForResponse(reply => reply.request().isNavigationRequest() && reply.url().includes('/stats/world?')),
    page.locator('main button[type=submit]').click(),
  ]);
  expect(response.status()).toBe(503);
  expect(response.headers()['cache-control']).toContain('no-store');
  expect(Object.fromEntries(new URL(response.url()).searchParams)).toEqual({
    world: world.id, start: '2026-10-01', end: '2026-10-31',
  });
  await expect(page.locator('main')).toContainText(worldText['world-unavailable']);
  for (const [start, end] of [['2026-02-29', '2026-03-01'], ['2026-10-31', '2026-10-01'], ['2026-10-01', '2026-11-01']]) {
    await probe(page, `${prefix}/world?world=${world.id}&start=${start}&end=${end}`, 400);
  }
  await probe(page, `${prefix}/world?world=unknown&start=2026-10-01&end=2026-10-01`, 400);
  for (const query of ['', '?guild=First&compare=Second&days=31']) {
    await visit(page, `${prefix}/guilds${query}`, 503, { lang, message: guildText['guild-unavailable'] });
  }
  for (const query of ['days=0', 'days=32', 'guild=%00', 'compare=%00', 'compare=Second', 'sort=level']) {
    await probe(page, `${prefix}/guilds?${query}`, 400);
  }

  const dates = 'start=2026-10-01&end=2026-10-02';
  const character = `${prefix}/characters/${missingId}`;
  for (const tab of ['history', 'experience', 'rankings', 'deaths', 'progress', 'online']) {
    await probe(page, `${character}/${tab}?${dates}&target=10`, 503, { lang, message: characterText.unavailable });
  }
  for (const query of ['start=2026-02-29', 'end=2099-01-01', 'target=01', 'metric=level', 'ids=unknown']) {
    await probe(page, `${character}/history?${query}`, 400, { lang, message: characterText.invalid });
  }
  await probe(page, `${character}/unknown`, 404, { lang });
  const compare = `${prefix}/compare`;
  await visit(page, compare, 200, { lang });
  const ids = `${missingId},${missingId.slice(0, -1)}e`;
  for (const metric of ['level', 'experience', 'magic_level', 'fist_level', 'club_level', 'sword_level',
    'axe_level', 'distance_level', 'shielding_level', 'fishing_level']) {
    await probe(page, `${compare}?${dates}&ids=${ids}&metric=${metric}`, 503, { lang, message: characterText.unavailable });
  }
  for (const query of [`ids=${missingId}`, `ids=${missingId},${missingId}`, `ids=${ids}&metric=unknown`, `ids=${ids}&target=1`]) {
    await probe(page, `${compare}?${query}`, 400, { lang, message: characterText.invalid });
  }
});

test('stats watches refuse anonymous and unrecognized bearer calls for each method', async ({ request }) => {
  const path = '/api/v1/stats/watches';
  for (const method of ['GET', 'POST', 'DELETE']) {
    for (const headers of [{}, { authorization: 'Bearer unrecognized-e2e' }]) {
      const response = await request.fetch(path, { method, headers });
      expect(response.status(), method).toBe(401);
      expect(response.headers()['cache-control']).toContain('no-store');
      expect(await response.text()).toBe('');
    }
  }
});
