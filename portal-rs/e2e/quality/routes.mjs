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
    documentedStatuses: expectations.filter(row => row.route === pattern && row.method === 'GET')
      .map(row => row.status),
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
