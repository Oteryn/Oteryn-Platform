import { readFileSync } from 'node:fs';
import { test, expect } from '../fixtures.mjs';
import { register, login } from '../helpers.mjs';
import { messages } from '../public-depth-helpers.mjs';
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
  await expect(page.locator('#switches + p + dl dt')).toHaveText(lang === 'pl'
    ? ['Bilety logowania do gry', 'Żądania wejścia do gry', 'Wydawanie zezwoleń na wejście do gry']
    : ['Game login tickets', 'Native admission requests', 'Native grant issuance']);
  await expect(page.locator('main form')).toHaveCount(0);
  await expect(page.locator('header#portal-header')).toHaveCount(1);
  await expect(page.locator('footer.foot')).toHaveCount(1);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);

  const preferred = lang === 'pl' ? 'en' : 'pl';
  const preferredTitle = messages('templates/admin', preferred)['native-login-title'];
  await page.context().addCookies([{
    name: 'portal_language',
    value: preferred,
    url: new URL(page.url()).origin,
  }]);
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('lang', preferred);
  await expect(page.locator('main h1')).toHaveText(preferredTitle);
  await page.locator('main a[href="/admin"]').click();
  await expect(page.locator(`main nav a[href="${statusPath}"]`)).toHaveText(preferredTitle);
});

test('i18n-parity for native sign-in status', () => {
  const statusKeys = catalogue => Object.keys(catalogue).filter(key => key.startsWith('native-login')).sort();
  const overviewKeys = lang => {
    const file = new URL(`../../templates/account/overview.${lang}.ftl`, import.meta.url);
    const source = readFileSync(file, 'utf8');
    return [...source.matchAll(/^(native-login[\w-]*) = \S.*$/gm)].map(([, key]) => key).sort();
  };
  const english = statusKeys(messages('templates/admin', 'en'));
  expect(english).toHaveLength(23);
  expect(statusKeys(messages('templates/admin', 'pl'))).toEqual(english);
  for (const lang of ['pl', 'en']) {
    expect(overviewKeys(lang)).toEqual(['native-login', 'native-login-help']);
  }
});
