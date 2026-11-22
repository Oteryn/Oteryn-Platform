import { test, expect } from '../fixtures.mjs';
import { submit, waitForLoginBudget } from '../helpers.mjs';
import { loginWithRecoveryCode, seedAccount } from '../staff-rbac-helpers.mjs';

test('the staff dashboard survives sign-in and MFA on a separate host', async ({ page, context }, { project }) => {
  const lang = project.metadata.locale;
  const { credentials, recoveryCodes } = await seedAccount(lang, 'moderator');
  await loginWithRecoveryCode(page, lang, credentials, recoveryCodes[0]);
  expect((await page.goto(`/${lang}/`)).status()).toBe(200);

  const publicOrigin = new URL(page.url()).origin;
  const staffOrigin = new URL(`https://${process.env.OTERYN_ADMIN_HOST}`).origin;
  const dashboardUrl = `${staffOrigin}/admin`;
  expect(new URL(staffOrigin).hostname).not.toBe(new URL(publicOrigin).hostname);
  expect((await context.cookies(staffOrigin)).filter(cookie => cookie.name === '__Host-oteryn_session')).toHaveLength(0);

  const toggle = page.locator('#portal-menu-toggle');
  if (await toggle.isVisible()) await toggle.click();
  await page.locator('[data-account-link]').click();
  const dashboard = page.locator('#podglad-konta').getByRole('link', {
    name: lang === 'pl' ? 'Panel administracyjny' : 'Staff dashboard',
    exact: true,
  });
  await expect(dashboard).toHaveAttribute('href', dashboardUrl);
  await dashboard.click();
  await expect(page).toHaveURL(`${staffOrigin}/login?return_to=/admin`);
  await expect(page.locator('main form')).toHaveAttribute('action', `/${lang}/login?return_to=/admin`);

  await waitForLoginBudget();
  await submit(page, 'main form', credentials);
  await expect(page).toHaveURL(`${staffOrigin}/${lang}/mfa/challenge`);
  const navigation = page.waitForNavigation();
  await submit(page, 'form[action$="/mfa/challenge"]', { code: recoveryCodes[1] });
  const response = await navigation;
  expect(response.status()).toBe(200);
  await expect(page).toHaveURL(dashboardUrl);
  expect((await page.goto(`${publicOrigin}/${lang}/`)).status()).toBe(200);
  await expect(page.locator('[data-account-link]')).toBeAttached();
});
