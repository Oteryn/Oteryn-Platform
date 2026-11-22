import { readFileSync } from 'node:fs';
import { test, expect } from '../fixtures.mjs';
import { messages, visit } from '../public-depth-helpers.mjs';

test('unprefixed wiki routes negotiate locale and stats aliases default to Polish', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const wiki = messages('crates/wiki/src/entities', lang);
  const article = messages('crates/wiki/src/i18n', lang);
  for (const route of ['/wiki', '/wiki/search?q=health+potion&page=2', '/wiki/items', '/wiki/items/absent-e2e']) {
    await visit(page, route, 503, { lang, message: wiki.unavailable });
  }
  await visit(page, '/wiki/articles/absent-e2e', 404, { lang, message: article.missing });

  const world = messages('templates/stats/world', 'pl');
  await visit(page, '/stats/world', 200, { lang: 'pl', message: world['world-title'] });
  await expect(page.locator('select[name=world]')).toBeVisible();
  const guilds = messages('templates/stats/guilds', 'pl');
  await visit(page, '/stats/guilds', 503, { lang: 'pl', message: guilds['guild-unavailable'] });
});

const resources = [
  'crates/wiki/src/i18n', 'crates/wiki/src/entities', 'crates/wiki/src/places', 'templates/wiki/calculators',
  'templates/atlas', 'crates/atlas/src/delivery', 'crates/atlas/src/bestiary',
  'templates/stats/world', 'templates/stats/guilds', 'templates/stats/character', 'templates/stats/rankings',
  'crates/playerdata/src/places', 'templates/pages', 'templates/events', 'templates/today',
  'crates/minigames/src/league', 'crates/minigames/src/puzzles',
  'crates/minigames/src/collection', 'crates/minigames/src/collection_pvp',
];

test('public feature catalogue keys have Polish and English definitions', () => {
  const keys = path => [...readFileSync(new URL(`../../${path}.ftl`, import.meta.url), 'utf8')
    .matchAll(/^([a-zA-Z][\w-]*)\s*=/gm)].map(([, key]) => key).sort();
  for (const directory of resources) {
    const en = keys(`${directory}/en`);
    expect(en.length, directory).toBeGreaterThan(0);
    expect(new Set(en).size, directory).toBe(en.length);
    expect(keys(`${directory}/pl`), directory).toEqual(en);
  }
  expect(keys('crates/atlas/src/spawns_pl')).toEqual(keys('crates/atlas/src/spawns_en'));
  const prefixes = /^(worlds|rankings)-/;
  const en = keys('crates/ui/i18n/en').filter(key => prefixes.test(key));
  expect(en.length).toBeGreaterThan(0);
  expect(keys('crates/ui/i18n/pl').filter(key => prefixes.test(key))).toEqual(en);
});
