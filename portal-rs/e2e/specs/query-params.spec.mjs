import {spawnSync} from 'node:child_process';
import {randomUUID} from 'node:crypto';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {test, expect} from '../fixtures.mjs';
import {login, password, pool, submit, waitForWikiBudget} from '../helpers.mjs';
import {listing, uuid7} from '../game-sim-helpers.mjs';
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
      item: 'fixture.alpha', creature: 'fixture.rat', q: 'e2e control query', level: '20', floor: '7',
      city: 'thais', rarity: '1', count: '10', tab: 'creatures',
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
const bazaarRefusal = {
  pl: 'Prośba została odrzucona. Sprawdź konto i dane obiektu.',
  en: 'This request was refused. Check your account and the asset details.',
};
const invalidMessages = [
  [/^(?:rankings\/|\/highscores$)/, {pl: 'Nieprawidłowe zapytanie', en: 'Invalid query'}],
  [/^(?:stats\/rankings\/|atlas\/delivery$)/, {pl: 'Nieprawidłowe filtry', en: 'Invalid filters'}],
  [/^stats\/compare$/, {pl: 'Nieprawidłowe zapytanie o statystyki', en: 'Invalid statistics request'}],
  [/^shop\/history$/, {pl: 'Sprawdź przesłane wartości.', en: 'Check the submitted values.'}],
  [/^wiki(?:$|\/(?!places$))/, {pl: 'Nieprawidłowe filtry lub zapytanie', en: 'Invalid filters or search query'}],
];

function expectLocalizedError(document, locale, route, label, messages) {
  messages ??= invalidMessages.find(([pattern]) => pattern.test(route))?.[1] ?? invalidRequest;
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
const freeText = new Set(['q', 'item', 'creature', 'city']);
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
      if (gated && (!gated.control || parameter === entries[0][0])) {
        probes.unshift({name: 'valid', values: [valid]});
      }
      if (route === 'stats/guilds' && parameter === 'days') {
        probes.push({name: 'under minimum', values: ['0']}, {name: 'over maximum', values: ['32']});
      }
      for (const probe of probes) {
        await test.step(`${parameter}: ${probe.name}`, async () => {
          const companion = probe.name === 'valid' ? (gated?.control ?? required) : required;
          const query = new URLSearchParams(companion);
          query.delete(parameter);
          for (const value of probe.values) query.append(parameter, value);
          await waitForWikiBudget(path, 1, catalog?.database);
          // Browser navigation exercises the rendered HTML at each project viewport.
          const browserNavigation = probe.name === 'HTML punctuation';
          const target = `${catalog?.origin ?? ''}${path}?${query}`;
          const bazaarCursor = /^(?:account\/)?bazaar(?:\/items)?$/.test(route) && probe.name !== 'repeated';
          const accept = route === 'shop/history' || bazaarCursor ? '*/*' : 'text/html';
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
          if (status === 400) {
            expectLocalizedError(document, expectedLocale, route, label, bazaarCursor ? bazaarRefusal : undefined);
          }
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
              ? probe.values.filter(value => invalidPage.has(value))
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
      await catalogControls(page, locale, catalog);
    }
    if (group === 'editorial') await editorialControls(page, locale);
    if (group === 'community') await partyControl(page, locale);
    for (const route of routes) await checkRoute(page, locale, route, catalogRoutes.has(route[0]) ? catalog : undefined);
    if (group === 'bazaar') await bazaarControls(page, locale);
  });
}

async function seed(entity, args) {
  const database = process.env.PORTAL_E2E_DATABASE_URL ?? process.env.OTERYN_DATABASE_URL;
  await pool(database).end();
  const root = fileURLToPath(new URL('../../', import.meta.url));
  const binary = process.env.PORTAL_E2E_BINARY ?? path.join(
    process.env.CARGO_TARGET_DIR ?? path.join(root, 'target'), 'debug', 'oteryn-portal',
  );
  const result = spawnSync(binary, ['seed', entity, JSON.stringify(args)], {
    cwd: root,
    env: {OTERYN_ENV: 'testing', OTERYN_MFA_KEY: process.env.OTERYN_MFA_KEY,
      OTERYN_DATABASE_URL: database},
    encoding: 'utf8',
  });
  // Seed output can contain MFA material. Keep subprocess output out of failure messages.
  expect(result.status, `seed CLI ${entity} exit code`).toBe(0);
  return JSON.parse(result.stdout);
}

async function seedPlayer(locale) {
  const account = {email: `query-${randomUUID()}@example.invalid`, password: password()};
  return {...account, id: (await seed('account', {...account, locale})).accountId};
}

async function result(page, target, locale, database) {
  await waitForWikiBudget(target, 1, database);
  const response = await page.goto(target, {waitUntil: 'domcontentloaded'});
  expect(response.status(), new URL(response.url()).pathname).toBe(200);
  await expect(page.locator('html')).toHaveAttribute('lang', locale);
  return page.locator('main');
}

async function seedNews(prefix, count) {
  await query(`INSERT INTO news
    (slug,title,body,published_at,title_pl,body_pl,translation_published_at,translation_source_updated_at)
    SELECT $1 || '-' || n, 'English ' || $1 || ' ' || n, 'English fixture body',
           now()-n*interval '1 second', 'Polski ' || $1 || ' ' || n, 'Polska treść testowa',
           now(), now()
    FROM generate_series(1,$2::int) n`, [prefix, count]);
}

async function editorialControls(page, locale) {
  const marker = `query-${randomUUID()}`;
  const {id} = await seedPlayer(locale);
  try {
    await seedNews(marker, 11);
    await query(`WITH added AS (
      INSERT INTO events(status,starts_at,ends_at,featured,created_by,updated_by,published_by)
      SELECT 'published', now()-interval '3650 days'+n*interval '1 second',
             now()+interval '1 day', true, $1, $1, $1
      FROM generate_series(1,201) n
      RETURNING id, starts_at
    ), ordered AS (
      SELECT id, row_number() OVER(ORDER BY starts_at) AS n FROM added
    )
    INSERT INTO event_translations(event_id,locale,slug,title,summary,body)
    SELECT id, $2, $3 || '-' || n, 'Event ' || $3 || ' ' || n, 'Fixture summary', 'Fixture body'
    FROM ordered`, [id, locale, marker]);

    for (const [route, first, second, last] of [
      ['news', 1, 2, 11], ['events', 0, 1, 201],
    ]) {
      const link = n => `a[href="/${locale}/${route}/${marker}-${n}"]`;
      let main = await result(page, `/${locale}/${route}?page=${first}`, locale);
      await expect(main.locator(link(1))).toHaveCount(1);
      await expect(main.locator(link(last))).toHaveCount(0);
      main = await result(page, `/${locale}/${route}?page=${second}`, locale);
      await expect(main.locator(link(last))).toHaveCount(1);
      await expect(main.locator(link(1))).toHaveCount(0);
    }
    for (const [lang, title] of [['en', 'English'], ['pl', 'Polski']]) {
      const heading = `${title} ${marker} 1`;
      let main = await result(page, `/news?lang=${lang}`, lang);
      await expect(main.getByRole('link', {name: heading, exact: true})).toHaveCount(1);
      main = await result(page, `/today?lang=${lang}`, lang);
      await expect(main.getByRole('heading', {name: heading, exact: true})).toHaveCount(1);
    }
    for (const [include, exclude] of [['news', 'events'], ['events', 'news']]) {
      const filters = new URLSearchParams({q: `${marker} 1`, kind: include, page: '1'});
      const main = await result(page, `/${locale}/search?${filters}`, locale);
      await expect(main.locator(`a[href="/${locale}/${include}/${marker}-1"]`)).toHaveCount(1);
      await expect(main.locator(`a[href="/${locale}/${exclude}/${marker}-1"]`)).toHaveCount(0);
      await expect(main.locator(`a[href="/${locale}/${include}/${marker}-2"]`)).toHaveCount(0);
    }
  } finally {
    await query("DELETE FROM events WHERE created_by=$1", [id]);
    await query("DELETE FROM news WHERE starts_with(slug,$1)", [`${marker}-`]);
  }
}

async function partyControl(page, locale) {
  const {id} = await seedPlayer(locale);
  const marker = `Party ${randomUUID()}`;
  try {
    const fixtures = [
      [world, 'hunt', 50, 150, 'any'],
      [world, 'boss', 50, 150, 'any'],
      [world, 'hunt', 10, 40, 'any'],
      [world, 'hunt', 50, 150, 'paladin'],
      [uuid7(), 'hunt', 50, 150, 'any'],
    ];
    for (const [index, [worldId, purpose, min, max, profession]] of fixtures.entries()) {
      const character = uuid7();
      const name = `${marker} ${index}`;
      await query(`WITH character AS (
        INSERT INTO account_characters(account_id,character_id,world_id,name,availability)
        VALUES($1,$2,$3,$4,'AVAILABLE') RETURNING character_id
      )
      INSERT INTO pd_party_posts(account_id,character_id,character_name,world_id,purpose,min_level,max_level,profession,note,starts_at,expires_at)
      SELECT $1,character_id,$4,$3,$5,$6,$7,$8,$4,now(),now()+interval '1 day' FROM character`,
      [id, character, worldId, name, purpose, min, max, profession]);
    }
    const path = `/${locale}/party`;
    const unfiltered = await result(page, path, locale);
    for (const index of fixtures.keys()) {
      await expect(unfiltered.getByText(`${marker} ${index}`, {exact: true}).last()).toBeVisible();
    }
    const filtered = await result(page, `${path}?world=${world}&purpose=hunt&level=100&profession=knight`, locale);
    await expect(filtered.getByText(`${marker} 0`, {exact: true}).last()).toBeVisible();
    for (const index of [1, 2, 3, 4]) {
      await expect(filtered.getByText(`${marker} ${index}`, {exact: true})).toHaveCount(0);
    }
  } finally {
    await query('DELETE FROM pd_party_posts WHERE account_id=$1', [id]);
    await query('DELETE FROM account_characters WHERE account_id=$1', [id]);
  }
}

async function bazaarControls(page, locale) {
  const seller = await seedPlayer(locale);
  const buyer = await seedPlayer(locale);
  const {recoveryCodes: [code]} = await seed('account-mfa', {accountId: seller.id, password: seller.password});
  await loginWithRecoveryCode(page, locale, seller, code);
  await page.goto(`/${locale}/account/sudo`);
  await submit(page, 'main form', {current_password: seller.password});
  const older = await listing({page, id: seller.id}, buyer, locale);
  const newer = await listing({page, id: seller.id}, buyer, locale);
  const [{before}] = await query(`SELECT (extract(epoch FROM created_at)*1000000000)::numeric(30,0)::text || ':' || id AS before
    FROM bazaar_listings WHERE id=$1`, [newer.id]);
  const card = lot => `[data-listing="${lot.id}"]`;
  for (const route of ['bazaar', 'bazaar/items', 'account/bazaar']) {
    const path = `/${locale}/${route}`;
    let main = await result(page, path, locale);
    await expect(main.locator(card(newer))).toHaveCount(1);
    await expect(main.locator(card(older))).toHaveCount(1);
    main = await result(page, `${path}?before=${encodeURIComponent(before)}`, locale);
    await expect(main.locator(card(older))).toHaveCount(1);
    await expect(main.locator(card(newer))).toHaveCount(0);
  }
  await checkRoute(page, locale, ['account/bazaar', {before}]);
}

test('player Store history applies the rendered cursor', async ({page}, info) => {
  const locale = info.project.metadata.locale;
  const account = await seedPlayer(locale);
  // This disposable account's immutable ledger is removed with the harness database.
  await query('INSERT INTO wallet_accounts(account_id) VALUES($1) ON CONFLICT DO NOTHING', [account.id]);
  const entries = await query(`WITH operations AS (
      INSERT INTO wallet_operations(id,kind,account_id,amount)
      SELECT uuidv7(),'adjustment',$1,1 FROM generate_series(1,27)
      RETURNING id,account_id
    ), posted AS (
      INSERT INTO wallet_entries(id,operation_id,account_id,available_delta,reserved_delta)
      SELECT uuidv7(),id,account_id,1,0 FROM operations
      RETURNING id,operation_id
    )
    SELECT id,operation_id FROM posted ORDER BY id DESC`, [account.id]);
  await login(page, locale, account);
  const path = `/${locale}/shop/history`;
  const first = await result(page, path, locale);
  await expect(first.locator('article')).toHaveCount(25);
  const cursor = entries[24].id;
  const next = first.locator(`a[href="${path}?before=${cursor}"]`);
  await expect(next).toHaveCount(1);
  for (const [index, entry] of entries.entries()) {
    await expect(first.getByText(entry.operation_id, {exact: true})).toHaveCount(index < 25 ? 1 : 0);
  }
  const second = await result(page, `${path}?before=${cursor}`, locale);
  await expect(second.locator('article')).toHaveCount(2);
  for (const [index, entry] of entries.entries()) {
    await expect(second.getByText(entry.operation_id, {exact: true})).toHaveCount(index < 25 ? 0 : 1);
  }
  await checkRoute(page, locale, ['shop/history', {before: cursor}]);
});

async function catalogControls(page, locale, {origin, database}) {
  const path = `${origin}/${locale}`;
  const itemLink = key => `a[href="/${locale}/wiki/items/fixture.${key}"]`;
  const options = [['alpha', 'beta', 'rat', 'wolf'], ['beta', 'alpha', 'wolf', 'rat']];
  const allItems = await result(page, `${path}/wiki/items`, locale, database);
  await expect(allItems.locator(itemLink('alpha'))).toHaveCount(1);
  await expect(allItems.locator(itemLink('beta'))).toHaveCount(1);
  for (const [item, excludedItem, source, excludedSource] of options) {
    for (const [route, kind] of [['wiki/search', 'kind=items&'], ['wiki/items', ''], ['search', 'kind=world&']]) {
      const main = await result(page, `${path}/${route}?${kind}q=fixture.${item}`, locale, database);
      await expect(main.locator(itemLink(item))).toHaveCount(1);
      await expect(main.locator(itemLink(excludedItem))).toHaveCount(0);
    }
    const atlas = await result(page, `${path}/atlas/delivery?item=fixture.${item}&tab=creatures`, locale, database);
    await expect(atlas.locator('input[name=item]')).toHaveValue(`fixture.${item}`);
    await expect(atlas.getByRole('heading', {name: `fixture.${source}`, exact: true})).toHaveCount(1);
    await expect(atlas.getByRole('heading', {name: `fixture.${excludedSource}`, exact: true})).toHaveCount(0);
  }
}
