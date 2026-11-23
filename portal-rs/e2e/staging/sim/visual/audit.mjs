import { mkdir, writeFile } from 'node:fs/promises';
import { measure } from './fingerprint.mjs';
import { goto, origin, withApi } from './helpers.mjs';

const menuSelector = [
  'body > header', 'body > footer', '#podglad-konta', 'main nav',
  'main .op-tabs', 'main [class*="breadcrumb"]',
].map(selector => `${selector} a[href]`).join(', ');

// These cover templates and variants outside the global menus. Parameterized
// articles and entities are discovered from rendered links, never guessed.
const publicTemplates = [
  'login', 'register', 'forgot-password', 'register/check-inbox', 'mfa/login',
  'characters', 'search', 'help', 'rules', 'server-information', 'legal/cookies',
  'wiki/search', 'wiki/places', 'stats/rankings/yesterday',
  'stats/rankings/week', 'stats/rankings/month', 'stats/rankings/record',
  'stats/rankings/oldest', 'stats/rankings/anniversaries', 'stats/rankings/name-changes',
  'stats/rankings/transfers', 'stats/rankings/sold', 'collection-pvp', 'bazaar/items',
];
const playerTemplates = [
  'account/dashboard', 'account/badges', 'account/security', 'account/security/email',
  'account/consents', 'account/privacy/delete', 'account/privacy/export', 'account/sudo',
  'mfa', 'account/hunts/compare', 'account/journal/reports/notes', 'shop/coins', 'shop/history',
  'shop/transfer', 'panel/premium', 'panel/payments', 'support/tickets/create',
  'support/reports/create', 'account/bazaar/sell', 'bazaar/items/new',
];
const staffTemplates = {
  'content-editor': [
    '/admin', '/admin/news', '/admin/news/create', '/admin/pages', '/admin/media',
    '/admin/events', '/admin/events/create', '/admin/wiki',
    '/admin/console/players', '/admin/console/audit',
  ],
  moderator: [
    '/admin', '/admin/players', '/admin/reports', '/admin/sanctions',
    '/admin/console/players', '/admin/console/audit',
  ],
};
const optional = /\/(?:mfa\/login|collection-pvp|bazaar\/items|shop(?:\/|$)|panel\/(?:premium|payments)|account\/bazaar\/sell|wiki\/places)(?:\/|$)/;
const unavailable = /\/(?:online|rankings|deaths|guilds|houses|wiki|atlas|stats|party|league|puzzles|collection|pass|shop|panel|bazaar|account\/(?:game-logins|hunts|journal|characters))(?:[/?]|$)/;
const unavailableCopy = /unavailable|not (?:available|configured)|try again later|niedostępn|spróbuj ponownie|brak danych/i;

function routeKey(path) {
  return path.replace(/^\/(pl|en)(?=\/)/, '/:locale')
    .replace(/\?(?:lang)=(pl|en)$/, '')
    .replace(/\b[\da-f]{8}-[\da-f]{4}-[\da-f]{4}-[\da-f]{4}-[\da-f]{12}\b/gi, ':id')
    .replace(/(\/(?:news|events)\/)visual-[a-f0-9]+/g, '$1:visual-item');
}
function screenName(path) {
  return routeKey(path).replace(/^\/:locale\/?/, '').replace(/^\//, '')
    .replace(/[^a-zA-Z0-9_-]/g, '_').slice(0, 108) || 'home';
}
function candidate(href, language) {
  const url = new URL(href);
  if (url.origin !== origin || (url.search && !/^\?lang=(pl|en)$/.test(url.search))) return null;
  if (/\/(?:verify-email|reset-password|logout|unsubscribe|receipt|download|export)(?:\/|$)/.test(url.pathname)) return null;
  if (/^\/(?:api|language|oauth|static)(?:\/|$)/.test(url.pathname)) {
    if (!/^\/static\/thais\/shop-(?:pl|en)\.html$/.test(url.pathname)) return null;
  }
  const locale = /^\/(pl|en)(?:\/|$)/.exec(url.pathname)?.[1] || url.searchParams.get('lang')
    || /shop-(pl|en)\.html$/.exec(url.pathname)?.[1];
  if (locale && locale !== language) return null;
  url.hash = '';
  return url.pathname + url.search;
}

async function links(page, language) {
  const rows = await page.locator(menuSelector).evaluateAll(elements => elements.map(el => el.href));
  const articles = await page.locator('main a[href]').evaluateAll(elements => {
    const paths = elements.filter(el => /^\/(?:pl|en)\/(?:news|events|wiki|characters)\/[^/]+$/.test(el.pathname));
    return paths.slice(0, 8).map(el => el.href);
  });
  for (const href of rows) new URL(href); // Validate external hrefs in place.
  return {
    menu: [...new Set(rows.map(href => candidate(href, language)).filter(Boolean))],
    content: [...new Set(articles.map(href => candidate(href, language)).filter(Boolean))],
  };
}

async function interactiveStates(page) {
  const result = {};
  const css = el => {
    const s = getComputedStyle(el);
    return Object.fromEntries(['color', 'backgroundColor', 'borderTop', 'outline',
      'outlineOffset', 'boxShadow', 'textDecorationLine'].map(key => [key, s[key]]));
  };
  for (const [kind, selector] of Object.entries({
    link: 'main a[href]:not(.op-button):visible',
    primary: '.op-button--primary:visible',
    input: 'main input:not([type=hidden],[type=checkbox],[type=radio]):visible, main select:visible, main textarea:visible',
  })) {
    const target = page.locator(selector).first();
    if (!await target.count()) continue;
    result[kind] = { normal: await target.evaluate(css) };
    if (kind === 'input') {
      // Keyboard focus invokes :focus-visible. Programmatic focus can inherit
      // pointer modality after earlier navigation.
      await page.keyboard.press('Tab');
      await target.focus();
      result[kind].focus = await target.evaluate(css);
      await target.blur();
    } else {
      try {
        await target.hover({ timeout: 750 });
        result[kind].hover = await target.evaluate(css);
        await page.mouse.move(0, 0);
      } catch {
        result[kind].hover = null;
      }
    }
  }
  // getComputedStyle intentionally suppresses visited history. Record the
  // author rules available from same-origin CSS instead.
  result.visitedRules = await page.evaluate(() => {
    const collect = rules => [...rules].flatMap(rule => {
      if (rule.selectorText?.includes(':visited')) return [rule.cssText];
      if (rule.cssRules) return collect(rule.cssRules);
      return [];
    });
    return [...document.styleSheets].flatMap(sheet => {
      try { return collect(sheet.cssRules); } catch { return []; }
    });
  });
  return result;
}

async function prepare(page) {
  await page.waitForLoadState('load', { timeout: 8_000 }).catch(() => {});
  await page.evaluate(async () => {
    await document.fonts.ready;
    for (const img of document.images) img.loading = 'eager';
    await Promise.race([
      Promise.all([...document.images].map(img => img.decode().catch(() => {}))),
      new Promise(resolve => setTimeout(resolve, 2_500)),
    ]);
  });
}

export async function capture(page, info, group, path, { states = true } = {}) {
  await prepare(page);
  const measured = await page.evaluate(measure);
  const interactions = states ? await interactiveStates(page) : {};
  await page.evaluate(() => window.scrollTo(0, 0));
  const screenshot = `${group}/${screenName(path)}.png`;
  await page.screenshot({
    path: info.outputPath(screenshot),
    fullPage: true,
    animations: 'disabled',
    timeout: 15_000,
    mask: [
      page.locator('input[type=password], textarea[readonly], img[src^="data:"]'),
      page.locator('form[action$="/mfa/confirm"]').locator('..').locator('p.op-prose'),
    ],
  });
  return { ...measured, interactions, screenshot };
}

const pageLimit = 95;
export class ActorSessionLost extends Error {}

export async function crawl(page, info, actor, language, viewport) {
  await page.setViewportSize({ width: viewport, height: { 1440: 900, 768: 1024, 390: 844 }[viewport] });
  await page.emulateMedia({ colorScheme: 'light' });
  const home = await goto(page, `/${language}/`);
  if (home?.status() !== 200) throw new Error(`Portal home HTTP ${home?.status()}`);
  if (actor !== 'anonymous' && !(await page.locator('[data-account-link]').count())) {
    throw new ActorSessionLost('The authenticated session was lost before the audit slice');
  }
  const light = await page.evaluate(() => [
    getComputedStyle(document.body).backgroundColor, getComputedStyle(document.body).color,
    getComputedStyle(document.body).colorScheme, getComputedStyle(document.querySelector('main')).backgroundColor,
  ]);
  await page.emulateMedia({ colorScheme: 'dark' });
  const dark = await page.evaluate(() => [
    getComputedStyle(document.body).backgroundColor, getComputedStyle(document.body).color,
    getComputedStyle(document.body).colorScheme, getComputedStyle(document.querySelector('main')).backgroundColor,
  ]);
  const homeResponds = JSON.stringify(light) !== JSON.stringify(dark);
  await page.emulateMedia({ colorScheme: 'light' });

  const queue = new Map([[`/${language}/`, { source: 'home' }]]);
  const checked = new Map();
  const visited = new Set();
  const pages = [];
  const problems = [];
  const inventory = [];
  const outliers = new Map();
  function append(path, source, from) {
    if (actor === 'anonymous' && /^\/(admin|console|account|(?:pl|en)\/account)(?:\/|$)/.test(path)) return;
    if (source === 'menu' && from !== path && !menuEvidence.has(path)) menuEvidence.set(path, from);
    if (checked.has(path) || queue.has(path)) return;
    queue.set(path, { source });
  }
  for (const suffix of publicTemplates) append(`/${language}/${suffix}`, 'template');
  if (actor !== 'anonymous') {
    append('/account', 'template');
    for (const suffix of playerTemplates) append(`/${language}/${suffix}`, 'template');
    for (const path of staffTemplates[actor] || []) append(`${path}?lang=${language}`, 'template');
  }
  const aliases = new Map();
  const refusedMenu = new Set();
  const menuEvidence = new Map();
  while (queue.size && checked.size < pageLimit) {
    const [path, { source }] = queue.entries().next().value;
    queue.delete(path);
    const response = await goto(page, path);
    const status = response?.status();
    const current = new URL(page.url());
    const destination = current.pathname + current.search;
    checked.set(path, status);
    inventory.push({ route: routeKey(path), source, status, destination: routeKey(destination) });
    const issue = (kind, expected, actual) => problems.push({ path: routeKey(path), kind, component: source, expected, actual });
    if (status === 429) {
      issue('shared egress', 'audit can load the page after one minute', 'HTTP 429 persisted');
      continue;
    }
    const isUnavailable = status === 503 && unavailable.test(path)
      && unavailableCopy.test(await page.locator('main').first().innerText().catch(() => ''));
    if (status >= 500 && !isUnavailable) issue('HTTP failure', 'a rendered page or documented unavailable state', `HTTP ${status}`);
    if ([403, 404].includes(status) && source === 'template' && !optional.test(path)) {
      // Keep source-only/feature-gated inventory distinct from advertised links.
      inventory.at(-1).state = 'unmounted or refused';
    }
    if (isUnavailable) inventory.at(-1).state = 'unavailable';
    if (actor !== 'anonymous' && !path.endsWith('/login') && current.pathname.endsWith('/login')
      && /^\/(?:admin|account|(?:pl|en)\/account)(?:\/|$)/.test(path)) {
      const probe = await goto(page, `/${language}/`);
      if (probe?.status() === 200 && !(await page.locator('[data-account-link]').count())) {
        throw new ActorSessionLost('The authenticated session was lost during the audit slice');
      }
      await goto(page, destination);
    }
    if (!response?.headers()['content-type']?.includes('text/html') || current.origin !== origin) continue;
    if (visited.has(destination)) {
      aliases.set(routeKey(path), routeKey(destination));
      continue;
    }
    visited.add(destination);
    const discovered = await links(page, language);
    for (const link of discovered.menu) append(link, 'menu', path);
    for (const link of discovered.content) append(link, 'content', path);

    const group = `${actor}-${language}-${viewport}`;
    const lightView = await capture(page, info, `${group}-light`, destination);
    await page.emulateMedia({ colorScheme: 'dark' });
    const darkView = await page.evaluate(measure);
    const themeResponds = homeResponds || JSON.stringify(darkView.theme) !== JSON.stringify(lightView.theme)
      || JSON.stringify(darkView.palette.colors) !== JSON.stringify(lightView.palette.colors);
    const view = themeResponds ? { light: lightView, dark: await capture(page, info, `${group}-dark`, destination) }
      : { light: lightView };
    await page.emulateMedia({ colorScheme: 'light' });
    for (const [scheme, entry] of Object.entries(view)) {
      for (const problem of entry.problems) problems.push({ path: routeKey(destination), scheme,
        screenshot: entry.screenshot, ...problem });
      for (const [category, values] of Object.entries(entry.palette)) {
        for (const value of values) {
          const key = JSON.stringify([category, value]);
          if (!outliers.has(key)) outliers.set(key, new Set());
          outliers.get(key).add(routeKey(destination));
        }
      }
    }
    pages.push({ path: routeKey(destination), status, views: view });
  }
  for (const row of inventory) {
    const original = [...checked.keys()].find(path => routeKey(path) === row.route);
    const from = menuEvidence.get(original);
    row.menuFrom = from ? routeKey(from) : undefined;
    if (from && [403, 404].includes(row.status) && !refusedMenu.has(row.route)) {
      refusedMenu.add(row.route);
      problems.push({ path: row.route, kind: 'menu destination', component: 'menu',
        expected: 'an advertised destination leads to a usable page or a documented unavailable state',
        actual: `HTTP ${row.status} from ${routeKey(from)}`,
        screenshot: pages.find(page => page.path === row.destination)?.views.light.screenshot });
    }
  }
  if (queue.size) problems.push({ kind: 'crawl bound', expected: `at most ${pageLimit} routes per actor`,
    actual: `${queue.size} remaining candidates` });
  const favicon = await withApi(async get => {
    const response = await get('/favicon.ico');
    return [response.status(), response.headers()['content-type']];
  }).catch(() => []);
  if (favicon[0] !== 200 || !favicon[1]?.startsWith('image/')) {
    problems.push({ path: `/${language}/`, kind: 'favicon', component: 'document',
      expected: 'a loadable favicon', actual: JSON.stringify(favicon) });
  }
  for (const page of pages) for (const view of Object.values(page.views)) {
    view.favicon = view.favicon === '/favicon.ico' ? 'default favicon' : 'explicit favicon link';
  }

  compareStyles(pages, problems);
  const paletteOutliers = [...outliers.entries()].filter(([, where]) => where.size <= 2).map(([key, where]) => ({
    token: JSON.parse(key), paths: [...where].sort(),
  }));
  const result = { actor, language, viewport, homeResponds, favicon, inventory,
    aliases: Object.fromEntries(aliases), pages, paletteOutliers, problems };
  await save(info, `${actor}-${language}-${viewport}.json`, result);
  return result;
}

function compareStyles(pages, problems) {
  const shell = ['backgroundColor', 'color', 'fontFamily'];
  const typography = ['fontFamily', 'fontSize', 'fontWeight', 'lineHeight'];
  const control = [...typography, 'backgroundColor', 'color', 'borderTop', 'borderRadius', 'padding'];
  const choices = {
    header: shell, footer: shell, nav: shell, logo: [],
    primary: control, secondary: control, danger: control,
    input: control, select: control, textarea: control,
    tableHeader: control, tableCell: control, h1: typography,
  };
  for (const scheme of ['light', 'dark']) for (const [component, keys] of Object.entries(choices)) {
    const variants = new Map();
    for (const page of pages.filter(item => item.status === 200 && item.views[scheme])) {
      const part = page.views[scheme].components[component];
      const css = part.map(({ css, boxes }) => {
        const values = Object.fromEntries(keys.map(key => [key, css[key]]));
        if (boxes?.length && ['header', 'footer', 'nav', 'logo'].includes(component)) {
          values.boxes = boxes.map(([x, y, width, height]) =>
            (component === 'footer' || component === 'logo'
              ? [x, width, height] : [x, y, width, height]).map(Math.round));
        }
        return JSON.stringify(values);
      }).sort();
      if (!css.length && !['header', 'footer', 'logo'].includes(component)) continue;
      const signature = '[' + css.join(',') + ']';
      if (!variants.has(signature)) variants.set(signature, []);
      variants.get(signature).push(page);
    }
    const [baseline, ...rest] = [...variants].sort((a, b) => b[1].length - a[1].length);
    for (const [actual, affected] of rest) for (const page of affected) {
      problems.push({ path: page.path, scheme, screenshot: page.views[scheme].screenshot, kind: 'style drift', component,
        expected: `${baseline[0]} (modal style, shell boxes rounded to a CSS pixel)`, actual });
    }
  }
}

export async function save(info, name, result) {
  const filename = info.outputPath(name);
  await mkdir(info.outputDir, { recursive: true });
  await writeFile(filename, JSON.stringify(result, null, 2) + '\n');
}

export function summary(results) {
  const counts = new Map();
  for (const result of results) for (const issue of result.problems) {
    const key = `${issue.kind} / ${issue.component ?? 'audit'}`;
    counts.set(key, (counts.get(key) ?? 0) + 1);
  }
  return [...counts].map(([key, count]) => `${key} (${count})`).sort();
}
