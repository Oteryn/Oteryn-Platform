import {uuid7} from '../game-sim-helpers.mjs';
import {test, expect} from '../fixtures.mjs';
import {waitForWikiBudget} from '../helpers.mjs';
import {query} from '../staff-rbac-helpers.mjs';

test('character search submits the name to the public profile lookup', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const path = `/${lang}/characters`;
  expect((await page.goto(path)).status()).toBe(200);
  const form = page.locator(`main form[action="${path}"]`);
  await form.locator('[name=name]').fill('Ledger Traveler');
  const [response] = await Promise.all([
    page.waitForResponse(r => new URL(r.url()).pathname === path && r.request().isNavigationRequest()),
    form.getByRole('button').click(),
  ]);
  expect(response.status()).toBe(302);
  expect(response.headers().location).toBe(`${path}/Ledger%20Traveler`);
});

test('localized portal search submits its query', async ({page}, info) => {
  const path = `/${info.project.metadata.locale}/search`;
  await waitForWikiBudget(path, 2);
  expect((await page.goto(path)).status()).toBe(200);
  const form = page.locator(`main form[action="${path}"]`);
  await form.locator('[name=q]').fill('Ledger Traveler');
  const [response] = await Promise.all([
    page.waitForResponse(r => new URL(r.url()).pathname === path && r.request().isNavigationRequest()),
    form.getByRole('button').click(),
  ]);
  expect(response.status()).toBe(200);
  expect(new URL(response.url()).searchParams.get('q')).toBe('Ledger Traveler');
  await expect(form.locator('[name=q]')).toHaveValue('Ledger Traveler');
});

async function applyFilter(page, path, input, value, expectedStatus) {
  const form = page.locator('main form[method=get]');
  if (input) await form.locator(`[name=${input}]`).fill(value);
  const [response] = await Promise.all([
    page.waitForResponse(r => r.request().isNavigationRequest() && new URL(r.url()).pathname === path),
    form.locator('button[type=submit]').click(),
  ]);
  expect(response.status()).toBe(expectedStatus);
  if (input) expect(new URL(response.url()).searchParams.get(input)).toBe(value);
  return response;
}

test('guild statistics submit filters while projections are cold', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  for (const path of ['/stats/guilds', `/${lang}/stats/guilds`]) {
    expect((await page.goto(path)).status()).toBe(503);
    const filtered = await applyFilter(page, path, 'days', '14', 503);
    expect(filtered.headers()['cache-control']).toContain('no-store');
    await expect(page.locator('main [role=status]')).toBeVisible();
  }
});

test('world and ranking statistics submit their filters without disclosing unavailable data', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  expect((await page.goto('/stats/world')).status()).toBe(200);
  const world = await page.locator('main select[name=world]').inputValue();
  expect(world).toBeTruthy();
  const selected = await applyFilter(page, '/stats/world', null, null, 503);
  expect(new URL(selected.url()).searchParams.get('world')).toBe(world);

  const rankings = `/${lang}/stats/rankings/week`;
  expect((await page.goto(rankings)).status()).toBe(503);
  await page.locator('main select[name=pvp]').selectOption('open');
  const filtered = await applyFilter(page, rankings, null, null, 503);
  expect(new URL(filtered.url()).searchParams.get('pvp')).toBe('open');
  await expect(page.locator('main article')).toHaveCount(0);
});

test('character history and comparison submit date filters and refuse cold projections', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const character = uuid7();
  const characterPath = `/${lang}/stats/characters/${character}/experience`;
  const comparePath = `/${lang}/stats/compare`;
  for (const [path, initialStatus] of [[characterPath, 503], [comparePath, 200]]) {
    expect((await page.goto(path)).status()).toBe(initialStatus);
    const form = page.locator('main form[method=get]');
    const date = new Date(Date.now() - 86_400_000).toISOString().slice(0, 10);
    await form.locator('[name=start]').fill(date);
    await form.locator('[name=end]').fill(date);
    if (path === comparePath) {
      await form.locator('[name=ids]').fill(`${character},${uuid7()}`);
    }
    const selected = await applyFilter(page, path, null, null, 503);
    expect(new URL(selected.url()).searchParams.get('start')).toBe(date);
    expect(selected.headers()['cache-control']).toContain('no-store');
    await expect(page.locator('main figure, main article')).toHaveCount(0);
  }
});

test('unprefixed portal search submits the pagination form for editorial results', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const term = `ledger-${uuid7()}`;
  await query(`
    WITH seeded AS (
      INSERT INTO pages(id,slug)
      SELECT uuidv7(), $1 || '-' || n FROM generate_series(1,21) n
      RETURNING id, slug
    ), english AS (
      INSERT INTO editorial_pages(page_id,locale,title,body,published_at)
      SELECT id, 'en', slug, 'Local editorial search result.', now()
      FROM seeded
      RETURNING page_id, title, body, revision
    )
    INSERT INTO editorial_pages(page_id,locale,title,body,published_at,source_revision)
    SELECT page_id, 'pl', title, body, now(), revision
    FROM english
  `, [term]);
  try {
    await waitForWikiBudget('/search', 2);
    expect((await page.goto(`/search?${new URLSearchParams({q: term, kind: 'pages'})}`)).status()).toBe(200);
    const result = page.locator(`main a[href^="/${lang}/pages/${term}-"]`);
    await expect(result).toHaveCount(20);
    const next = page.locator('main form:has([name=page][value="2"])');
    const [response] = await Promise.all([
      page.waitForResponse(r => r.request().isNavigationRequest() && new URL(r.url()).pathname === '/search'),
      next.getByRole('button').click(),
    ]);
    expect(response.status()).toBe(200);
    expect(new URL(response.url()).searchParams.get('page')).toBe('2');
    await expect(result).toHaveCount(1);
    await expect(next).toHaveCount(0);
  } finally {
    await query("DELETE FROM pages WHERE slug LIKE $1 || '-%'", [term]);
  }
});

// Public forms withheld for unavailable or incomplete Game projections:
// F-community-3881337848774, F-community-16287300610837,
// F-community-193593469157960, F-community-219222581560817 (deaths).
// F-community-4730707544746, F-community-274165327503467 (guild directory).
// F-community-71546943467822 (community rankings need a complete watermark).
// F-stats-216992143813701, F-stats-222324429744041: the stats filter is
// available, but forward/back controls require multiple pages of Game ranks.
// F-wiki-5428367139875, F-wiki-172496912900389 (entity filters) and
// F-wiki-22119884537424, F-wiki-58594829099106, F-wiki-154492065254748,
// F-wiki-250551961713734 (search and pagination) need an admitted catalogue.
// The pinned world catalogue candidate is deliberately unsupported.
