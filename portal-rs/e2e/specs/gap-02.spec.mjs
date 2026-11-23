import { test, expect } from '../fixtures.mjs';
import { register, login } from '../helpers.mjs';
import { seededStaff } from '../staff-rbac-helpers.mjs';

const statusPath = '/admin/native-login';

test('player finds production game sign-in status on their account', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const account = await register(page, lang);
  await login(page, lang, account);
  expect((await page.goto('/account')).status()).toBe(200);
  await page.locator('main nav a[href="#native-login"]').click();
  await expect(page).toHaveURL(/#native-login$/);
  await expect(page.locator('#native-login [role=status]')).toHaveText(
    lang === 'pl' ? 'Niedostępne' : 'Unavailable',
  );
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  expect((await page.request.get(statusPath, { maxRedirects: 0 })).status()).toBe(403);
});

test('Platform operator reaches production qualification status from the staff panel', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  await seededStaff(page, lang, 'platform_admin');
  expect((await page.goto('/admin')).status()).toBe(200);
  await page.locator(`main nav a[href="${statusPath}"]`).click();
  await expect(page).toHaveURL(new RegExp(`${statusPath}$`));
  await expect(page.locator('main h1')).toHaveText(
    lang === 'pl' ? 'Gotowość logowania do gry' : 'Native login readiness',
  );
  await expect(page.locator('main [role=status]')).toHaveAttribute('data-state', 'blocked');
  await expect(page.locator('main [data-state=pending]')).toHaveCount(5);
  await expect(page.locator('main [data-state=unavailable]')).toHaveCount(3);
  await expect(page.locator('main form')).toHaveCount(0);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});
