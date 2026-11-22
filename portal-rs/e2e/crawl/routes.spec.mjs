import crypto from 'node:crypto';
import fs from 'node:fs';
import { spawnSync } from 'node:child_process';
import { test as base, expect } from '@playwright/test';
import { disposableDatabaseUrl, login, password, seedPasswordRange, waitForWikiBudget } from '../helpers.mjs';
import { enrollMfa, loginWithRecoveryCode, query } from '../staff-rbac-helpers.mjs';
import { localeOf, projectionPaths, redirectDestination, routeOf, staffDenied, unavailableView } from './contracts.mjs';

const test = base.extend({
  offlinePage: async ({ browser }, use) => {
    // Parse alternate HTML without applying the currently rendered page's CSP
    // or allowing detached document resources to reach the network.
    const context = await browser.newContext({ offline: true });
    try {
      await use(await context.newPage());
    } finally {
      await context.close();
    }
  },
});

const { sections } = JSON.parse(fs.readFileSync(new URL('../inventory.json', import.meta.url)));
const expectations = JSON.parse(fs.readFileSync(new URL('../route-expectations.json', import.meta.url)));
const staffPages = expectations.filter(row => row.actor === 'staff' && row.method === 'GET' && row.status === 200
  && !row.route.replaceAll('{lang}', '').replace('{slug}', '').includes('{'));
// Diagnostics omit URL query values, request headers and rendered account data.
const label = url => url.pathname.replace(/(\/(?:reset-password|email-change\/(?:confirm|revert))\/)[^/]+$/, '$1[token]')
  + (url.search ? '?' + [...url.searchParams.keys()].sort().join('&') : '');
const staffLocaleQuery = pathname => /^\/admin\/(?:console|news|media)(?:\/|$)/.test(pathname);

function documentLocale(url, browserLocale) {
  const localizedStaff = url.pathname.match(/^\/admin\/(pl|en)\//)?.[1]
    ?? url.pathname.match(/^\/admin\/pages\/[^/]+\/(pl|en)$/)?.[1];
  const fallback = url.pathname === '/' || staffLocaleQuery(url.pathname) ? 'pl' : browserLocale;
  return localizedStaff ?? localeOf(url, url.searchParams.get('lang') ?? fallback);
}

function localURL(href, base, origin) {
  const url = new URL(href, base);
  if (url.origin !== origin || url.username || url.password) return;
  url.hash = '';
  return url;
}

function inspectDocument({ html, view, privateValues }) {
  const doc = html === undefined ? document : new DOMParser().parseFromString(html, 'text/html');
  const main = doc.querySelector('main');
  const contents = {
    lang: doc.documentElement.lang,
    shell: Boolean(doc.querySelector('header#portal-header') && doc.querySelector('footer.foot')),
    title: Boolean(main?.querySelector('h1')?.textContent.trim()),
  };
  if (view && main) {
    const normalize = text => text.replace(/\s+/g, ' ').trim();
    const children = [...main.children];
    const rawText = [...main.childNodes].filter(node => node.nodeType === Node.TEXT_NODE)
      .map(node => normalize(node.textContent)).filter(Boolean);
    const textMatches = view.selector
      ? rawText.length === 0 && [...main.querySelectorAll(view.selector)].some(node => normalize(node.textContent) === view.text)
      : rawText.length === 1 && rawText[0] === view.text;
    const layoutMatches = children.length === view.layout.length
      && children.every((node, index) => node.matches(view.layout[index]));
    const raw = doc.documentElement.outerHTML;
    const text = doc.documentElement.textContent;
    const privateData = privateValues.some(value => raw.includes(value) || text.includes(value));
    const data = main.querySelector('article, table, [role="table"], [role="grid"], dl, li:not(nav li)');
    const mutation = main.querySelector(view.filters ? 'form:not([method="get" i])' : 'form');
    contents.empty = textMatches && layoutMatches && !privateData && !data && !mutation;
  }
  if (html !== undefined) return contents;
  return {
    ...contents,
    overflow: doc.documentElement.scrollWidth > innerWidth,
    csp: window.crawlCsp,
    links: [...doc.querySelectorAll('a[href], area[href]')].map(node => node.href),
    assets: [...doc.querySelectorAll('link[rel~="stylesheet"][href], script[src], img[src]')]
      .map(node => node.href || node.src),
  };
}

function unavailableMatches(response, contents, html, privateValues) {
  const noStore = response.headers()['cache-control']?.split(',').some(value => value.trim() === 'no-store');
  return noStore && contents.empty && !privateValues.some(value => html.includes(value));
}

function seed(entity, args) {
  const result = spawnSync(process.env.PORTAL_E2E_BINARY, ['seed', entity, JSON.stringify(args)], {
    env: { ...process.env, OTERYN_ENV: 'testing', OTERYN_DATABASE_URL: disposableDatabaseUrl() },
    stdio: 'ignore',
  });
  expect(result.status, `seed ${entity} exited successfully`).toBe(0);
}

test.beforeAll(async () => {
  const [fixture] = await query('SELECT session_user = current_database() AS scoped_login');
  expect(fixture.scoped_login, 'runner uses the fixture database role').toBe(true);
  seed('world', { worldId: '019a0000-0000-7000-8000-00000000e001', name: 'Crawl World', region: 'local' });
  await seedPasswordRange(password());
});

async function seedPrivateAccount(locale) {
  const account = { email: `crawl-${crypto.randomUUID()}@example.invalid`, password: password() };
  seed('account', { ...account, locale });
  // Platform-owned private notes exist even without a Game Journal source.
  const note = `crawl-note-${crypto.randomUUID()}`;
  const rows = await query(`
    INSERT INTO pd_report_notes(account_id, target_kind, target_id, body)
    SELECT id, 'session', uuidv7(), $2 FROM accounts WHERE email=$1
    RETURNING account_id, target_id
  `, [account.email, note]);
  expect(rows.length, 'fixture account has a private Journal note').toBe(1);
  return { account, privateValues: [account.email, rows[0].account_id, rows[0].target_id, note] };
}

async function authenticate(page, locale, actor, account) {
  const admin = actor === 'platform_admin';
  if (admin) {
    const rows = await query(`
      INSERT INTO account_roles(account_id, role_id)
      SELECT a.id, r.id FROM accounts a CROSS JOIN roles r WHERE a.email=$1 AND r.key='platform_admin'
      RETURNING account_id
    `, [account.email]);
    expect(rows.length, 'staff fixture uses the built-in platform admin role').toBe(1);
  }
  let step = 'password sign-in';
  try {
    await login(page, locale, account);
    if (admin) {
      step = 'MFA enrollment';
      await enrollMfa(page, locale, account);
      const recovery = page.locator('main textarea[readonly]').first();
      await expect(recovery).toBeVisible();
      const code = await recovery.inputValue();
      if (!code) throw new Error('MFA enrollment did not issue a recovery code');
      step = 'recovery code sign-in';
      await loginWithRecoveryCode(page, locale, account, code);
      await expect(page).not.toHaveURL(/mfa\/challenge/);
    }
    step = 'signed-in home';
    await page.goto(`/${locale}/`);
    await expect(page.locator(`form[action="/${locale}/logout"]`).first()).toBeAttached();
  } catch (error) {
    // Action errors can include the value typed into a credential input.
    throw new Error(`${actor} could not complete ${step} (${error.name})`);
  }
}

for (const actor of ['anonymous', 'player', 'platform_admin']) {
  test(`crawl as ${actor}`, async ({ page, baseURL, offlinePage }, info) => {
    const locale = info.project.metadata.locale;
    const origin = new URL(baseURL).origin;
    const own = await seedPrivateAccount(locale);
    const other = await seedPrivateAccount(locale);
    const privateValues = [...own.privateValues, ...other.privateValues];
    if (actor !== 'anonymous') await authenticate(page, locale, actor, own.account);
    const queue = new Map();
    const visited = new Set();
    const loadedAssets = new Set();
    const defects = new Set();
    const declaredRefusals = new Map();
    const httpNotices = [];
    const covered = [];
    let current = `/${locale}/`;
    function enqueue(input, from, document = true) {
      const url = localURL(input, origin, origin);
      if (!url) return;
      const key = JSON.stringify([url.href, document]);
      if (queue.has(key)) return;
      if (queue.size >= 1500) throw new Error('Crawl exceeded its 1500 URL budget');
      queue.set(key, { url, from, document });
    }

    // Seed the registered destinations so a missing link cannot hide a broken page.
    enqueue('/', 'home');
    enqueue(`/${locale}/`, 'localized home');
    for (const { path } of sections) {
      enqueue(path.startsWith('/') ? path : `/${locale}/${path}`, 'registry');
    }
    for (const path of projectionPaths(actor)) enqueue(`/${locale}/${path}`, 'projection');
    enqueue('/admin', 'staff panel');
    enqueue('/account/game-logins', 'game login history alias');
    enqueue(`/${locale}/account/passkeys`, 'passkey access');
    for (const { route } of staffPages) {
      const target = new URL(route.replaceAll('{lang}', locale).replace('{slug}', 'rules'), origin);
      if (target.pathname.endsWith('/data')) {
        const allowed = actor === 'platform_admin';
        const expected = allowed ? 200 : 403;
        const response = await page.request.get(target.href, {
          maxRedirects: 0, headers: { accept: 'application/json' },
        });
        try {
          const headers = response.headers();
          if (response.status() !== expected || headers.location
            || (allowed && !headers['content-type']?.startsWith('application/json'))) {
            defects.add(`staff routes → ${label(target)} expected HTTP ${expected}${allowed ? ' with JSON' : ''}, got HTTP ${response.status()}`);
          } else {
            covered.push({ path: label(target), status: expected });
          }
        } finally {
          await response.dispose();
        }
        continue;
      }
      // These staff pages select language through their documented query parameter.
      if (staffLocaleQuery(target.pathname)) target.searchParams.set('lang', locale);
      enqueue(target.href, 'staff routes');
    }

    await page.addInitScript(() => {
      window.crawlCsp = [];
      document.addEventListener('securitypolicyviolation', event => {
        if (event.disposition === 'enforce') window.crawlCsp.push(event.effectiveDirective);
      });
    });
    await page.route('**/*', route => {
      const request = route.request();
      if (request.method() === 'GET' && localURL(request.url(), origin, origin)) return route.continue();
      if (request.method() !== 'GET' || request.isNavigationRequest()) {
        defects.add(`${current} attempted a non-GET or cross-origin navigation`);
      }
      return route.abort();
    });
    page.on('request', request => {
      if (request.redirectedFrom() && !localURL(request.url(), origin, origin)) {
        defects.add(`${current} attempted a cross-origin redirect`);
      }
    });
    page.on('pageerror', () => defects.add(`${current} raised a browser script error`));
    page.on('response', response => {
      if (response.request().isNavigationRequest() && response.frame() === page.mainFrame()) return;
      const url = localURL(response.url(), origin, origin);
      if (url && response.status() === 200 && ['font', 'image', 'media', 'script', 'stylesheet'].includes(response.request().resourceType())) {
        loadedAssets.add(url.href);
      }
      if (url && response.status() >= 400) {
        defects.add(`${current} loaded ${label(url)} with HTTP ${response.status()}`);
      }
    });
    page.on('requestfailed', request => {
      const url = localURL(request.url(), origin, origin);
      if (url && request.failure()?.errorText !== 'net::ERR_ABORTED') {
        defects.add(`${current} could not load ${label(url)}`);
      }
    });
    page.on('console', message => {
      if (message.type() !== 'error') return;
      const notice = message.text().match(/^Failed to load resource: the server responded with a status of (403|503) \([^)]+\)$/);
      if (notice) httpNotices.push({ source: current, url: message.location().url, status: Number(notice[1]) });
      else defects.add(`${current} logged a browser console error`);
    });

    for (const { url, from, document } of queue.values()) {
      current = label(url);
      try {
        if (!document) {
          if (loadedAssets.has(url.href)) continue;
          const asset = await page.request.get(url.href, { maxRedirects: 0 });
          const status = asset.status();
          await asset.dispose();
          if (status !== 200) defects.add(`${from} → ${current} asset returned HTTP ${status}`);
          continue;
        }

        const route = routeOf(url);
        const view = unavailableView(url, actor, locale);
        const forbidden = staffDenied(route, actor)
          || (route === 'account/passkeys' && actor === 'platform_admin');
        const redirectPath = redirectDestination(url, actor, locale);
        const expectedStatus = forbidden && !redirectPath ? 403 : view ? 503 : 200;
        const target = redirectPath ? new URL(redirectPath, origin) : url;
        const expectedLang = documentLocale(target, locale);
        const isAlternateLocale = expectedLang !== locale
          && /^(?:\/(?:pl|en)(?:\/|$)|\/admin\/(?:pl|en)\/)/.test(target.pathname);
        const headers = { accept: 'text/html', 'accept-language': isAlternateLocale ? expectedLang : locale };
        const inspection = { view, privateValues };

        if (redirectPath) {
          const redirectStatus = actor === 'anonymous' && route === 'password/change' ? 302 : 303;
          const redirect = await page.request.get(url.href, { maxRedirects: 0, headers });
          try {
            const location = redirect.headers().location;
            if (redirect.status() !== redirectStatus || !location || localURL(location, url, origin)?.href !== target.href) {
              defects.add(`${from} → ${current} expected ${redirectStatus} to ${redirectPath}, got HTTP ${redirect.status()}`);
              continue;
            }
          } finally {
            await redirect.dispose();
            if (route.startsWith('language/')) await page.context().clearCookies({ name: 'portal_language' });
          }
        }
        if (visited.has(target.href)) continue;
        await waitForWikiBudget(target.href, info.config.workers);
        if (isAlternateLocale) {
          // The matching locale project renders these documents at its viewport.
          // Here the actual link must still reach the exact HTML response.
          const response = await page.request.get(target.href, { maxRedirects: 0, headers });
          try {
            const contentType = response.headers()['content-type'] ?? '';
            if (response.status() !== expectedStatus || !contentType.startsWith('text/html')) {
              defects.add(`${from} → ${current} alternate language expected HTML with HTTP ${expectedStatus}, got ${response.status()}`);
              continue;
            }
            const html = await response.text();
            const contents = await offlinePage.evaluate(inspectDocument, { ...inspection, html });
            if (!contents.shell || !contents.title || contents.lang !== expectedLang
              || (view && !unavailableMatches(response, contents, html, privateValues))) {
              defects.add(`${from} → ${current} invalid ${expectedLang} alternate document`);
            }
            covered.push({ path: label(target), status: expectedStatus, locale: expectedLang, ...(view && { reason: view.reason }) });
          } finally {
            await response.dispose();
          }
          visited.add(target.href);
          continue;
        }
        const response = await page.goto(target.href);
        if (response?.request().redirectedFrom() || response?.status() !== expectedStatus || page.url() !== target.href) {
          defects.add(`${from} → ${current} expected HTTP ${expectedStatus}, reached ${label(new URL(page.url()))} with HTTP ${response?.status()}${view ? ` (${view.reason})` : ''}`);
          continue;
        }
        if (!response.headers()['content-type']?.startsWith('text/html')) {
          defects.add(`${from} → ${current} did not return a browser page`);
          continue;
        }
        if (expectedStatus !== 200) declaredRefusals.set(target.href, expectedStatus);
        if (visited.has(target.href)) continue;
        visited.add(target.href);
        covered.push({ path: label(target), status: expectedStatus, locale: documentLocale(target, locale), rendered: true, ...(view && { reason: view.reason }) });

        // Scroll width depends on the actual metrics of the loaded shell fonts.
        await page.evaluate(() => document.fonts.ready.then(() => null));
        const rendered = await page.evaluate(inspectDocument, inspection);
        if (!rendered.shell || !rendered.title) defects.add(`${current} is missing the shared shell or its main heading`);
        if (rendered.lang !== expectedLang) defects.add(`${current} document language should be ${expectedLang}`);
        if (rendered.overflow) defects.add(`${current} overflows the viewport`);
        if (rendered.csp.length) defects.add(`${current} violated CSP (${rendered.csp.join(', ')})`);
        if (view && !unavailableMatches(response, rendered, await response.text(), privateValues)) {
          defects.add(`${current} invalid unavailable view (${view.reason})`);
        }
        for (const href of rendered.links) enqueue(href, label(target));
        for (const href of rendered.assets) enqueue(href, label(target), false);
      } catch (error) {
        const networkError = error.message.match(/net::ERR_[A-Z_]+|ECONNRESET|ECONNREFUSED|ETIMEDOUT|socket hang up|Timeout \d+ms/)?.[0];
        defects.add(`${from} → ${current} failed (${networkError ?? error.name})`);
      }
    }
    for (const { source, url, status } of httpNotices) {
      if (declaredRefusals.get(url) !== status) defects.add(`${source} logged an unexpected HTTP ${status} browser notice`);
    }
    await info.attach('crawl', { body: JSON.stringify(covered), contentType: 'application/json' });
    expect([...defects], 'route crawl defects').toEqual([]);
    expect(covered.length, 'verified route coverage').toBeGreaterThan(sections.length);
  });
}

test('Today exposes only the declared empty-Game JSON and widget', async ({ request }, info) => {
  const locale = info.project.metadata.locale;
  const data = await request.get(`/${locale}/today/data`, { maxRedirects: 0 });
  expect(data.status(), 'no accepted WorldDaily snapshot or watermark').toBe(503);
  expect(data.headers()['cache-control'].split(',').map(value => value.trim())).toContain('no-store');
  expect(await data.json()).toEqual({ status: 'unavailable' });

  const widget = await request.get(`/${locale}/today/widget`, { maxRedirects: 0 });
  expect(widget.status(), 'no accepted WorldDaily snapshot or watermark').toBe(503);
  expect(widget.headers()['cache-control'].split(',').map(value => value.trim())).toContain('no-store');
  expect(widget.headers()['content-type']).toContain('text/html');
  const body = await widget.text();
  expect(body.includes(locale === 'pl'
    ? 'Dzienne dane świata są teraz niedostępne.'
    : 'World daily data is currently unavailable.')).toBe(true);
});

test('a cold atlas read without browser decoration is uncacheable', async ({ request }, info) => {
  const locale = info.project.metadata.locale;
  const response = await request.get(`/${locale}/atlas`, {
    maxRedirects: 0, headers: { accept: '*/*' },
  });
  expect(response.status(), 'no accepted atlas version').toBe(503);
  expect(response.headers()['cache-control']?.split(',').map(value => value.trim()) ?? []).toContain('no-store');
});
