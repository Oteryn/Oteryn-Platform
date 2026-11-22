import fs from 'node:fs';
import { test, expect } from '../fixtures.mjs';
import { login, password, register, seedPasswordRange, waitForWikiBudget } from '../helpers.mjs';
import { enrollMfa, loginWithRecoveryCode, query } from '../staff-rbac-helpers.mjs';

const read = path => fs.readFileSync(new URL(path, import.meta.url), 'utf8');
const inventory = JSON.parse(read('../inventory.json'));
const pattern = route => new RegExp('^' + route.split(/(\{[^}]+\})/).map(part => {
  if (part === '{lang}') return '(?:pl|en)';
  if (part.startsWith('{*')) return '.+';
  if (part.startsWith('{')) return '[^/]+';
  return part.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}).join('') + '$');
const routes = inventory.routes.filter(row => row.methods.includes('GET'))
  .map(({route}) => ({route, matches: pattern(route)}))
  .sort((a, b) => b.route.replace(/\{[^}]+\}/g, '').length - a.route.replace(/\{[^}]+\}/g, '').length);
const coldProjections = [...read('../COVERAGE.md').matchAll(
  /^\| `([^`]+)` \| .*PASS: GET 503; (?:authority or projection unavailable|valid experience tab refuses absent accepted character\/privacy projection authority)/gm,
)].map(([, route]) => pattern(route));
const cold = pathname => coldProjections.some(route => route.test(pathname));

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
    };
  });
  const path = label(url);
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

async function crawl(page, origin, seeds, locale, info) {
  const queue = seeds.map(value => ({url: new URL(value, origin), from: 'seed', link: true}));
  const probes = new Map();
  const visited = new Set();
  const defects = new Set();
  let current = '/';
  const checkStatus = (url, status, source) => {
    if (status === 404 || (status >= 500 && !(status === 503 && cold(url.pathname)))) {
      defects.add(`${source} → ${label(url)} returned HTTP ${status}`);
    }
  };
  page.on('response', response => {
    const url = sameOrigin(response.url(), origin, origin);
    if (url) checkStatus(url, response.status(), current);
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
    if (url && cold(url.pathname) && /^Failed to load resource: the server responded with a status of 503 \(Service Unavailable\)$/.test(message.text())) return;
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

  async function probe(url) {
    if (probes.has(url.href)) return probes.get(url.href);
    if (probes.size >= 4000) throw new Error('Site integrity GET probe budget exceeded (4000 URLs)');
    await waitForWikiBudget(url.href, 2);
    const response = await page.request.get(url.href, {
      maxRedirects: 0, timeout: 15_000, headers: {'accept-language': locale},
    });
    try {
      const type = response.headers()['content-type'] ?? '';
      const status = response.status();
      const result = {
        status,
        location: response.headers().location,
        html: /^text\/html\b/i.test(type) && /<(?:html[\s>]|!doctype\s+html)/i.test(await response.text()),
      };
      if (status === 503 && cold(url.pathname) && !response.headers()['cache-control']?.includes('no-store')) {
        defects.add(`${label(url)} cold projection returned 503 without no-store`);
      }
      probes.set(url.href, result);
      return result;
    } finally {
      await response.dispose();
    }
  }

  try {
    for (let index = 0; index < queue.length; index++) {
      const {url, from, link} = queue[index];
      let target = url;
      try {
        for (const redirects = new Set(); ; ) {
          if (redirects.has(target.href) || redirects.size >= 10) {
            defects.add(`${from} → ${label(url)} exceeded redirect budget`);
            break;
          }
          redirects.add(target.href);
          const result = await probe(target);
          checkStatus(target, result.status, from);
          if (result.status >= 300 && result.status < 400 && result.location) {
            target = sameOrigin(result.location, target, origin);
            if (!target) break;
            continue;
          }
          const key = pagePattern(target);
          if (from === 'seed' && result.status !== 200) {
            defects.add(`seed ${label(target)} returned HTTP ${result.status}, expected 200`);
          }
          if (!link || !result.html || visited.has(key)) break;
          if (visited.size >= 500) throw new Error('Site integrity page budget exceeded (500 route patterns)');
          visited.add(key);
          current = label(target);
          const response = await page.goto(target.href, {waitUntil: 'load'});
          expect(response?.status(), current).toBe(result.status);
          const rendered = await inspectPage(page, new URL(page.url()), defects);
          for (const ref of rendered.refs) {
            const url = sameOrigin(ref.value, rendered.base, origin);
            if (url) queue.push({url, from: current, link: ref.link});
          }
          break;
        }
      } catch (error) {
        // Request failures can include headers, cookies, and URL query values.
        if (error.message.startsWith('Site integrity ')) throw error;
        defects.add(`${from} → ${label(url)} could not be checked (${error.name})`);
      }
    }
    expect(visited.size, 'crawl rendered at least one HTML page').toBeGreaterThan(0);
  } finally {
    await info.attach('site-integrity', {contentType: 'application/json', body: JSON.stringify({
      pages: [...visited], urlsProbed: probes.size, defects: [...defects],
    })});
  }
  expect([...defects], 'site integrity defects').toEqual([]);
}

for (const actor of ['anonymous', 'player', 'platform_admin']) {
  test(`site integrity as ${actor}`, async ({page}, info) => {
    test.setTimeout(600_000);
    const locale = info.project.metadata.locale;
    if (actor === 'anonymous') {
      await seedPasswordRange(password());
    } else {
      const account = await register(page, locale);
      if (actor === 'platform_admin') {
        await query(
          "INSERT INTO account_roles(account_id,role_id) SELECT a.id,r.id FROM accounts a CROSS JOIN roles r WHERE a.email=$1 AND r.key='platform_admin'",
          [account.email],
        );
      }
      await login(page, locale, account);
      if (actor === 'platform_admin') {
        await enrollMfa(page, locale, account);
        // The helper can enumerate recovery textareas before the new document
        // has rendered. Wait for the first control before reading its value.
        const recovery = page.locator('textarea[readonly]').first();
        await expect(recovery).toBeVisible();
        await loginWithRecoveryCode(page, locale, account, await recovery.inputValue());
        await expect(page).not.toHaveURL(/mfa\/challenge/);
      }
    }
    const origin = new URL(info.project.use.baseURL).origin;
    await crawl(page, origin, [`/${locale}/`, ...(actor === 'platform_admin' ? [`/admin?lang=${locale}`] : [])], locale, info);
  });
}
