import fs from 'node:fs';
import path from 'node:path';
import { test, expect } from '../fixtures.mjs';
import { login, password, seedAccount, seedPasswordRange, waitForWikiBudget } from '../helpers.mjs';
import { coldView, emptyGameFixture, protectedDestination } from '../crawl-contracts.mjs';
import { enrollMfa, loginWithRecoveryCode, query } from '../staff-rbac-helpers.mjs';

const read = path => fs.readFileSync(new URL(path, import.meta.url), 'utf8');
const inventory = JSON.parse(read('../inventory.json'));
const pattern = route => new RegExp('^' + route.split(/(\{[^}]+\})/).map(part => {
  if (part === '{lang}') return '(?:pl|en)';
  if (part.startsWith('{*')) return '.+';
  if (part.startsWith('{')) return '[^/]+';
  return part.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}).join('') + '$');
const MAX_SAMPLES = 3;
const sections = [
  'home', 'admin', 'account', 'identity', 'services', 'wiki', 'atlas', 'community', 'stats', 'public',
];
const sectionOf = pathname => {
  const [top] = pathname.replace(/^\/(?:pl|en)(?=\/|$)/, '').split('/').slice(1);
  if (!top) return 'home';
  if (['admin', 'console'].includes(top)) return 'admin';
  if (top === 'account' || top === 'panel') return 'account';
  if (['login', 'register', 'verify-email', 'forgot-password', 'reset-password', 'mfa', 'password',
    'email-change', 'recovery-key'].includes(top)) return 'identity';
  if (top === 'support' || top === 'shop') return 'services';
  if (top === 'wiki' || top === 'search') return 'wiki';
  if (top === 'atlas') return 'atlas';
  if (top === 'stats' || top === 'rankings') return 'stats';
  if (['characters', 'deaths', 'guilds', 'houses', 'worlds', 'online', 'today', 'bazaar',
    'events', 'party', 'league', 'pass', 'collection', 'collection-pvp'].includes(top)) return 'community';
  return 'public';
};
const routes = inventory.routes.filter(row => row.methods.includes('GET'))
  .map(({route}) => ({route, matches: pattern(route)}))
  .sort((a, b) => b.route.replace(/\{[^}]+\}/g, '').length - a.route.replace(/\{[^}]+\}/g, '').length);
const emptyGame = emptyGameFixture(process.env);

const normalize = value => value.normalize('NFC').replace(/\s+/g, ' ').trim();
const catalogues = Object.fromEntries(['pl', 'en'].map(locale => [
  locale,
  new Map([...read(`../../crates/ui/i18n/${locale}.ftl`).matchAll(/^([\w-]+)\s*=([^\n]*)/gm)]
    .map(([, key, value]) => [key, normalize(value)])),
]));
const catalogueValues = Object.fromEntries(Object.entries(catalogues)
  .map(([locale, catalogue]) => [locale, new Set(catalogue.values())]));
const rawKeys = new Set([...catalogues.pl.keys(), ...catalogues.en.keys()]
  .filter(key => !catalogueValues.pl.has(key) && !catalogueValues.en.has(key)));

function sameOrigin(value, base, origin) {
  try {
    const url = new URL(value, base);
    if (url.origin !== origin || url.username || url.password) return null;
    url.hash = '';
    return url;
  } catch {
    return null;
  }
}

function label(url) {
  const keys = [...new Set(url.searchParams.keys())].sort();
  return url.pathname + (keys.length ? '?' + keys.map(encodeURIComponent).join('&') : '');
}

function pagePattern(url) {
  const route = routes.find(candidate => candidate.matches.test(url.pathname))?.route;
  const locale = url.pathname.match(/^\/(pl|en)(?:\/|$)/)?.[1];
  return (route ?? url.pathname).replace('{lang}', locale ?? '') +
    (url.search ? '?' + [...new Set(url.searchParams.keys())].sort().join('&') : '');
}

async function inspectPage(page, url, defects) {
  const result = await page.evaluate(() => {
    const chrome = document.querySelectorAll(
      '#portal-header, #podglad-konta, footer.foot, body > a[href="#main"]',
    );
    const strings = new Set();
    for (const root of chrome) {
      for (const element of [root, ...root.querySelectorAll('*')]) {
        if (element.closest('script, style, svg')) continue;
        for (const node of element.childNodes) {
          if (node.nodeType === Node.TEXT_NODE) strings.add(node.textContent);
        }
        for (const attr of ['aria-label', 'title', 'placeholder', 'alt']) {
          if (element.hasAttribute(attr)) strings.add(element.getAttribute(attr));
        }
      }
    }
    return {
      lang: document.documentElement.lang,
      headings: document.querySelectorAll('h1').length,
      viewport: document.documentElement.clientWidth,
      width: Math.max(document.documentElement.scrollWidth, document.body.scrollWidth),
      text: document.body.innerText,
      chrome: [...strings],
      refs: [...document.querySelectorAll('[href], [src], [poster], [srcset], object[data]')]
        .flatMap(element => {
          const refs = ['href', 'src', 'poster', 'data'].flatMap(attr => {
            const value = element.getAttribute(attr);
            if (!value || value.trim().startsWith('#')) return [];
            return [{value, link: attr === 'href' && element.matches('a, area')}];
          });
          // The browser also reports requests for selected responsive image candidates.
          const srcset = element.getAttribute('srcset');
          if (srcset && !srcset.trimStart().startsWith('data:')) {
            refs.push(...srcset.split(',').map(candidate => ({
              value: candidate.trim().split(/\s+/)[0], link: false,
            })).filter(ref => ref.value));
          }
          return refs;
        }),
      base: document.baseURI,
      csp: window.siteIntegrityCspViolations,
    };
  });
  const path = label(url);
  for (const directive of result.csp ?? []) defects.add(`${path} violated CSP directive ${directive}`);
  if (!result.headings) defects.add(`${path} has no h1`);
  if (result.width > result.viewport) {
    defects.add(`${path} overflows horizontally (${result.width}px > ${result.viewport}px)`);
  }
  const marker = result.text.match(/(?:\[\[\s*(?:missing|i18n)[:\s][^\]]+\]\]|\[\s*missing translation[^\]]*\]|translation missing:)/i);
  if (marker) defects.add(`${path} renders a missing-translation marker`);
  const locale = url.pathname.match(/^\/(pl|en)(?:\/|$)/)?.[1] ?? result.lang;
  if (!Object.hasOwn(catalogues, locale) || result.lang !== locale) {
    defects.add(`${path} has unexpected document language ${result.lang || '(empty)'}`);
  }
  for (const string of result.chrome.map(normalize).filter(Boolean)) {
    if (rawKeys.has(string)) defects.add(`${path} exposes raw shell key ${string}`);
    if (!Object.hasOwn(catalogues, locale)) continue;
    const other = locale === 'pl' ? 'en' : 'pl';
    if (catalogueValues[other].has(string) && !catalogueValues[locale].has(string)) {
      defects.add(`${path} uses ${other} shell text ${JSON.stringify(string)} on a ${locale} page`);
    }
  }
  return result;
}

function enqueue(state, url, from, link) {
  // A URL can be both a resource (such as a canonical link) and a navigation target.
  const key = JSON.stringify([url.href, link]);
  if (state.seen.has(key)) return;
  state.seen.add(key);
  state.queue.push({url: url.href, from, link});
}

const probeKey = (url, link) => JSON.stringify([url.href, link]);
const assetTypes = new Set(['font', 'image', 'media', 'script', 'stylesheet']);

async function crawl(page, origin, locale, actor, section, state, defects, pages, workers) {
  const {probes, visited} = state;
  let current = '/';
  await page.addInitScript(() => {
    window.siteIntegrityCspViolations = [];
    document.addEventListener('securitypolicyviolation', event => {
      window.siteIntegrityCspViolations.push(event.effectiveDirective);
    });
  });
  const checkStatus = (url, status, source, document) => {
    if (status === 404 || (status >= 500 && !(status === 503 && document && coldView(url, actor, emptyGame)))) {
      defects.add(`${source} → ${label(url)} returned HTTP ${status}`);
    }
  };
  page.on('response', response => {
    const url = sameOrigin(response.url(), origin, origin);
    if (!url) return;
    const request = response.request();
    checkStatus(url, response.status(), current, request.isNavigationRequest());
    // Use the browser's successful GET for assets it actually fetched. Links
    // and resources the browser didn't fetch still get their own probe.
    if (response.ok() && request.method() === 'GET' && assetTypes.has(request.resourceType())) {
      probes.set(probeKey(url, false), {status: response.status(), html: false});
    }
  });
  page.on('requestfailed', request => {
    const url = sameOrigin(request.url(), origin, origin);
    const error = request.failure()?.errorText;
    if (url && error !== 'net::ERR_ABORTED') {
      defects.add(`${current} → ${label(url)} failed (${error})`);
    }
  });
  page.on('pageerror', error => defects.add(`${current} script error: ${error.message}`));
  page.on('console', message => {
    if (message.type() !== 'error') return;
    const url = sameOrigin(message.location().url, origin, origin);
    if (url && coldView(url, actor, emptyGame) && /^Failed to load resource: the server responded with a status of 503 \(Service Unavailable\)$/.test(message.text())) return;
    defects.add(`${current} console error: ${message.text()}`);
  });

  // The browser can load assets directly, but it must never submit a form or
  // follow a cross-origin document redirect during discovery.
  await page.route('**/*', async route => {
    const request = route.request();
    if (request.method() !== 'GET' ||
        (request.isNavigationRequest() && !sameOrigin(request.url(), origin, origin))) {
      await route.abort();
    } else {
      await route.continue();
    }
  });

  async function probe(url, link) {
    const key = probeKey(url, link);
    if (probes.has(key)) return probes.get(key);
    if (probes.size >= 4000) throw new Error('Site integrity GET probe budget exceeded (4000 URLs)');
    await waitForWikiBudget(url.href, workers);
    const response = await page.request.get(url.href, {
      maxRedirects: 0, timeout: 15_000, headers: {
        'accept-language': locale, ...(link ? {accept: 'text/html'} : {}),
      },
    });
    try {
      const type = response.headers()['content-type'] ?? '';
      const status = response.status();
      const result = {
        status,
        location: response.headers().location,
        html: /^text\/html\b/i.test(type) && /<(?:html[\s>]|!doctype\s+html)/i.test(await response.text()),
      };
      if (link && coldView(url, actor, emptyGame)) {
        if (status !== 503) defects.add(`${label(url)} without a producer returned HTTP ${status}, expected 503`);
        if (!result.html) defects.add(`${label(url)} did not return the unavailable view`);
        if (!response.headers()['cache-control']?.split(',').some(value => value.trim() === 'no-store')) {
          defects.add(`${label(url)} cold projection returned without no-store`);
        }
      }
      probes.set(key, result);
      return result;
    } finally {
      await response.dispose();
    }
  }

  for (let index = 0; index < state.queue.length; index++) {
    const entry = state.queue[index];
    const url = new URL(entry.url);
    if (entry.done || (section !== 'cross-section' && sectionOf(url.pathname) !== section)) continue;
    const {from, link} = entry;
    const expected = link && protectedDestination(url, locale, actor, emptyGame);
    entry.done = true;
    let target = url;
    let phase = 'probing the response';
    try {
      for (const redirects = new Set(); ; ) {
        if (redirects.has(target.href) || redirects.size >= 10) {
          defects.add(`${from} → ${label(url)} exceeded redirect budget`);
          break;
        }
        redirects.add(target.href);
        phase = 'probing the response';
        const result = await probe(target, link);
        checkStatus(target, result.status, from, link);
        if (result.status >= 300 && result.status < 400 && result.location) {
          target = sameOrigin(result.location, target, origin);
          if (!target) {
            if (expected) defects.add(`${label(url)} redirected outside the portal`);
            break;
          }
          continue;
        }
        if (expected && (target.pathname !== expected.path || result.status !== expected.status)) {
          defects.add(`${label(url)} ended at ${label(target)} with HTTP ${result.status}, expected ${expected.path} with HTTP ${expected.status}`);
        }
        if (section !== 'cross-section' && sectionOf(target.pathname) !== section && link && result.html) {
          enqueue(state, target, from === 'seed' ? from : label(url), true);
          break;
        }
        const key = pagePattern(target);
        if (from === 'seed' && result.status !== 200) {
          defects.add(`seed ${label(target)} returned HTTP ${result.status}, expected 200`);
        }
        if (!link || !result.html) break;
        const samples = visited.get(key) ?? [];
        if (samples.includes(target.href) || samples.length >= MAX_SAMPLES) break;
        if (!visited.has(key) && visited.size >= 500) {
          throw new Error('Site integrity page budget exceeded (500 route patterns)');
        }
        current = label(target);
        phase = 'waiting for the wiki quota';
        await waitForWikiBudget(target.href, workers);
        phase = 'loading the document';
        const response = await page.goto(target.href, {waitUntil: 'load'});
        if (response?.status() !== result.status || page.url() !== target.href) {
          defects.add(`${current} browser ended at ${label(new URL(page.url()))} with HTTP ${response?.status()}, expected ${label(target)} with HTTP ${result.status}`);
        }
        phase = 'inspecting the document';
        const view = coldView(target, actor, emptyGame);
        if (view) {
          expect(response?.headers()['cache-control']?.split(',').map(value => value.trim())).toContain('no-store');
          const main = page.locator('main');
          if (view.selector) {
            await expect(main.locator(view.selector)).toHaveText(view.text);
          } else {
            await expect(main).toContainText(view.text);
          }
          await expect(main.locator(view.filters ? 'article, ol > li, form:not([method="get" i])' : 'article, form')).toHaveCount(0);
        }
        samples.push(target.href);
        visited.set(key, samples);
        pages.push(current);
        const rendered = await inspectPage(page, new URL(page.url()), defects);
        for (const ref of rendered.refs) {
          const url = sameOrigin(ref.value, rendered.base, origin);
          if (url) enqueue(state, url, current, ref.link);
        }
        break;
      }
    } catch (error) {
      // Request failures can include headers, cookies, and URL query values.
      if (error.message.startsWith('Site integrity ')) throw error;
      defects.add(`${from} → ${label(url)} failed while ${phase} (${error.name})`);
    }
  }
}

const actors = ['anonymous', 'player', 'platform_admin'];

test.describe.configure({mode: 'parallel'});

for (const actor of actors) {
  test.describe(`site integrity as ${actor}`, () => {
    // Each actor's section shards share a frontier. Playwright can distribute
    // whole actor groups across workers without splitting that dependency.
    test.describe.configure({mode: 'default'});

    function file(info, name) {
      return path.join(process.env.PORTAL_E2E_AUTH_DIRECTORY, `site-integrity-${info.project.name}-${info.repeatEachIndex}-${actor}-${name}.json`);
    }

    test('prepare actor', async ({page}, info) => {
      info.setTimeout(300_000);
      const locale = info.project.metadata.locale;
      if (actor === 'anonymous') {
        await seedPasswordRange(password());
      } else {
        const account = seedAccount(locale);
        if (actor === 'platform_admin') {
          await query(
            "INSERT INTO account_roles(account_id,role_id) SELECT a.id,r.id FROM accounts a CROSS JOIN roles r WHERE a.email=$1 AND r.key='platform_admin'",
            [account.email],
          );
        }
        await login(page, locale, account);
        if (actor === 'platform_admin') {
          await enrollMfa(page, locale, account);
          const recovery = page.locator('textarea[readonly]').first();
          await expect(recovery).toBeVisible();
          await loginWithRecoveryCode(page, locale, account, await recovery.inputValue());
          await expect(page).not.toHaveURL(/mfa\/challenge/);
        }
        await page.goto(`/${locale}/`);
        await expect(page.locator(`form[action="/${locale}/logout"]`)).toBeAttached();
        const passkeys = `/${locale}/account/passkeys`;
        const links = page.locator(`#podglad-konta a[href="${passkeys}"]`);
        if (actor === 'platform_admin') {
          await expect(links).toHaveCount(0);
          expect((await page.request.get(passkeys)).status()).toBe(403);
        } else {
          await expect(links.first()).toBeAttached();
          await page.route('**/static/thais/panel-frame.svg', route => route.continue());
          const [frame, document] = await Promise.all([
            page.waitForResponse(response =>
              new URL(response.url()).pathname === '/static/thais/panel-frame.svg'),
            page.goto(passkeys),
          ]);
          expect(document?.status()).toBe(200);
          expect(frame.status()).toBe(200);
          const csp = (await document.headerValue('content-security-policy')) ?? '';
          expect(csp.split(';').map(directive => directive.trim()).filter(Boolean).sort()).toEqual([
            "base-uri 'none'", "default-src 'self'", "font-src 'self'",
            "frame-ancestors 'none'", "img-src 'self'", "object-src 'none'",
          ]);
        }
      }
      fs.writeFileSync(file(info, 'auth'), JSON.stringify(await page.context().storageState()), {mode: 0o600});
      const origin = new URL(info.project.use.baseURL).origin;
      const state = {queue: [], seen: new Set()};
      enqueue(state, new URL(`/${locale}/`, origin), 'seed', true);
      if (actor === 'platform_admin') enqueue(state, new URL(`/admin?lang=${locale}`, origin), 'seed', true);
      fs.writeFileSync(file(info, 'crawl'), JSON.stringify({
        queue: state.queue, seen: [...state.seen], probes: [], visited: [],
      }), {mode: 0o600});
    });

    test.use({storageState: async ({}, use, info) => {
      const auth = file(info, 'auth');
      await use(info.title === 'prepare actor' ? undefined : auth);
    }});

    for (const section of [...sections, 'cross-section']) {
      test(section, async ({page}, info) => {
        // A shard may render both locales and wait for a real quota window.
        // Individual action, navigation and request deadlines still apply.
        info.setTimeout(300_000);
        const locale = info.project.metadata.locale;
        const origin = new URL(info.project.use.baseURL).origin;
        const saved = JSON.parse(fs.readFileSync(file(info, 'crawl'), 'utf8'));
        const state = {
          queue: saved.queue, seen: new Set(saved.seen), probes: new Map(saved.probes), visited: new Map(saved.visited),
        };
        const pages = [];
        const defects = new Set();
        const before = state.probes.size;
        try {
          await crawl(page, origin, locale, actor, section, state, defects, pages, info.config.workers);
          if (section === 'home') {
            expect(pages.length, 'crawl rendered a seed HTML page').toBeGreaterThan(0);
          }
          if (section === 'cross-section') {
            expect(state.queue.every(entry => entry.done), 'all discovered URLs checked').toBe(true);
          }
        } finally {
          fs.writeFileSync(file(info, 'crawl'), JSON.stringify({
            queue: state.queue, seen: [...state.seen], probes: [...state.probes], visited: [...state.visited],
          }), {mode: 0o600});
          await info.attach('site-integrity', {contentType: 'application/json', body: JSON.stringify({
            pages, urlsProbed: state.probes.size - before, defects: [...defects],
          })});
        }
        expect([...defects], 'site integrity defects').toEqual([]);
      });
    }
  });
}
