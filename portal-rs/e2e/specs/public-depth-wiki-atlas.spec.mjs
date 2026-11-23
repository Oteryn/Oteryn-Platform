import { test, expect } from '../fixtures.mjs';
import { csrfRejected, login } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { messages, missingId, probe, seedPlayer, visit } from '../public-depth-helpers.mjs';

const entityKinds = ['items', 'creatures', 'npcs', 'spells', 'quests', 'loot', 'achievements', 'areas'];

test('wiki entity routes enforce catalogue availability and validate search filters', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const t = messages('crates/wiki/src/entities', lang);
  const prefix = `/${lang}/wiki`;
  await visit(page, prefix, 503, { lang, message: t.unavailable });
  for (const route of ['/search', ...entityKinds.flatMap(kind => [`/${kind}`, `/${kind}/absent-e2e`])]) {
    await probe(page, prefix + route, 503, { lang, message: t.unavailable, empty: 'main tbody tr' });
  }
  await probe(page, `${prefix}/search?q=health+potion&kind=items&category=consumable&classification=common&page=1000`,
    503, { lang, message: t.unavailable });
  for (const query of ['q=x', 'page=0', 'page=1001', 'page=1&page=2', 'kind=unknown',
    'category=%00', 'classification=%00', 'sort=name']) {
    await probe(page, `${prefix}/search?${query}`, 400, { lang, message: t.invalid });
  }
  await visit(page, `${prefix}/not-a-kind`, 404, { lang, message: t.missing });
});

test('wiki calculators submit real arithmetic and report invalid or missing authority', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const t = messages('templates/wiki/calculators', lang);
  const path = `/${lang}/wiki/calculators`;
  await visit(page, path, 200, { lang, message: t.estimate });
  await expect(page.locator('main h1')).toHaveText(t.title);
  await expect(page.locator('main h2')).toHaveText([t.experience, t.stamina, t.training, t.loot]);
  await expect(page.locator('main button')).toHaveText(Array(4).fill(t.calculate));
  for (const [id, key] of Object.entries({
    'experience-current': 'current', 'experience-target': 'target',
    'stamina-current': 'minutes', 'stamina-offline': 'offline',
    'training-remaining': 'remaining', 'training-family': 'family', 'training-dummy': 'dummy',
    'loot-total': 'total', 'loot-members': 'members',
  })) await expect(page.locator(`label[for=${id}]`)).toHaveText(t[key]);

  for (const [id, fields, output, label] of [
    ['loot', { total: '101', members: '4' }, '25; 1', t['loot-result']],
    ['loot', { total: '101', members: '5' }, '20; 1', t['loot-result']],
    ['stamina', { current: '2339', offline: '1140' }, '2341', t['stamina-result']],
    ['stamina', { current: '2520', offline: '1140' }, '2520', t['stamina-result']],
  ]) {
    const form = page.locator(`form:has(input[value=${id}])`);
    for (const [name, value] of Object.entries(fields)) {
      await form.locator(`input[name=${name}]`).fill(value);
    }
    const [response] = await Promise.all([
      page.waitForResponse(reply => reply.request().isNavigationRequest() && reply.url().includes(`calculator=${id}`)),
      form.getByRole('button').click(),
    ]);
    expect(response.status()).toBe(200);
    for (const [name, value] of Object.entries(fields)) {
      expect(new URL(response.url()).searchParams.get(name)).toBe(value);
    }
    await expect(page.locator('section[aria-live] h2')).toHaveText(label);
    await expect(page.locator('output')).toHaveText(output);
  }

  for (const query of ['calculator=loot&total=10&members=0', 'calculator=loot&total=-1&members=4',
    'calculator=stamina&current=2521&offline=600', 'calculator=experience&current=1',
    'calculator=training&remaining=1&family=1&dummy=', 'calculator=unknown', 'extra=1']) {
    await visit(page, `${path}?${query}`, 400, { lang, message: t.invalid });
    await expect(page.locator('output')).toHaveCount(0);
  }
  for (const query of ['calculator=experience&current=100&target=10',
    'calculator=training&remaining=100&family=0&dummy=exercise-dummy']) {
    await visit(page, `${path}?${query}`, 503, { lang, message: t.unavailable });
  }
  const other = lang === 'en' ? 'pl' : 'en';
  await page.locator(`nav[aria-label="${t.language}"] a[hreflang=${other}]`).click();
  await expect(page).toHaveURL(new RegExp(`/${other}/wiki/calculators$`));
  await expect(page.locator('main h1')).toHaveText(messages('templates/wiki/calculators', other).title);
});

test('atlas routes validate coordinates, viewport and delivery filters before publishing data', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const path = `/${lang}/atlas`;
  const delivery = messages('crates/atlas/src/delivery', lang);
  for (const route of ['', '?x=100&y=101&z=7&zoom=1']) {
    await visit(page, path + route, 503);
    await expect(page.locator('main img')).toHaveCount(0);
  }
  for (const query of ['x=-1', 'y=4294967296', 'z=256', 'zoom=no', 'floor=7']) {
    await probe(page, `${path}?${query}`, 400);
  }
  await visit(page, `${path}/delivery?item=rope&creature=rat&q=rat&level=10&floor=7&city=thais&rarity=10&count=2&tab=creatures`,
    503, { lang, message: delivery.unavailable });
  for (const query of ['item=%00', 'creature=%00', 'q=%00', 'city=%00', 'level=0', 'floor=128',
    'rarity=1000001', 'count=0', 'count=100001', 'tab=invalid', 'sort=profit']) {
    await probe(page, `${path}/delivery?${query}`, 400, { lang, message: delivery.invalid });
  }
  const css = await page.request.get('/atlas/delivery.css');
  expect(css.status()).toBe(200);
  expect(css.headers()['content-type']).toContain('text/css');
  expect((await css.text()).length).toBeGreaterThan(0);
  const viewport = `version=${'0'.repeat(40)}&floor=7&min_x=1&min_y=2&max_x=3&max_y=4`;
  for (const [url, status] of [
    [`/atlas/tiles/${'0'.repeat(40)}/7/1/0/0.webp`, 503],
    [`/atlas/spawns?${viewport}`, 503],
    [`/atlas/spawns?${viewport.replace('max_x=3', 'max_x=0')}`, 400],
    ['/atlas/private-markers', 401],
  ]) expect((await page.request.get(url)).status(), url).toBe(status);
  const strings = await page.request.get(`/atlas/spawns-text/${lang}`);
  expect(strings.status()).toBe(200);
  const spawnText = readSpawnText(lang);
  expect(await strings.json()).toEqual(spawnText);
});

function readSpawnText(lang) {
  return messages('crates/atlas/src', `spawns_${lang}`);
}

test('wiki places filters reject unsupported options and keep recommendations unavailable', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const path = `/${lang}/wiki/places`;
  for (const [purpose, sort, party] of [['hunt', 'experience', 1], ['bestiary', 'profit', 2], ['boss', 'profit', 4]]) {
    await visit(page, `${path}?purpose=${purpose}&sort=${sort}&party=${party}`, 503, { lang });
  }
  for (const query of ['sort=level', 'purpose=unknown', 'party=3', 'page=2']) {
    await probe(page, `${path}?${query}`, 400);
  }
});

test('atlas private markers and reports enforce actor ownership and persist valid changes', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const markers = '/atlas/private-markers';
  const reports = '/atlas/map-reports';
  const owner = await seedPlayer(lang);
  await login(page, lang, owner);
  await expect(page).not.toHaveURL(new RegExp(`/${lang}/login$`));
  const origin = new URL(page.url()).origin;
  const headers = async request => {
    const response = await request.get(markers);
    expect(response.status()).toBe(200);
    expect(response.headers()['cache-control']).toContain('no-store');
    const state = await response.json();
    expect(state.markers).toEqual([]);
    expect(Boolean(state.csrf_token)).toBe(true);
    return { origin, 'x-csrf-token': state.csrf_token };
  };
  const authorized = await headers(page.request);
  const point = { x: 100, y: 101, floor: 7 };
  const marker = { ...point, text: 'Private route waypoint' };
  expect((await page.request.post(markers, { headers: { origin }, data: marker })).status()).toBe(403);
  expect((await page.request.post(markers, { headers: authorized, data: { ...marker, text: ' ' } })).status()).toBe(422);
  const created = await page.request.post(markers, { headers: authorized, data: marker });
  expect(created.status()).toBe(201);
  const saved = await created.json();
  expect(saved).toEqual({ id: expect.any(String), ...point, label: marker.text });
  expect((await (await page.request.get(markers)).json()).markers).toEqual([saved]);

  const other = await browser.newContext({
    baseURL: origin, ignoreHTTPSErrors: true, locale: lang, viewport: info.project.use.viewport,
  });
  try {
    const reader = await other.newPage();
    await login(reader, lang, await seedPlayer(lang));
    const otherHeaders = await headers(reader.request);
    expect((await reader.request.delete(`${markers}/${saved.id}`, { headers: otherHeaders })).status()).toBe(404);
  } finally {
    await other.close();
  }
  expect((await page.request.delete(`${markers}/${saved.id}`, { headers: authorized })).status()).toBe(204);
  expect((await page.request.delete(`${markers}/${saved.id}`, { headers: authorized })).status()).toBe(404);
  expect((await (await page.request.get(markers)).json()).markers).toEqual([]);

  const description = `Map annotation report ${crypto.randomUUID()}`;
  expect((await page.request.post(reports, { headers: authorized, data: { ...point, text: 'short' } })).status()).toBe(422);
  expect((await page.request.post(reports, { headers: authorized, data: { ...point, text: description } })).status()).toBe(201);
  expect(await query('SELECT status FROM atlas_map_reports WHERE description=$1', [description])).toEqual([{ status: 'pending' }]);

  const bestiary = `/${lang}/atlas/bestiary/${missingId}`;
  await visit(page, `${bestiary}?class=mammal&difficulty=easy&stars=5`, 503);
  for (const query of ['class=%00', 'difficulty=%00', 'stars=6']) {
    await visit(page, `${bestiary}?${query}`, 400);
  }
});

test('atlas hunting aggregates refuse missing data while private participation persists', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const path = `/${lang}/atlas/places`;
  const preference = `${path}/participation`;
  expect((await page.request.get(preference)).status()).toBe(403);
  await login(page, lang, await seedPlayer(lang));
  const initial = await page.request.get(preference);
  expect(initial.status()).toBe(200);
  expect(initial.headers()['cache-control']).toContain('no-store');
  expect(await initial.json()).toEqual({ participating: false });
  await visit(page, path, 503);
  await visit(page, `${path}?world=local`, 400);
  expect((await page.request.get(`${preference}?participating=true`)).status()).toBe(400);

  const markerState = await page.request.get('/atlas/private-markers');
  expect(markerState.status()).toBe(200);
  const { csrf_token } = await markerState.json();
  await csrfRejected(page, preference, { participating: 'true' });
  const headers = { origin: new URL(page.url()).origin };
  const invalid = await page.request.post(preference, {
    headers, form: { _csrf: csrf_token, participating: '1' }, maxRedirects: 0,
  });
  expect(invalid.status()).toBe(400);
  for (const participating of [true, false]) {
    const update = await page.request.post(preference, {
      headers, form: { _csrf: csrf_token, participating: String(participating) }, maxRedirects: 0,
    });
    expect(update.status()).toBe(303);
    expect(update.headers().location).toBe(path);
    const state = await page.request.get(preference);
    expect(state.status()).toBe(200);
    expect(await state.json()).toEqual({ participating });
  }
});
