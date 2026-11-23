import {test, expect} from '../fixtures.mjs';
import {query, seededAccount, seededLogin, seededStaff} from '../staff-rbac-helpers.mjs';
import {uuid7} from '../game-sim-helpers.mjs';

test('players and staff can read Bazaar qualification and listing status', async ({page, browser}, info) => {
  const lang = info.project.metadata.locale;
  const publicPath = `/${lang}/bazaar/readiness`;
  const staffPath = `/admin/${lang}/bazaar/readiness`;
  const title = lang === 'pl' ? 'Gotowość bazaru' : 'Bazaar readiness';
  const unavailable = lang === 'pl' ? 'Kwalifikacja produkcyjna w toku' : 'Production qualification pending';

  const player = await seededAccount(lang);
  await seededLogin(page, lang, player);
  await page.goto(`/${lang}/account/bazaar`);
  await page.locator(`main a[href="${publicPath}"], header a[href="${publicPath}"]`).click();
  await expect(page).toHaveURL(new RegExp(`${publicPath}$`));
  await expect(page.getByRole('heading', {name: title, exact: true})).toBeVisible();
  await expect(page.getByRole('heading', {name: unavailable, exact: true})).toBeVisible();
  expect(await page.locator('main form').count()).toBe(0);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);

  const context = await browser.newContext({
    baseURL: info.project.use.baseURL,
    ignoreHTTPSErrors: true,
    locale: lang,
    viewport: info.project.use.viewport,
  });
  try {
    const staff = await context.newPage();
    await seededStaff(staff, lang, 'moderator');
    const listing = uuid7();
    // Persist only a Platform draft. There is no Game asset or receipt in this case.
    await query(`INSERT INTO bazaar_listings (id, owner, kind, form, currency, price, accept_offers, ends_at)
      VALUES ($1, $2, 'item', 'auction', 'coins', 1, false, clock_timestamp() + interval '1 day')`,
    [listing, player.id]);
    const lookup = `${staffPath}?listing=${listing}`;
    expect((await page.request.get(lookup, {maxRedirects: 0})).status()).toBe(403);

    await staff.goto('/admin', {headers: {'Accept-Language': lang}});
    await staff.locator(`main nav a[href="${staffPath}"]`).click();
    await expect(staff.getByRole('heading', {name: title, exact: true})).toBeVisible();
    await staff.locator('input[name=listing]').fill(listing);
    await staff.locator('main form button').click();
    await expect(staff).toHaveURL(new RegExp(`listing=${listing}$`));
    await expect(staff.locator('[data-listing-state]')).toHaveText(lang === 'pl' ? 'Szkic' : 'Draft');
    await expect(staff.locator('[data-settlement-state]')).toHaveText(lang === 'pl' ? 'Brak zapisu' : 'No record');
    expect(await staff.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
    expect((await staff.request.get(`${staffPath}?listing=invalid`)).status()).toBe(422);
    expect((await staff.request.get(`${staffPath}?listing=${uuid7()}`)).status()).toBe(404);
  } finally {
    await context.close();
  }
});
