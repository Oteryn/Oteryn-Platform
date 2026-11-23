import { test, expect } from '../fixtures.mjs';
import { login } from '../helpers.mjs';
import { seededStaff } from '../staff-rbac-helpers.mjs';
import { messages, missingId, probe, seedPlayer, visit } from '../public-depth-helpers.mjs';

test('world tools lead from a player session to localized journal and place states', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const atlas = messages('crates/atlas/src/bestiary', lang);
  const places = messages('crates/wiki/src/places', lang);
  const bestiaryPath = `/${lang}/atlas/bestiary`;
  const placesPath = `/${lang}/wiki/places`;

  await visit(page, bestiaryPath, 200, { lang });
  await expect(page.locator('main a[href$="/login"]')).toBeVisible();
  await login(page, lang, await seedPlayer(lang));
  await page.goto(`/${lang}`);
  const navLink = page.locator(`#portal-header a[href="${bestiaryPath}"]`).first();
  const menu = page.locator('#portal-menu-toggle');
  if (await menu.isVisible()) await menu.click();
  await page.locator('#portal-world-button').click();
  await navLink.click();
  await expect(page).toHaveURL(new RegExp(`${bestiaryPath}$`));
  await expect(page.locator('main')).toContainText(atlas.unavailable);
  await expect(page.locator('main a[href$="/login"]')).toHaveCount(0);
  await probe(page, `${bestiaryPath}/${missingId}`, 503, { lang, message: atlas.unavailable, empty: '.bestiary-creature' });
  await probe(page, `${bestiaryPath}/${missingId}?stars=6`, 400);

  await page.locator(`main a[href="${placesPath}"]`).click();
  await expect(page.locator('main')).toContainText(places.unavailable);
  await page.locator('select[name="purpose"]').selectOption('boss');
  await page.locator('select[name="party"]').selectOption('4');
  await page.locator('select[name="sort"]').selectOption('profit');
  const [response] = await Promise.all([
    page.waitForResponse(reply => reply.request().isNavigationRequest() && reply.url().includes('purpose=boss')),
    page.locator('main button[type="submit"]').click(),
  ]);
  expect(response.status()).toBe(503);
  await expect(page.locator('select[name="purpose"]')).toHaveValue('boss');
  await expect(page.locator('select[name="party"]')).toHaveValue('4');
  await expect(page.locator('select[name="sort"]')).toHaveValue('profit');
  await expect(page.locator('main article')).toHaveCount(0);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
  await probe(page, `${placesPath}?party=3`, 400, { lang, message: places.invalid });

});

test('Wiki staff can open the Game evidence status', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const places = messages('crates/wiki/src/places', lang);
  await seededStaff(page, lang, 'platform_admin');
  await page.setExtraHTTPHeaders({ 'accept-language': lang });
  await page.goto('/admin/wiki');
  const [response] = await Promise.all([
    page.waitForResponse(reply => reply.request().isNavigationRequest() && reply.url().endsWith('/admin/wiki/game-evidence')),
    page.locator('main a[href="/admin/wiki/game-evidence"]').click(),
  ]);
  expect(response.status()).toBe(200);
  expect(response.headers()['cache-control']).toContain('no-store');
  await expect(page.locator('main h1')).toHaveText(places['game-evidence']);
  await expect(page.locator('main')).toContainText(places['awaiting-qualification']);
});
