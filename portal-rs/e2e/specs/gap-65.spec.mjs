import { expect, test } from '../fixtures.mjs';
import { query, seededAccount, seededLogin, seededStaff } from '../staff-rbac-helpers.mjs';

test('a player reaches hireling actions and history from account navigation', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const target = `/${lang}/panel/hirelings`;
  const unavailable = lang === 'pl' ? 'Tymczasowo niedostępne.' : 'Temporarily unavailable.';
  const anonymous = await page.request.get(target, { maxRedirects: 0 });
  expect(anonymous.status()).toBe(303);
  expect(anonymous.headers().location).toBe(`/${lang}/login`);

  const account = await seededAccount(lang);
  await seededLogin(page, lang, account);
  await page.goto(`/${lang}/`);
  if (info.project.use.viewport.width <= 960) {
    await page.locator('#portal-menu-toggle').click();
  }
  await page.locator('[data-account-link]').click();
  const link = page.getByRole('dialog').locator(`a[href="${target}"]`);
  await expect(link).toHaveText(lang === 'pl' ? 'Pomocnicy' : 'Hirelings');
  const [response] = await Promise.all([
    page.waitForResponse(response => response.request().isNavigationRequest()
      && new URL(response.url()).pathname === target),
    link.click(),
  ]);
  expect(response.status()).toBe(200);
  expect(response.headers()['cache-control']).toContain('no-store');

  const main = page.locator('main');
  for (const [section, count] of [['purchase', 1], ['hirelings', 2], ['expeditions', 6], ['history', 1]]) {
    await main.locator(`a[href="#${section}"]`).click();
    await expect(page).toHaveURL(new RegExp(`#${section}$`));
    const values = main.locator(`#${section} dd`);
    await expect(values).toHaveCount(count);
    await expect(values).toHaveText(Array(count).fill(unavailable));
  }
  await expect(main.locator('form[method="post"]')).toHaveCount(0);
  expect(await query('SELECT operation_id FROM hireling_command_intents WHERE account_id=$1', [account.id])).toEqual([]);
  expect(await page.evaluate(async () => {
    await document.fonts.ready;
    return document.documentElement.scrollWidth - innerWidth;
  })).toBeLessThanOrEqual(0);

  expect((await page.goto('/admin')).status()).toBe(403);
  await expect(page.locator('#hireling-status')).toHaveCount(0);
});

test('an authorized operator opens the hireling ledger summary from the staff panel', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  await seededStaff(page, lang, 'platform_admin');
  const response = await page.goto('/admin');
  expect(response.status()).toBe(200);
  expect(response.headers()['cache-control']).toContain('no-store');

  const summary = page.locator('#hireling-status');
  await page.locator('main nav a[href="#hireling-status"]').click();
  await expect(summary.getByRole('heading')).toHaveText(
    lang === 'pl' ? 'Status pomocników domowych' : 'House helper status',
  );
  const [{ unresolved }] = await query('SELECT count(*) AS unresolved FROM hireling_command_intents');
  await expect(summary.locator('[data-hireling-count="unresolved"]')).toHaveText(String(unresolved));
  await expect(summary).toContainText(lang === 'pl' ? 'Widoczny' : 'Visible');
  await expect(summary).toContainText(lang === 'pl' ? 'Niedostępna' : 'Unavailable');
  await expect(summary.locator('form')).toHaveCount(0);
  expect(await page.evaluate(async () => {
    await document.fonts.ready;
    return document.documentElement.scrollWidth - innerWidth;
  })).toBeLessThanOrEqual(0);
});
