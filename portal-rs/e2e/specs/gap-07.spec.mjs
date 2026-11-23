import { test, expect } from '../fixtures.mjs';
import { password, seedPasswordRange, submit } from '../helpers.mjs';
import { action, seededAccount, seededLogin, seededStaff, seededSudo } from '../staff-rbac-helpers.mjs';

test('credential change and staff sign-out appear in Platform revocation history', async ({ page, browser }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const copy = lang === 'en' ? {
    unavailable: 'This information is currently unavailable.',
    credential: 'Sign-in credentials changed',
    kick: 'Staff requested Game sign-out',
    kickControl: 'Request Game sign-out',
    enabled: 'Enabled',
  } : {
    unavailable: 'Te informacje są obecnie niedostępne.',
    credential: 'Zmieniono dane logowania',
    kick: 'Obsługa zleciła wylogowanie z gry',
    kickControl: 'Zleć wylogowanie z gry',
    enabled: 'Włączony',
  };
  const account = await seededAccount(lang);
  await seededLogin(page, lang, account);
  await seededSudo(page, lang, account);
  await page.goto(`/${lang}/password/change`);
  const nextPassword = password();
  await seedPasswordRange(nextPassword);
  await submit(page, 'main form', {
    current_password: account.password,
    password: nextPassword,
    password_confirmation: nextPassword,
  });
  await expect(page).toHaveURL(/\/login$/);
  await seededLogin(page, lang, { ...account, password: nextPassword });

  expect((await page.request.get('/admin/players')).status()).toBe(403);
  await page.goto('/account');
  await page.locator(`main a[href="/${lang}/account/sessions"]`).first().click();
  const history = page.locator('#game-revocations');
  await expect(history).toContainText(copy.credential);
  await expect(history.getByText(copy.unavailable, { exact: true })).toBeVisible();

  const staffContext = await browser.newContext({
    baseURL: new URL(page.url()).origin,
    ignoreHTTPSErrors: true,
    locale: lang,
    viewport: info.project.use.viewport,
  });
  try {
    const staffPage = await staffContext.newPage();
    await seededStaff(staffPage, lang, 'platform_admin');
    await staffPage.goto(`/admin?lang=${lang}`);
    await staffPage.locator('a[href="/admin/players"]').first().click();
    const status = staffPage.locator('#revocation-export');
    await expect(status.getByText(copy.enabled, { exact: true })).toBeVisible();
    await expect(status.getByText(copy.unavailable, { exact: true })).toBeVisible();
    await staffPage.locator('#q').fill(account.id);
    await staffPage.locator('form[role="search"] button').click();
    await staffPage.locator(`a[href="/admin/players/${account.id}"]`).click();
    const staffHistory = staffPage.locator('#game-revocations');
    await expect(staffHistory).toContainText(copy.credential);
    await expect(staffHistory.getByRole('button', { name: copy.kickControl })).toBeVisible();

    await action(staffPage, '#game-revocations form', { reason: 'Requested by the account owner' }, 'kick');
    await expect(staffHistory.getByRole('listitem').first()).toContainText(copy.kick);
    await expect(staffHistory.getByText(copy.unavailable, { exact: true })).toBeVisible();
    await page.reload();
    await expect(history.getByRole('listitem').first()).toContainText(copy.kick);
    await expect(history).toContainText(copy.credential);
    await expect(history).not.toContainText('Requested by the account owner');
    for (const view of [history, staffHistory]) {
      expect(await view.evaluate(section => section.scrollWidth <= section.clientWidth)).toBe(true);
    }
  } finally {
    await staffContext.close();
  }
});
