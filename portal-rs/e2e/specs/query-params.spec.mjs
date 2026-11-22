import {spawnSync} from 'node:child_process';
import {randomUUID} from 'node:crypto';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {test, expect} from '../fixtures.mjs';
import {password, pool, waitForWikiBudget} from '../helpers.mjs';
import {loginWithRecoveryCode, query} from '../staff-rbac-helpers.mjs';

const unknown = 'e2e-unknown-query-value';
const unicode = 'Zażółć gęślą 世界';
const overlong = 'x'.repeat(2300);
// An inert element and attribute reveal HTML context breaks without executing code.
const markup = `" data-query-probe="marker"><query-probe>'&</query-probe>`;
const sqlPunctuation = "O'Neil; %, _ --";
const huge = '999999999999999999999999999999999999';
const invalidPage = new Set(['-1', '0', '2147483647', huge, unknown, overlong, unicode,
  markup, sqlPunctuation]);
const world = '019b0760-0000-7000-8000-000000000001';
const statsDates = {world, start: '2026-01-01', end: '2026-01-01'};
const comparison = {
  ids: `${world},019b0760-0000-7000-8000-000000000002`,
  metric: 'level', start: statsDates.start, end: statsDates.end,
};
const places = {sort: 'experience', purpose: 'hunt', party: '1'};
const analyticsEpochRange = {start: '1767225600', end: '1767312000'};
const analyticsCalendarRange = {from: '2026-01-01T00:00', to: '2026-01-02T00:00'};
const catalogRoutes = new Set(['search', 'wiki/search', 'wiki/items', 'atlas/delivery']);

const rankings = ['level', 'experience', 'magic', 'fist', 'club', 'sword', 'axe',
  'distance', 'shielding', 'fishing'];
const growth = ['yesterday', 'week', 'month', 'record', 'oldest', 'anniversaries',
  'name-changes', 'transfers', 'sold'];
const wikiKinds = ['creatures', 'npcs', 'spells', 'quests', 'achievements', 'loot', 'areas'];

const wikiFilters = {
  q: 'e2e control query', category: '', classification: '', kind: '', page: '1',
};

const publicRoutes = {
  editorial: [
    ['news', {page: '1', lang: 'pl'}],
    ['events', {page: '1'}],
    ['today', {lang: 'pl'}],
  ],
  search: [
    ['search', {q: 'e2e control query', kind: 'news', page: '1'}],
    ['wiki/search', wikiFilters],
    ['wiki/items', wikiFilters],
    ['wiki/places', places, places],
    ['atlas/delivery', {
      item: 'fixture.alpha', creature: 'rat', q: 'e2e control query', level: '20', floor: '7',
      city: 'thais', rarity: '1', count: '10', tab: 'items',
    }],
  ],
  community: [
    ...rankings.map(kind => [`rankings/${kind}`, {page: '1', vocation: '0', world}]),
    ['/highscores', {page: '1', vocation: '0', world}, {}, '/pl/rankings/level'],
    ['online', {world, sort: 'level'}],
    ['deaths', {world: 'Example'}],
    ['houses', {world}],
    ['guilds', {q: 'e2e control query', page: '1'}],
    ['party', {world, purpose: 'hunt', level: '100', profession: 'any'}],
  ],
  bazaar: [
    ['bazaar', {before: `1:${world}`}],
    ['bazaar/items', {before: `1:${world}`}],
  ],
  statistics: [
    ...growth.map(kind => [`stats/rankings/${kind}`, {world, vocation: '0', pvp: 'open', page: '1'}]),
    ['stats/world', statsDates, statsDates],
    ['stats/guilds', {guild: 'Rose', compare: 'Second', days: '7'}, {guild: 'Rose'}],
    ['stats/compare', comparison, comparison],
  ],
};

const staffRoutes = [
  ['account/bazaar', {before: `1:${world}`}],
  ['shop/history', {before: world}],
  ['support/tickets', {page: '0'}],
  ['support/reports', {page: '1'}],
  ['admin/support/tickets', {page: '0'}],
  ['/admin/news', {page: '1', lang: 'pl'}],
  ['/admin/media', {page: '1', lang: 'pl'}],
  ['/admin/events', {page: '1'}],
  ['/admin/announcements', {page: '1'}],
  ['/admin/roles', {after: world}],
  ['/admin/players', {q: 'e2e control query'}],
  ['/admin/commerce', {after: world, lang: 'pl'}],
  ['/admin/console/players', {q: 'e2e control query', character: 'Aldric Vale', lang: 'pl'}],
  ...['', '/creatures', '/activity', '/levels', '/health', '/economy'].flatMap(suffix => {
    const route = `/admin/analytics${suffix}`;
    const family = suffix === '/economy' ? 'economy_circulation_daily' : 'characters_created_daily';
    return [
      [route, {world, family}],
      [route, analyticsEpochRange, analyticsEpochRange],
      [route, analyticsCalendarRange, analyticsCalendarRange],
    ];
  }),
];

const auditRoute = ['/admin/console/audit', {
  admin: world, request_id: world, operation: 'rbac.role.created', target: world, lang: 'pl',
}];

const ranksUnavailable = {
  // No admitted highscores world exists. Exercise the default selection.
  control: {vocation: '0'},
  notice: {
    pl: 'Tymczasowo niedostępne. Spróbuj ponownie później.',
    en: 'Temporarily unavailable. Please try again later.',
  },
};
const growthUnavailable = {
  control: {vocation: '0', pvp: 'open', page: '1'},
  notice: {
    pl: 'Statystyki są niedostępne. Spróbuj ponownie później.',
    en: 'Statistics are unavailable. Please try again later.',
  },
};
const gatedListings = new Map([
  ...rankings.map(kind => [`rankings/${kind}`, ranksUnavailable]),
  ['/highscores', ranksUnavailable],
  ...growth.map(kind => [`stats/rankings/${kind}`, growthUnavailable]),
  ['online', {
    control: {sort: 'level'},
    notice: {pl: 'Tymczasowo niedostępne.', en: 'Temporarily unavailable.'},
  }],
  ['houses', {
    control: {},
    notice: {
      pl: 'Informacje o domach są niedostępne. Spróbuj ponownie później.',
      en: 'House information is unavailable. Please try again later.',
    },
  }],
  ['deaths', {
    textParameters: ['world'],
    notice: {pl: 'Dane o zgonach są obecnie niedostępne.', en: 'Death records are currently unavailable.'},
  }],
  ['guilds', {
    textParameters: ['q'],
    notice: {pl: 'Informacje o gildiach są niedostępne.', en: 'Guild information is unavailable.'},
  }],
  ['stats/guilds', {
    textParameters: ['guild', 'compare'],
    notice: {pl: 'Niedostępne', en: 'Unavailable'},
  }],
  ['stats/world', {
    notice: {pl: 'Niedostępne', en: 'Unavailable'},
    noticeCount: 2,
  }],
  ['stats/compare', {
    notice: {pl: 'Niedostępne', en: 'Unavailable'},
  }],
  ['wiki/places', {
    notice: {pl: 'Brak zatwierdzonych danych.', en: 'No qualified data available.'},
    noticeSelector: 'main > p:last-child',
  }],
]);

const invalidRequest = {pl: 'Nieprawidłowe żądanie', en: 'Invalid request'};
const invalidMessages = [
  [/^(?:rankings\/|\/highscores$)/, {pl: 'Nieprawidłowe zapytanie', en: 'Invalid query'}],
  [/^(?:stats\/rankings\/|atlas\/delivery$)/, {pl: 'Nieprawidłowe filtry', en: 'Invalid filters'}],
  [/^stats\/compare$/, {pl: 'Nieprawidłowe zapytanie o statystyki', en: 'Invalid statistics request'}],
  [/^shop\/history$/, {pl: 'Sprawdź przesłane wartości.', en: 'Check the submitted values.'}],
  [/^wiki(?:$|\/(?!places$))/, {pl: 'Nieprawidłowe filtry lub zapytanie', en: 'Invalid filters or search query'}],
];

function expectLocalizedError(document, locale, route, label) {
  const messages = invalidMessages.find(([pattern]) => pattern.test(route))?.[1] ?? invalidRequest;
  expect.soft(document.lang, label).toBe(locale);
  expect.soft(document.content, label).toContain(messages[locale]);
}

async function inspectDocument(page, html, url, parameter, noticeSelector = 'main [role=status]') {
  return page.evaluate(({html, url, parameter, noticeSelector}) => {
    const doc = html === null ? document : new DOMParser().parseFromString(html, 'text/html');
    const path = new URL(url).pathname;
    const controls = [...doc.querySelectorAll('main input, main select, main a[href]')];
    return {
      lang: doc.documentElement.lang,
      mainCount: doc.querySelectorAll('main').length,
      notices: [...doc.querySelectorAll(noticeSelector)].map(element => element.textContent.trim()),
      content: doc.querySelector('main')?.textContent ?? '',
      markerCount: doc.querySelectorAll('query-probe, [data-query-probe="marker"]').length,
      effectiveValues: controls.flatMap(element => {
        if (element instanceof HTMLAnchorElement) {
          const target = new URL(element.getAttribute('href'), url);
          return target.pathname === path ? target.searchParams.getAll(parameter) : [];
        }
        return element.name === parameter ? [element.value] : [];
      }),
    };
  }, {html, url, parameter, noticeSelector});
}

async function readResponse(page, target, locale, browserNavigation, accept) {
  if (browserNavigation) {
    const response = await page.goto(target, {waitUntil: 'domcontentloaded'});
    return {
      status: response.status(), url: response.url(), body: await response.text(),
      elapsed: response.request().timing().responseStart,
    };
  }

  const started = performance.now();
  const response = await page.request.get(target, {
    headers: {accept, 'accept-language': locale},
    timeout: 10_000,
  });
  try {
    return {
      status: response.status(), url: response.url(), body: await response.text(),
      elapsed: performance.now() - started,
    };
  } finally {
    await response.dispose();
  }
}
const freeText = new Set(['q', 'operation', 'target', 'character', 'item', 'creature', 'city']);
const zeroBased = new Set(['/admin/media', 'support/tickets', 'admin/support/tickets']);
const boundedText = ['unknown', 'unicode', 'SQL punctuation', 'HTML punctuation'];
const singleValue = [...boundedText, 'overlong'];
const clampedPage = ['zero', 'large integer'];

// Every other malformed probe must be rejected. These routes accept bounded
// text, clamp paging, or fall back to a default when a selector is unknown.
const acceptedProbes = new Map([
  ['news', {lang: singleValue}],
  ['events', {page: clampedPage}],
  ['today', {lang: [...singleValue, 'repeated']}],
  ['search', {q: boundedText}],
  ['wiki/search', {q: boundedText, category: boundedText, classification: boundedText}],
  ['wiki/items', {q: boundedText, category: boundedText, classification: boundedText}],
  ['atlas/delivery', {q: boundedText, creature: boundedText, city: boundedText}],
  ['shop/history', {before: ['valid']}],
  ['support/tickets', {page: clampedPage}],
  ['support/reports', {page: ['large integer']}],
  ['admin/support/tickets', {page: clampedPage}],
  ['/admin/news', {lang: singleValue}],
  ['/admin/media', {lang: singleValue, page: clampedPage}],
  ['/admin/events', {page: clampedPage}],
  ['/admin/announcements', {page: clampedPage}],
  ['/admin/commerce', {lang: singleValue}],
  ['/admin/console/players', {q: ['unknown', 'unicode'], character: ['unknown']}],
  ['/admin/console/audit', {operation: boundedText, target: boundedText}],
]);

function expectedStatus(route, parameter, probe, gated) {
  if (gated) {
    // Punctuation and Unicode are well-formed in these bounded text fields.
    const wellFormed = probe === 'valid'
      || gated.textParameters?.includes(parameter) && boundedText.includes(probe);
    return wellFormed ? 503 : 400;
  }
  return acceptedProbes.get(route)?.[parameter]?.includes(probe) ? 200 : 400;
}

function cases(parameter, valid, locale) {
  const values = [
    ['unknown', unknown], ['overlong', overlong], ['unicode', unicode],
    ['SQL punctuation', sqlPunctuation], ['HTML punctuation', markup],
  ];
  if (parameter === 'page') {
    values.push(['negative', '-1'], ['zero', '0'], ['large integer', '2147483647'],
      ['numeric overflow', huge]);
  }
  return [
    ...values.map(([name, value]) => ({name, values: [value]})),
    {name: 'repeated', values: [parameter === 'lang' ? locale : valid, unknown]},
  ];
}

async function checkRoute(page, locale, [route, parameters, required = {}, destination], catalog) {
  const path = route.startsWith('/') ? route : `/${locale}/${route}`;
  const expectedPath = destination ?? path;
  const expectedLocale = destination ? 'pl' : locale;
  const gated = gatedListings.get(route);
  const entries = Object.entries(parameters);
  let slowest = 0;
  await test.step(path, async () => {
    for (const [parameter, valid] of entries) {
      const probes = cases(parameter, valid, locale);
      if (route === 'shop/history' || gated && (!gated.control || parameter === entries[0][0])) {
        probes.unshift({name: 'valid', values: [valid]});
      }
      if (route === 'stats/guilds' && parameter === 'days') {
        probes.push({name: 'under minimum', values: ['0']}, {name: 'over maximum', values: ['32']});
      }
      for (const probe of probes) {
        await test.step(`${parameter}: ${probe.name}`, async () => {
          const companion = probe.name === 'valid' ? (gated?.control ?? required) : required;
          const query = new URLSearchParams(companion);
          if (route.startsWith('/admin') && 'lang' in parameters) query.set('lang', locale);
          query.delete(parameter);
          for (const value of probe.values) query.append(parameter, value);
          await waitForWikiBudget(path, 1, catalog?.database);
          // Browser navigation exercises the rendered HTML at each project viewport.
          // The console player search also requires real browser Fetch Metadata.
          const browserNavigation = probe.name === 'HTML punctuation' || route === '/admin/console/players';
          const target = `${catalog?.origin ?? ''}${path}?${query}`;
          const accept = route === 'shop/history' ? '*/*' : 'text/html';
          const response = await readResponse(page, target, locale, browserNavigation, accept);
          const label = `${path} ${parameter} (${probe.name})`;
          const status = expectedStatus(route, parameter, probe.name, gated);
          expect.soft(new URL(response.url).pathname, label).toBe(expectedPath);
          expect.soft(response.status, `${label} HTTP status`).toBe(status);

          slowest = Math.max(slowest, response.elapsed);
          expect.soft(response.elapsed, `${label} response timing recorded`).toBeGreaterThanOrEqual(0);
          expect.soft(response.elapsed, `${label} response time (ms)`).toBeLessThan(3_000);
          expect.soft(/<\/?query-probe[\s>]/i.test(response.body), `${label} raw HTML marker`).toBe(false);

          const document = await inspectDocument(
            page, browserNavigation ? null : response.body, response.url, parameter, gated?.noticeSelector,
          );
          expect.soft(document.markerCount, `${label} inert HTML marker in DOM`).toBe(0);
          if (status === 400) expectLocalizedError(document, expectedLocale, route, label);
          if (status === 503) {
            expect.soft(document.lang, label).toBe(expectedLocale);
            expect.soft(document.mainCount, `${label} unavailable page`).toBe(1);
            expect.soft(document.notices, `${label} unavailable message`).toEqual(
              Array(gated.noticeCount ?? 1).fill(gated.notice[expectedLocale]),
            );
          }
          if (response.status === 200) {
            expect.soft(document.mainCount, `${label} listing content`).toBe(1);
          }
          const wikiFacet = /^wiki\/(search|items)$/.test(route)
            && ['category', 'classification'].includes(parameter);
          if (response.status === 200 && wikiFacet) {
            // Catalog facets are exact text filters, so unmatched terms must
            // round-trip safely and render the localized empty-result state.
            expect.soft(document.effectiveValues, `${label} exact text filter`).toEqual(probe.values);
            expect.soft(document.content, `${label} empty result`).toContain(
              expectedLocale === 'pl' ? 'Brak opublikowanych wyników.' : 'No published results.',
            );
          }
          if (response.status === 200 && !freeText.has(parameter) && !wikiFacet) {
            const rejectedValues = parameter === 'page'
              ? probe.values.filter(value => invalidPage.has(value) && !(value === '0' && zeroBased.has(route)))
              : probe.values.filter(value => value !== valid && value !== locale);
            for (const value of rejectedValues) {
              expect.soft(document.effectiveValues, `${label} effective filter`).not.toContain(value);
            }
          }
        });
      }
    }
  });
  test.info().annotations.push({type: 'query-timing', description: `${path} max ${Math.round(slowest)} ms`});
}

test('wiki landing and entity aliases reject impossible pagination', async ({page}, info) => {
  const locale = info.project.metadata.locale;
  for (const suffix of ['wiki', ...wikiKinds.map(kind => `wiki/${kind}`)]) {
    const path = `/${locale}/${suffix}`;
    await waitForWikiBudget(path);
    const response = await page.goto(`${path}?page=${huge}`);
    expect.soft(response.status(), path).toBe(400);
    expectLocalizedError(await inspectDocument(page, null, response.url(), "page"), locale, suffix, path);
  }
});

test.beforeAll(async ({request}) => {
  // Prime the serving process and its database pool before measuring probes.
  await expect.poll(async () => {
    const response = await request.get('/readyz');
    try {
      return response.status();
    } finally {
      await response.dispose();
    }
  }).toBe(200);
  const response = await request.get('/en/news');
  try {
    expect(response.status(), 'editorial path warmed').toBe(200);
  } finally {
    await response.dispose();
  }
});

for (const [group, routes] of Object.entries(publicRoutes)) {
  // The search group can cross several real shared WikiSearch quota windows.
  test(`public ${group} query parameters`, async ({page}, info) => {
    test.setTimeout(group === 'search' ? 900_000 : 360_000);
    const locale = info.project.metadata.locale;
    let catalog;
    if (group === 'search') {
      catalog = {
        origin: process.env.PORTAL_E2E_QUERY_ORIGIN,
        database: process.env.PORTAL_E2E_QUERY_DATABASE_URL,
      };
      if (!catalog.origin || !catalog.database) {
        throw new Error('Start local.mjs and run the matrix with its env.json to select the query catalog');
      }
      await expect.poll(async () => {
        const response = await page.request.get(`${catalog.origin}/readyz`);
        try {
          return response.status();
        } finally {
          await response.dispose();
        }
      }).toBe(200);
      for (const suffix of ['wiki/items', 'wiki/search?q=fixture.alpha', 'atlas/delivery?item=fixture.alpha']) {
        const path = `/${locale}/${suffix}`;
        await waitForWikiBudget(path, 1, catalog.database);
        const response = await page.goto(`${catalog.origin}${path}`);
        expect(response.status(), path).toBe(200);
        if (suffix.startsWith('atlas/')) {
          await expect(page.locator('main input[name=item]')).toHaveValue('fixture.alpha');
        } else {
          await expect(page.locator('main')).toContainText('fixture.alpha');
        }
      }
    }
    for (const route of routes) await checkRoute(page, locale, route, catalogRoutes.has(route[0]) ? catalog : undefined);
  });
}

function seed(entity, args) {
  const root = fileURLToPath(new URL('../../', import.meta.url));
  const binary = process.env.PORTAL_E2E_BINARY ?? path.join(
    process.env.CARGO_TARGET_DIR ?? path.join(root, 'target'), 'debug', 'oteryn-portal',
  );
  const result = spawnSync(binary, ['seed', entity, JSON.stringify(args)], {
    cwd: root,
    env: {...process.env, OTERYN_ENV: 'testing',
      OTERYN_DATABASE_URL: process.env.PORTAL_E2E_DATABASE_URL ?? process.env.OTERYN_DATABASE_URL},
    encoding: 'utf8',
  });
  // Seed output can contain MFA material. Keep subprocess output out of failure messages.
  expect(result.status, `seed CLI ${entity} exit code`).toBe(0);
  return JSON.parse(result.stdout);
}

async function authenticateStaff(page, locale) {
  // Run the existing disposable database guard before invoking the seed CLI.
  const db = pool();
  await db.end();
  const account = {email: `query-${randomUUID()}@example.invalid`, password: password()};
  const {accountId} = seed('account', {...account, locale});
  const {recoveryCodes} = seed('account-mfa', {accountId, password: account.password});
  await query(`INSERT INTO account_roles (account_id, role_id)
    SELECT $1, id FROM roles WHERE key IN ('platform_admin', 'game_support')`, [accountId]);
  expect(recoveryCodes.length, 'seeded MFA recovery codes').toBeGreaterThan(0);
  await loginWithRecoveryCode(page, locale, account, recoveryCodes[0]);
  const response = await page.goto('/admin');
  expect(new URL(response.url()).pathname).toBe('/admin');
  expect(response.status(), 'authenticated platform admin').toBe(200);
}

test('staff audit query parameters', async ({page}, info) => {
  const locale = info.project.metadata.locale;
  await authenticateStaff(page, locale);
  await checkRoute(page, locale, auditRoute);
});

test('staff listing query parameters', async ({page}, info) => {
  test.setTimeout(600_000);
  const locale = info.project.metadata.locale;
  await authenticateStaff(page, locale);
  const search = await page.goto(`/admin/console/players?lang=${locale}&q=Aldric`);
  expect(search.status(), 'staff fixture can read console player search').toBe(200);
  for (const route of staffRoutes) await checkRoute(page, locale, route);
});
