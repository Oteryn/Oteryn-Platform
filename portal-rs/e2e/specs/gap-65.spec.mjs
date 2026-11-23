import { readFileSync } from 'node:fs';
import { expect, test } from '../fixtures.mjs';
import { assertShell } from '../shell.mjs';
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
  await assertShell(page);

  const main = page.locator('main');
  for (const [section, count] of [['purchase', 1], ['hirelings', 2], ['expeditions', 6], ['history', 1]]) {
    await main.locator(`a[href="#${section}"]`).click();
    await expect(page).toHaveURL(new RegExp(`#${section}$`));
    const values = main.locator(`#${section} dd`);
    await expect(values).toHaveCount(count);
    await expect(values).toHaveText(Array(count).fill(unavailable));
  }
  await expect(main.locator('form[method="post"]')).toHaveCount(0);
  expect((await page.request.get('/invalid/panel/hirelings')).status()).toBe(404);
  expect((await page.request.post(target)).status()).toBe(403);
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
  const [{ unresolved, attention }] = await query(`
    SELECT count(*) AS unresolved, count(*) FILTER (WHERE NOT EXISTS (
      SELECT 1 FROM outbox WHERE kind = 'hirelings.command.v1'
        AND idempotency_key = intent.operation_id::text AND state <> 'dead'
    )) AS attention FROM hireling_command_intents AS intent
  `);
  await expect(summary.locator('[data-hireling-count="unresolved"]')).toHaveText(String(unresolved));
  await expect(summary.locator('[data-hireling-count="dispatch-attention"]')).toHaveText(String(attention));
  await expect(summary).toContainText(lang === 'pl' ? 'Widoczny' : 'Visible');
  await expect(summary).toContainText(lang === 'pl' ? 'Niedostępna' : 'Unavailable');
  await expect(summary.locator('form')).toHaveCount(0);
  expect(await page.evaluate(async () => {
    await document.fonts.ready;
    return document.documentElement.scrollWidth - innerWidth;
  })).toBeLessThanOrEqual(0);
});

test('i18n-parity for the hireling views', () => {
  for (const dir of ['../../crates/hirelings/locales/', '../../templates/admin/']) {
    const keys = lang => Array.from(readFileSync(new URL(`${dir}${lang}.ftl`, import.meta.url), 'utf8')
      .matchAll(/^([a-z][\w-]*)\s*=/gm), match => match[1]).sort();
    const english = keys('en');
    expect(english.length).toBeGreaterThan(0);
    expect(new Set(english).size).toBe(english.length);
    expect(keys('pl')).toEqual(english);
  }
});
