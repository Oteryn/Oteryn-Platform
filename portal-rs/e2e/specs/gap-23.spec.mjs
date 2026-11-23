import {test, expect} from '../fixtures.mjs';
import {seededAccount, seededLogin, seededStaff} from '../staff-rbac-helpers.mjs';
import {assertShell} from '../shell.mjs';

test('release operators can preview an enforcing rollout from the staff panel', async ({page}, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  await seededStaff(page, lang, 'platform_admin');
  expect((await page.goto('/admin')).status()).toBe(200);

  await page.locator('main nav a[href="/admin/client-build"]').click();
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  await expect(page.locator('[data-update-channel]')).toHaveCount(2);
  await expect(page.locator('[data-game-readiness]')).toHaveText(lang === 'pl' ? 'Niedostępne' : 'Unavailable');
  await assertShell(page);

  await page.locator('select[name=channel]').selectOption('beta');
  await page.locator('select[name=mode]').selectOption('enforcing');
  await page.locator('select[name=unknown]').selectOption('refuse');
  await page.locator('form[action="/admin/client-build"] button').click();
  await expect(page.locator('[data-preview-result]')).toContainText('NATIVE_LOGIN_OFFER_UNSUPPORTED');
  await expect(page.locator('select[name=channel]')).toHaveValue('beta');
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);

  const malformed = await page.goto('/admin/client-build?channel=beta&release=rel-a&sequence=0');
  expect(malformed.status()).toBe(400);
  await expect(page.locator('main [role=alert]')).toBeVisible();
});

test('the qualification page requires staff access', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  await seededLogin(page, lang, await seededAccount(lang));
  expect((await page.goto('/admin/client-build')).status()).toBe(403);
});
