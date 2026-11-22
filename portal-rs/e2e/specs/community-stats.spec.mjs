import { test, expect } from '../fixtures.mjs';
import { follow, open, strings } from '../community-helpers.mjs';

const character = '019a0000-0000-7000-8000-000000000001';
const second = '019a0000-0000-7000-8000-000000000002';
const start = '2026-11-01';
const end = '2026-11-02';

test('world registry picker and date range preserve the privacy gate', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const path = `/${lang}/stats/world`;
  const t = strings('templates/stats/world', lang);
  await open(page, '/stats/world');
  await open(page, path);
  const main = page.locator('main');
  await expect(main.getByRole('heading', {level: 1})).toHaveText(t['world-title']);
  await expect(main.getByText(t['world-note'])).toBeVisible();
  await expect(main.getByText(t['world-select'])).toBeVisible();
  await expect(main.getByRole('heading', {name: t['world-kills']})).toBeVisible();
  const picker = main.getByLabel(t['world-picker'], {exact: true});
  const [world] = await picker.selectOption({label: 'Local Bazaar World'});
  expect(world).toMatch(/^[\da-f-]{36}$/);
  await main.getByLabel(t['world-start']).fill(start);
  await main.getByLabel(t['world-end']).fill(end);
  await follow(page, main.getByRole('button', {name: t['world-show']}), path, 503, {world, start, end});
  await expect(main.getByText(t['world-unavailable'], {exact: true}).first()).toBeVisible();
  await expect(main.getByRole('heading', {name: t['world-census']})).toHaveCount(0);
  await open(page, `${path}?world=${world}&start=${end}&end=${start}`, 400);
});

test('growth ranking links and filter controls retain the unavailable state', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const path = `/${lang}/stats/rankings`;
  const t = strings('templates/stats/rankings', lang);
  await open(page, `${path}/yesterday`, 503);
  const main = page.locator('main');
  await expect(main.getByRole('heading', {level: 1})).toHaveText(t.title);
  await expect(main.getByText(t.hint, {exact: true})).toBeVisible();
  const kinds = ['yesterday', 'week', 'month', 'record', 'oldest', 'anniversaries', 'name-changes', 'transfers', 'sold'];
  for (const kind of kinds) {
    const link = main.getByRole('link', {name: t[kind], exact: true});
    await expect(link).toHaveAttribute('href', `${path}/${kind}`);
    await follow(page, link, `${path}/${kind}`, 503);
    await expect(main.getByRole('status')).toHaveText(t.unavailable);
    await expect(main.locator('article')).toHaveCount(0);
  }

  const vocation = main.getByLabel(t.vocation, {exact: true});
  const pvp = main.getByLabel(t.pvp, {exact: true});
  await expect(main.getByLabel(t.world, {exact: true}).locator('option')).toHaveText([t.all]);
  await expect(vocation.locator('option')).toHaveText([t.all, ...Array.from({length: 6}, (_, i) => t[`vocation-${i}`])]);
  await expect(pvp.locator('option')).toHaveText([t.all, t.open, t.optional, t.hardcore]);
  for (const rule of ['open', 'optional', 'hardcore']) {
    await vocation.selectOption('2');
    await pvp.selectOption(rule);
    await follow(page, main.getByRole('button', {name: t.apply}), `${path}/sold`, 503,
      {world: '', vocation: '2', pvp: rule});
    await expect(main.getByRole('status')).toHaveText(t.unavailable);
  }
  for (const query of ['world=invalid', 'vocation=9', 'pvp=invalid', 'page=0']) {
    await open(page, `${path}/sold?${query}`, 400);
    await expect(main.getByRole('status')).toHaveText(t.invalid);
  }
  await open(page, `${path}/missing`, 404);
  await expect(main.getByRole('status')).toHaveText(t['not-found']);
});

test('guild and character comparison submit filters and enforce date and identity bounds', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const guilds = `/${lang}/stats/guilds`;
  const guild = strings('templates/stats/guilds', lang);
  await open(page, '/stats/guilds', 503);
  await open(page, guilds, 503);
  const main = page.locator('main');
  await expect(main.getByRole('heading', {level: 1})).toHaveText(guild['guild-title']);
  for (const key of ['guild-note', 'guild-extras']) {
    await expect(main.getByText(guild[key], {exact: true})).toBeVisible();
  }
  await main.getByLabel(guild['guild-guild'], {exact: true}).fill('First Guild');
  await main.getByLabel(guild['guild-compare'], {exact: true}).fill('Second Guild');
  const days = main.getByLabel(guild['guild-days'], {exact: true});
  await days.fill('32');
  expect(await days.evaluate(input => input.validity.rangeOverflow)).toBe(true);
  await days.fill('14');
  await follow(page, main.getByRole('button', {name: guild['guild-submit'], exact: true}), guilds, 503,
    {guild: 'First Guild', compare: 'Second Guild', days: '14'});
  await expect(main.getByRole('status')).toHaveText(guild['guild-unavailable']);
  await open(page, `${guilds}?compare=Second`, 400);
  await open(page, `${guilds}?days=32`, 400);

  const compare = `/${lang}/stats/compare`;
  const t = strings('templates/stats/character', lang);
  await open(page, compare);
  await expect(main.getByRole('heading', {level: 1})).toHaveText(t.title);
  await expect(main.getByRole('status')).toHaveText(t.empty);
  await expect(main.getByText(t.hint, {exact: true})).toBeVisible();
  const idsInput = main.getByLabel(t.ids, {exact: true});
  const metric = main.getByLabel(t.metric, {exact: true});
  const metrics = ['level', 'experience', 'magic_level', 'fist_level', 'club_level', 'sword_level',
    'axe_level', 'distance_level', 'shielding_level', 'fishing_level'];
  await expect(metric.locator('option')).toHaveText(metrics.map(key => t[key]));
  const ids = `${character},${second}`;
  for (const key of metrics) {
    await idsInput.fill(ids);
    await metric.selectOption(key);
    await main.getByLabel(t.start, {exact: true}).fill(start);
    await main.getByLabel(t.end, {exact: true}).fill(end);
    await follow(page, main.getByRole('button', {name: t.apply, exact: true}), compare, 503,
      {ids, metric: key, start, end});
    await expect(main.getByRole('status')).toHaveText(t.unavailable);
    await expect(main.locator('figure, article')).toHaveCount(0);
  }
  await idsInput.fill('invalid');
  await main.getByLabel(t.start, {exact: true}).fill(start);
  await main.getByLabel(t.end, {exact: true}).fill(end);
  await follow(page, main.getByRole('button', {name: t.apply, exact: true}), compare, 400, {ids: 'invalid'});
  await expect(main.getByRole('status')).toHaveText(t.invalid);

  for (const tab of ['history', 'experience', 'rankings', 'deaths', 'progress', 'online']) {
    await open(page, `/${lang}/stats/characters/${character}/${tab}`, 503);
    await expect(main.getByRole('status')).toHaveText(t.unavailable);
  }
  await open(page, `/${lang}/stats/characters/${character}/missing`, 404);
  await expect(main.getByRole('status')).toHaveText(t['not-found']);
});
