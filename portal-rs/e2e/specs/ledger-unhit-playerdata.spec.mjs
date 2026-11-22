import {randomUUID} from 'node:crypto';
import {uuid7} from '../game-sim-helpers.mjs';
import {test, expect} from '../fixtures.mjs';
import {login, register} from '../helpers.mjs';

test('owner submits hunt filters and a comparison, and sharing independently fails closed', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  await login(page, lang, await register(page, lang));
  const path = `/${lang}/account/hunts`;
  expect((await page.goto(path)).status()).toBe(200);

  const filter = page.locator(`main form[action="${path}"]`);
  await filter.locator('[name=party]').selectOption('true');
  const [filtered] = await Promise.all([
    page.waitForResponse(r => r.request().isNavigationRequest() && new URL(r.url()).pathname === path),
    filter.getByRole('button').click(),
  ]);
  expect(filtered.status()).toBe(200);
  expect(new URL(filtered.url()).searchParams.get('party')).toBe('true');
  await expect(page.locator('main .hunt-cards article')).toHaveCount(0);

  const compare = page.locator(`main form[action="${path}/compare"]`);
  await compare.locator('[name=ids]').fill(randomUUID());
  const [comparison] = await Promise.all([
    page.waitForResponse(r => r.request().isNavigationRequest() && new URL(r.url()).pathname === `${path}/compare`),
    compare.getByRole('button').click(),
  ]);
  expect(comparison.status()).toBe(400);

  const share = await page.request.post(`${path}/${randomUUID()}/share`, {
    headers: {origin: new URL(page.url()).origin}, maxRedirects: 0,
  });
  expect(share.status()).toBe(503);
  expect(share.headers()['cache-control']).toBe('private, no-store');
});

test('journal notes refuse writes to an unknown hunt session', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  await login(page, lang, await register(page, lang));
  expect((await page.goto(`/${lang}/account/privacy`)).status()).toBe(200);
  const csrf = await page.locator('main form [name=_csrf]').inputValue();
  const path = `/${lang}/account/journal/reports/notes?kind=session&id=${uuid7()}`;
  const response = await page.request.post(path, {
    form: {_csrf: csrf, body: 'Private note for a missing session.'},
    headers: {origin: new URL(page.url()).origin}, maxRedirects: 0,
  });
  expect(response.status()).toBe(404);
  expect(response.headers()['cache-control']).toBe('private, no-store');
});

test('player updates Atlas participation while the aggregate page is unavailable', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  await login(page, lang, await register(page, lang));
  const places = `/${lang}/atlas/places`;
  const preference = `${places}/participation`;
  expect((await page.goto(places)).status()).toBe(503);
  expect((await page.goto(`/${lang}/account/privacy`)).status()).toBe(200);
  const csrf = await page.locator('main form [name=_csrf]').inputValue();
  const headers = {origin: new URL(page.url()).origin};
  for (const participating of [true, false]) {
    const updated = await page.request.post(preference, {
      form: {_csrf: csrf, participating: String(participating)}, headers, maxRedirects: 0,
    });
    expect(updated.status()).toBe(303);
    expect(updated.headers().location).toBe(places);
    const response = await page.request.get(preference);
    expect(response.status()).toBe(200);
    expect(await response.json()).toEqual({participating});
  }
});

test('Collection PvP refuses tournament creation while collection authority is unavailable', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const account = await register(page, lang);
  await login(page, lang, account);
  expect((await page.goto(`/${lang}/account/privacy`)).status()).toBe(200);
  const csrf = await page.locator('main form [name=_csrf]').inputValue();
  const denied = await page.request.post(`/${lang}/collection-pvp`, {
    form: {_csrf: csrf, op: 'create', capacity: '4'},
    headers: {origin: new URL(page.url()).origin}, maxRedirects: 0,
  });
  expect(denied.status()).toBe(503);
  expect(denied.headers()['cache-control']).toBe('private, no-store');
});

// The active harness cannot render the following forms without more authority:
// F-atlas-161391314177223: owned bestiary progress; the configured source
// returns unavailable.
// F-minigames-114652082160743, F-minigames-187973382217026,
// F-minigames-280573004026262: collection and deck controls require an
// accepted owned character collection.
// F-minigames-24012547260821, F-minigames-177564890723493,
// F-minigames-218934052584363, R-minigames-115252755875764: battle actions
// require a deck and an owned active battle from that collection.
// F-minigames-58824648762686, F-minigames-267968195854043: season pass
// controls need an admitted season and authoritative progress.
// F-payments-249659384691764: native coin checkout requires an existing
// Game-authorized order issued for the authenticated account.
