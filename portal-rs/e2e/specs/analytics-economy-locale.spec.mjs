import { test, expect } from '../fixtures.mjs';
import { seedAccount, seedEntity } from '../seed-helpers.mjs';
import { loginWithRecoveryCode, query } from '../staff-rbac-helpers.mjs';

test('economy analytics localizes the unavailable view', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  expect((await page.goto('/admin/analytics/economy')).status()).toBe(403);

  const staff = await seedAccount(lang);
  await query(
    "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'",
    [staff.accountId],
  );
  const { recoveryCodes } = await seedEntity('account-mfa', {
    accountId: staff.accountId,
    password: staff.password,
  });
  await loginWithRecoveryCode(page, lang, staff, recoveryCodes[0]);

  // The preference cookie must take precedence over the conflicting header.
  await page.context().addCookies([{
    name: 'portal_language', value: lang, url: new URL(page.url()).origin,
  }]);
  await page.setExtraHTTPHeaders({ 'Accept-Language': lang === 'pl' ? 'en' : 'pl' });
  const response = await page.goto('/admin/analytics/economy');
  expect(response.status()).toBe(200);
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  await expect(page.locator('#portal-header a.brand')).toHaveAttribute('href', `/${lang}/`);
  const headings = lang === 'pl'
    ? ['Obieg złota', 'Przepływy złota', 'Handel między graczami', 'Rynek', 'Sklep w grze']
    : ['Gold circulation', 'Gold flows', 'Player trades', 'Market', 'In-game store'];
  for (const name of headings) {
    await expect(page.locator('main').getByRole('heading', { level: 2, name, exact: true })).toBeVisible();
  }
  await expect(page.locator('main')).not.toContainText(/economy_\w+_daily/);
  const end = lang === 'pl' ? 'Do (UTC, z wyłączeniem podanej daty)' : 'To (UTC, exclusive)';
  await expect(page.getByLabel(end, { exact: true })).toHaveAttribute('required', '');
  const status = page.locator('[data-state="unavailable"]');
  await expect(status).toHaveCount(5);
  const pl = 'Wymagana jest zaakceptowana baza ponownej synchronizacji oraz aktualne dane z Gry';
  const en = 'accepted resync baseline and fresh Game evidence required';
  await expect(status.first()).toContainText(lang === 'pl' ? pl : en);
  await expect(page.locator('main')).not.toContainText(lang === 'pl' ? en : pl);
  expect((await page.request.get('/admin/analytics/economy?world=invalid')).status()).toBe(400);
});
