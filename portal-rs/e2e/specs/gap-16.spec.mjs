import { randomUUID } from 'node:crypto';
import { expect, test } from '../fixtures.mjs';
import { messages, seedWorld } from '../public-depth-helpers.mjs';
import { query, seededLogin, seededMfaAccount } from '../staff-rbac-helpers.mjs';

test('players filter deaths and reach daily kill availability for the selected world', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const deaths = messages('crates/community/src/deaths', lang);
  const stats = messages('templates/stats/world', lang);
  const world = await seedWorld();
  const recent = `/${lang}/deaths`;
  const worldStats = `/${lang}/stats/world`;
  const today = new Date().toISOString().slice(0, 10);
  const navigate = () => page.waitForResponse(response => response.request().isNavigationRequest());

  await page.goto(`/${lang}/worlds`);
  if (info.project.use.viewport.width <= 960) {
    await page.locator('.om-fallback-menu > summary').click();
  }
  const menuLink = page.locator(`#portal-navigation a[href="${recent}"]`);
  await menuLink.locator('xpath=ancestor::details[contains(@class, "om-section")]/summary').click();
  const opened = navigate();
  await menuLink.click();
  expect((await opened).status()).toBe(503);

  const form = page.locator('main form');
  await form.getByLabel(deaths.world, { exact: true }).fill(world.name);
  await form.getByLabel(deaths.start, { exact: true }).fill(today);
  await form.getByLabel(deaths.end, { exact: true }).fill(today);
  const filtered = navigate();
  await form.getByRole('button').click();
  const deathResponse = await filtered;
  expect(deathResponse.status()).toBe(503);
  expect(deathResponse.headers()['cache-control']).toContain('no-store');
  await expect(form.locator('[name=world]')).toHaveValue(world.name);
  await expect(page.locator('main')).toContainText(deaths.unavailable);
  await expect(page.locator('main article')).toHaveCount(0);
  expect((await page.request.get(`${recent}?world=${randomUUID()}`)).status()).toBe(400);
  expect((await page.request.get(`${recent}?start=${today}`)).status()).toBe(400);

  const characterDeaths = `/${lang}/characters/${encodeURIComponent('Unqualified Character')}/deaths`;
  expect((await page.goto(characterDeaths)).status()).toBe(503);
  await expect(page.locator('main')).toContainText(deaths.unavailable);
  await page.locator(`main a[href="${recent}"]`).click();
  await page.locator(`main a[href="${worldStats}"]`).click();
  await form.locator('[name=world]').selectOption(world.id);
  await form.locator('[name=start]').fill(today);
  await form.locator('[name=end]').fill(today);
  const submitted = navigate();
  await form.getByRole('button').click();
  const statsResponse = await submitted;
  expect(statsResponse.status()).toBe(503);
  expect(statsResponse.headers()['cache-control']).toContain('no-store');
  await expect(form.locator('[name=world]')).toHaveValue(world.id);
  const kills = page.getByRole('region', { name: stats['world-kills'] });
  await expect(kills.getByRole('status')).toHaveText(stats['world-unavailable']);
  await expect(kills.locator('dd')).toHaveCount(0);
  expect((await page.request.get(`${worldStats}?world=${world.id}&start=2099-01-01&end=2099-01-01`)).status()).toBe(400);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
});

test('game catalogue staff can read the two feed qualification decisions', async ({ page }, info) => {
  test.setTimeout(120_000);
  const lang = info.project.metadata.locale;
  const text = messages('templates/admin', lang);
  const path = '/admin/public-projections';
  const account = await seededMfaAccount(lang);
  const [{ id: role }] = await query('INSERT INTO roles(key) VALUES($1) RETURNING id', [`projection_${randomUUID().replaceAll('-', '')}`]);
  await query('INSERT INTO account_roles(account_id,role_id) VALUES($1,$2)', [account.id, role]);
  await query('INSERT INTO role_permissions(role_id,permission) VALUES($1,$2)', [role, 'admin.access']);
  await seededLogin(page, lang, account);
  await page.goto('/admin');
  await expect(page.locator(`nav a[href="${path}"]`)).toHaveCount(0);
  expect((await page.request.get(path)).status()).toBe(403);

  await query('INSERT INTO role_permissions(role_id,permission) VALUES($1,$2)', [role, 'game_catalog.access']);
  await page.reload();
  const opened = page.waitForResponse(response => response.request().isNavigationRequest());
  await page.locator(`nav a[href="${path}"]`).click();
  expect((await opened).status()).toBe(200);
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(text['public-projections-title']);
  await expect(page.locator('main')).toContainText(text['public-projections-deaths-hold']);
  await expect(page.locator('main')).toContainText(text['public-projections-kills-hold']);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
});
