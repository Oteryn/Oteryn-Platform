import fs from 'node:fs';

const inventory = JSON.parse(fs.readFileSync(
  process.env.PORTAL_QUALITY_INVENTORY ?? new URL('../inventory.json', import.meta.url),
));
const expectations = JSON.parse(fs.readFileSync(new URL('../route-expectations.json', import.meta.url)));
const missingId = '019a541b-a000-7000-8000-000000000001';

export const maxSamples = 3;
const kinds = {
  wiki: ['items', 'creatures', 'spells'],
  community: ['experience', 'magic', 'distance'],
  stats: ['yesterday', 'week', 'month'],
};
// This launcher supplies no native Game producer. These browser routes
// explicitly fail closed when Bazaar or character creation authority is absent.
const coldGameRoutes = new Set([
  '/{lang}/bazaar', '/{lang}/account/bazaar', '/{lang}/bazaar/{listing}',
  '/{lang}/account/bazaar/sell', '/{lang}/bazaar/items', '/{lang}/bazaar/items/new',
  '/{lang}/bazaar/items/{listing}', '/{lang}/bazaar/items/{id}',
  '/{lang}/account/characters/requests/{operation}', '/{lang}/account/characters/suggest',
]);

function candidates(pattern, locale, ids) {
  const replacements = {
    lang: [locale], world: [ids.world], id: pattern.startsWith('/admin/players/') && ids.player ? [ids.player] :
      pattern.includes('/support/tickets/') && ids.ticket ? [ids.ticket, missingId] :
      pattern.includes('/support/reports/') && ids.report ? [ids.report, missingId] : [missingId],
    kind: pattern.includes('/wiki/') ? kinds.wiki :
      pattern.includes('/stats/') ? kinds.stats : kinds.community,
    tab: ['experience', 'history', 'deaths'],
    slug: pattern.includes('/pages/') ? ['rules', 'help', 'support'] : ['missing-quality'],
    name: ['MissingQuality'], character: ['MissingQuality'], family: ['missing-quality'],
    key: ['missing-quality'], channel: ['stable'], filename: ['timestamp.json'],
    floor: ['0'], zoom: ['0'], x: ['0'], y: ['0'],
  };
  let paths = [pattern];
  for (const [token, key] of pattern.matchAll(/\{([^}]+)\}/g)) {
    paths = paths.flatMap(value =>
      (replacements[key] ?? [missingId]).map(replacement => value.replace(token, replacement)),
    ).slice(0, maxSamples);
  }
  return paths;
}

export function routes(locale, ids) {
  const getRoutes = [...inventory.routes.filter(route => route.methods.includes('GET')),
    ...expectations.filter(route => route.method === 'GET')];
  const patterns = new Set(getRoutes.map(({ route }) => route));
  for (const { path } of inventory.sections) {
    patterns.add(path.startsWith('/') ? path : '/{lang}/' + path);
  }
  const landing = ['/{lang}/', '/{lang}/login', '/account', '/admin'];
  return [...new Set([...landing.filter(pattern => patterns.has(pattern)), ...patterns])].map(pattern => ({
    pattern, paths: candidates(pattern, locale, ids),
    documentedStatuses: coldGameRoutes.has(pattern) ? [503] :
      expectations.filter(row => row.route === pattern && row.method === 'GET').map(row => row.status),
  }));
}

export function matches(pattern, pathname) {
  const regex = pattern.split(/(\{[^}]+\})/).map(part => {
    if (part.startsWith('{*')) return '.+';
    if (part.startsWith('{')) return '[^/]+';
    return part.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  }).join('');
  return new RegExp(`^${regex}$`).test(pathname);
}

function routePrecedence(pattern) {
  return pattern.split('/').map(segment =>
    segment.startsWith('{*') ? '0' : segment.startsWith('{') ? '1' : '2',
  ).join('');
}

export function matchingRoute(inventory, pathname) {
  let result;
  let precedence = '';
  for (const entry of inventory) {
    if (!matches(entry.pattern, pathname)) continue;
    const rank = routePrecedence(entry.pattern);
    if (rank <= precedence) continue;
    result = entry;
    precedence = rank;
  }
  return result;
}

export function documentLanguage(url, pattern, locale) {
  const { pathname, searchParams } = url;
  const prefix = pathname.match(/^\/(pl|en)(?:\/|$)/)?.[1];
  if (prefix) return prefix;
  const segment = pattern.split('/').indexOf('{lang}');
  const explicit = pathname.split('/')[segment];
  if ((explicit === 'pl' || explicit === 'en') && matches(pattern, pathname)) return explicit;
  // These legacy editorial routes expose an explicit language filter.
  if (/^\/(?:news(?:\/[^/]+)?|admin\/(?:media|news(?:\/create|\/[^/]+\/edit)?))$/.test(pathname)) {
    const filter = searchParams.get('lang');
    if (filter === 'pl' || filter === 'en') return filter;
  }
  return locale;
}
