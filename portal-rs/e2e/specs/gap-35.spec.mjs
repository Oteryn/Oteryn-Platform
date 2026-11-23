import { test, expect } from '../fixtures.mjs';
import { messages, visit } from '../public-depth-helpers.mjs';
import { query, seededLogin, seededMfaAccount } from '../staff-rbac-helpers.mjs';

test('a player opens Today and checks the World daily history state', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const today = `/${lang}/today`;
  const t = messages('templates/today', lang);

  await visit(page, `/${lang}`, 200, { lang, noStore: false });
  await page.locator(`footer a[href="${today}"]`).click();
  await expect(page).toHaveURL(new RegExp(`${today}$`));

  const daily = page.locator('main section[aria-labelledby="world-daily-title"]');
  await expect(daily.getByRole('heading', { name: t.title })).toBeVisible();
  await expect(daily.getByRole('status')).toHaveText(t.unavailable);
  await expect(daily.getByRole('heading', { name: t['upcoming-raids'] })).toBeVisible();
  await expect(daily.getByText(t.unavailable, { exact: true })).toHaveCount(2);
  await expect(page.locator('main h2#news')).toBeVisible();

  const date = new Date(Date.now() - 86_400_000).toISOString().slice(0, 10);
  await daily.getByLabel(t.date).fill(date);
  const world = daily.locator('select[name="world"]');
  if (await world.count()) await world.selectOption({ index: 1 });
  await daily.getByRole('button', { name: t.apply }).click();
  await expect(page).toHaveURL(new RegExp(`[?&]date=${date}$`));
  await expect(daily.locator('time')).toHaveText(date);
  await expect(daily.getByRole('status')).toHaveText(t.unavailable);
  expect(await page.evaluate(() => document.documentElement.scrollWidth))
    .toBeLessThanOrEqual(page.viewportSize().width);

  const query = new URL(page.url()).search;
  const data = await page.request.get(`${today}/data${query}`);
  expect(data.status()).toBe(503);
  expect(data.headers()['cache-control']).toContain('no-store');
  expect(await data.json()).toEqual({ status: 'unavailable' });
  const invalid = await page.request.get(`${today}?date=2026-02-30`);
  expect(invalid.status()).toBe(400);

  await daily.getByRole('link', { name: t['current-view'] }).click();
  await expect(page).not.toHaveURL(/date=/);
  await expect(daily.getByRole('status')).toHaveText(t.unavailable);
});


test('a World operator reaches the projection status through Administration', async ({ page }, info) => {
  test.setTimeout(90_000);
  const lang = info.project.metadata.locale;
  const t = messages('templates/today', lang);
  const anonymous = await page.request.get('/admin/today', { maxRedirects: 0 });
  expect(anonymous.status()).toBe(303);
  expect(anonymous.headers().location).toBe('/login');

  const operator = await seededMfaAccount(lang);
  await query(
    "INSERT INTO account_roles (account_id, role_id) SELECT $1, id FROM roles WHERE key = 'game_viewer'",
    [operator.id],
  );
  await seededLogin(page, lang, operator);
  await visit(page, '/admin', 200, { noStore: false });
  await page.locator('main nav a[href^="/admin/today"]').click();
  await expect(page.locator('main h1')).toHaveText(t['operator-title']);
  const evidence = page.locator('main section[aria-labelledby="daily-evidence"]');
  await expect(evidence.getByRole('status')).toHaveText(t.unavailable);
  await expect(evidence.locator('time')).toHaveCount(0);
});
