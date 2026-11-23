import { test, expect } from '../fixtures.mjs';
import { login } from '../helpers.mjs';
import { follow, open, seedPlayer, strings } from '../community-helpers.mjs';

const world = '019b0760-0000-7000-8000-000000000001';
const character = '019a0000-0000-7000-8000-000000000001';
const wikiKinds = ['items', 'creatures', 'npcs', 'spells', 'quests', 'achievements', 'loot', 'areas'];

test('world registry exposes cold status, links and API revalidation', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const t = strings('crates/ui/i18n', lang);
  await open(page, `/${lang}/worlds`);
  const main = page.locator('main');
  await expect(main.getByRole('heading', {level: 1})).toHaveText(t['worlds-title']);
  await expect(main.getByText(t['worlds-description'])).toBeVisible();
  const entry = main.locator('article').filter({hasText: 'Local Bazaar World'});
  await expect(entry.getByRole('heading')).toHaveText('Local Bazaar World');
  await expect(entry.locator('dt')).toContainText([
    t['worlds-status'], t['worlds-players'], t['worlds-as-of'],
  ]);
  await expect(entry.locator('dd').last()).toHaveText(t['worlds-unknown']);

  const response = await page.request.get('/api/v1/worlds');
  expect(response.status()).toBe(200);
  const body = await response.json();
  expect(body.schema).toBe(1);
  expect(body.generated_at).toBeGreaterThan(0);
  expect(body.worlds).toContainEqual(expect.objectContaining({
    name: 'Local Bazaar World', status: 'unknown', players_online: null,
  }));
  const etag = response.headers().etag;
  expect(etag).toMatch(/^W\/"[a-f0-9]{64}"$/);
  const revalidated = await page.request.get('/api/v1/worlds', {headers: {'if-none-match': etag}});
  expect(revalidated.status()).toBe(304);
  expect(await revalidated.body()).toHaveLength(0);

  await open(page, '/worlds');
  await expect(page.locator('main h1')).toHaveText(t['worlds-title']);

  const other = lang === 'pl' ? 'en' : 'pl';
  await open(page, `/${lang}/`);
  await follow(page, page.locator(`#status a[href="/${lang}/worlds"]`), `/${lang}/worlds`);
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  await follow(page, page.locator(`footer.foot-interior a[hreflang="${other}"]`), `/${other}/worlds`);
  await expect(page.locator('main h1')).toHaveText(strings('crates/ui/i18n', other)['worlds-title']);
});

test('cold community lists and details disclose no rows to anonymous or player', async ({page}, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const base = `/${lang}`;
  const cold = [
    ['online', 'crates/ui/i18n', 'online-unavailable'],
    ['online?sort=level', 'crates/ui/i18n', 'online-unavailable'],
    ['deaths', 'crates/community/src/deaths', 'unavailable'],
    ['characters/Cold%20Reader/deaths', 'crates/community/src/deaths', 'unavailable'],
    ['guilds?q=Reader&page=2', 'templates/guilds', 'unavailable'],
    ['guilds/Cold%20Guild', 'templates/guilds', 'unavailable'],
    ['houses', 'templates/houses', 'unavailable'],
    [`houses/oteryn:content.house.reader_house?world=${world}`, 'templates/houses', 'unavailable'],
    ['characters/Cold%20Reader', 'templates/profiles', 'unavailable'],
    ['wiki/places', 'crates/wiki/src/places', 'unavailable'],
    ...['level', 'experience', 'magic', 'fist', 'club', 'sword', 'axe', 'distance', 'shielding', 'fishing']
      .map(kind => [`rankings/${kind}`, 'crates/ui/i18n', 'rankings-unavailable']),
  ];
  const bestiary = `${base}/atlas/bestiary/${character}`;
  await open(page, bestiary, 401);
  const places = `${base}/atlas/places`;
  const loginRedirect = await page.request.get(places, {maxRedirects: 0});
  expect(loginRedirect.status()).toBe(303);
  expect(loginRedirect.headers().location).toBe(`${base}/login`);
  for (const actor of ['anonymous', 'player']) {
    if (actor === 'player') {
      const account = await seedPlayer(lang);
      await login(page, lang, account);
      await expect(page).toHaveURL(new URL(`/${lang}/`, page.url()).href);
      await open(page, bestiary, 503);
      await open(page, places, 503);
      await open(page, `${bestiary}?stars=6`, 400);
    }
    for (const [path, directory, key] of cold) {
      await test.step(`${actor} GET ${path}`, async () => {
        await open(page, `${base}/${path}`, 503);
        await expect(page.locator('html')).toHaveAttribute('lang', lang);
        await expect(page.locator('main').getByText(strings(directory, lang)[key], {exact: true})).toBeVisible();
        await expect(page.locator('main article, main tbody tr')).toHaveCount(0);
      });
    }
  }
  for (const path of ['/online', '/deaths', '/characters/Cold%20Reader/deaths', '/guilds',
    '/guilds/Cold%20Guild', '/houses', `/houses/oteryn:content.house.reader_house?world=${world}`]) {
    await open(page, path, 503);
  }

  for (const [path, status] of [
    ['online?sort=invalid', 400], ['deaths?unknown=1', 400],
    ['guilds?page=0', 400], ['houses/oteryn:content.house.reader_house', 400],
    ['rankings/experience?page=0', 400], ['rankings/invalid', 404],
  ]) await open(page, `${base}/${path}`, status);

  await open(page, `${base}/characters`);
  const t = strings('templates/profiles', lang);
  const main = page.locator('main');
  await expect(main.getByRole('heading', {level: 1})).toHaveText(t.title);
  const input = main.getByLabel(t.name, {exact: true});
  const button = main.getByRole('button', {name: t.search, exact: true});
  await input.fill('.');
  await follow(page, button, `${base}/characters`, 400, {name: '.'});
  await expect(main.getByRole('status')).toHaveText(t.invalid);
  await input.fill('Cold Reader');
  await follow(page, button, `${base}/characters/Cold%20Reader`, 503);
  await expect(main.getByRole('status')).toHaveText(t.unavailable);
});

test('wiki, atlas and Today refuse unsupported Game reads', async ({page}, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const base = `/${lang}`;
  const ui = strings('crates/ui/i18n', lang);
  const entities = strings('crates/wiki/src/entities', lang);
  for (const [path, message] of [
    ['wiki', entities.unavailable],
    ['wiki/search?q=reader', entities.unavailable],
    ...wikiKinds.flatMap(kind => [
      [`wiki/${kind}`, entities.unavailable], [`wiki/${kind}/missing`, entities.unavailable],
    ]),
    ['atlas', ui['error-503-title']],
    ['atlas?x=100&y=100&z=7', ui['error-503-title']],
    ['atlas/delivery', strings('crates/atlas/src/delivery', lang).unavailable],
  ]) {
    await open(page, `${base}/${path}`, 503);
    await expect(page.locator('html')).toHaveAttribute('lang', lang);
    await expect(page.locator('main')).toContainText(message);
    await expect(page.locator('main time')).toHaveCount(0);
  }
  for (const path of ['/wiki', '/wiki/search?q=reader', '/wiki/items', '/wiki/items/missing']) {
    await open(page, path, 503);
  }
  await open(page, '/wiki/articles/community-e2e-missing', 404);
  await open(page, `${base}/wiki/articles/community-e2e-missing`, 404);
  await expect(page.locator('main h1')).toHaveText(strings('crates/wiki/src/i18n', lang).missing);
  await open(page, `${base}/wiki/search?q=x`, 400);
  await expect(page.locator('main h1')).toHaveText(entities.invalid);
  await open(page, `${base}/atlas/delivery?tab=invalid`, 400);
  const css = await page.request.get('/atlas/delivery.css');
  expect(css.status()).toBe(200);
  expect(css.headers()['content-type']).toContain('text/css');
  const atlasTile = await page.request.get(`/atlas/tiles/${'0'.repeat(40)}/7/0/0/0.webp`);
  expect(atlasTile.status()).toBe(503);
  const spawnText = await page.request.get(`/atlas/spawns-text/${lang}`);
  expect(spawnText.status()).toBe(200);
  const labelsForSpawns = await spawnText.json();
  expect(labelsForSpawns).toMatchObject(lang === 'pl' ? {
    spawns: 'Spawny potworów', fullscreen: 'Pełny ekran', place: 'Link do tego miejsca',
  } : {
    spawns: 'Monster spawns', fullscreen: 'Full screen', place: 'Link to this place',
  });
  const viewport = `version=${'0'.repeat(40)}&floor=7&min_y=100&max_y=105`;
  const spawn = await page.request.get(`/atlas/spawns?${viewport}&min_x=100&max_x=105`);
  expect(spawn.status()).toBe(503);
  expect(spawn.headers()['cache-control']).toContain('no-store');
  const reversed = await page.request.get(`/atlas/spawns?${viewport}&min_x=105&max_x=100`);
  expect(reversed.status()).toBe(400);
  const labels = strings('templates/today', lang);
  await open(page, `/today?lang=${lang}`);
  await open(page, `${base}/today`);
  await expect(page.locator('main h1')).toHaveText(lang === 'pl' ? 'Dziś' : 'Today');
  await expect(page.locator('main > [role=status]')).toHaveText(
    lang === 'pl' ? 'Część źródeł jest niedostępna.' : 'Some sources are unavailable.'
  );
  const daily = page.locator('main section[aria-labelledby=world-daily-title]');
  await expect(daily.getByRole('heading')).toHaveText(labels.title);
  await expect(daily.getByRole('status')).toHaveText(labels.unavailable);
  await expect(daily.locator('time')).toHaveCount(0);
  for (const prefix of ['', base]) {
    const data = await page.request.get(`${prefix}/today/data`);
    expect(data.status()).toBe(503);
    expect(data.headers()['cache-control']).toContain('no-store');
    expect(await data.json()).toEqual({status: 'unavailable'});
  }
  const widget = await open(page, `${base}/today/widget`, 503);
  expect(widget.headers()['content-type']).toContain('text/html');
  await expect(page.getByRole('status')).toHaveText(labels.unavailable);
  await expect(page.locator('time')).toHaveCount(0);
  const invalidWorld = await page.request.get(`${base}/today/data?world=invalid`);
  expect(invalidWorld.status()).toBe(400);
  expect(await invalidWorld.json()).toEqual({status: 'unavailable'});
  const alias = await page.request.get(`/today/widget?lang=${lang}`);
  expect(alias.status()).toBe(503);
  expect(await alias.text()).toContain(labels.unavailable);
});

test('wiki calculator submits each calculation and checks results and refusal', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const t = strings('templates/wiki/calculators', lang);
  const path = `/${lang}/wiki/calculators`;
  await open(page, path);
  const main = page.locator('main');
  await expect(main.getByRole('heading', {level: 1})).toHaveText(t.title);
  for (const key of ['estimate', 'training-help', 'loot-help']) {
    await expect(main.getByText(t[key], {exact: true})).toBeVisible();
  }
  for (const [calculator, inputs, status, result] of [
    ['experience', [['current', '0', 'current'], ['target', '2', 'target']], 503],
    ['stamina', [['current', '2400', 'minutes'], ['offline', '600', 'offline']], 200, '2400'],
    ['training', [['remaining', '100', 'remaining'], ['family', '1', 'family'], ['dummy', 'reader', 'dummy']], 503],
    ['loot', [['total', '101', 'total'], ['members', '4', 'members']], 200, '25; 1'],
  ]) {
    const section = main.locator('section').filter({has: page.getByRole('heading', {name: t[calculator], exact: true})});
    for (const [, value, label] of inputs) await section.getByLabel(t[label], {exact: true}).fill(value);
    const params = Object.fromEntries(inputs.map(([key, value]) => [key, value]));
    await follow(page, section.getByRole('button', {name: t.calculate}), path, status, {calculator, ...params});
    if (status === 503) {
      await expect(main.getByRole('alert')).toHaveText(t.unavailable);
      await expect(main.locator('output')).toHaveCount(0);
    } else {
      await expect(main.getByRole('heading', {name: t[`${calculator}-result`]})).toBeVisible();
      await expect(main.locator('output')).toHaveText(result);
    }
  }
  await open(page, `${path}?calculator=loot&total=101&members=0`, 400);
  await expect(main.getByRole('alert')).toHaveText(t.invalid);
  const other = lang === 'pl' ? 'en' : 'pl';
  await follow(page, page.locator(`header:not(#portal-header) a[href="/${other}/wiki/calculators"][hreflang="${other}"]`), `/${other}/wiki/calculators`);
  await expect(page.locator('html')).toHaveAttribute('lang', other);
});
