import { test, expect } from '../fixtures.mjs';
import { login } from '../helpers.mjs';
import { loginWithRecoveryCode } from '../staff-rbac-helpers.mjs';
import { cookie, expectSession, seedAccount, seedEntity } from './auth-fixtures.mjs';

test.use({ trace: 'off', screenshot: 'off' });

for (const requiresMfa of [false, true]) {
  test(`passkey management matches route access with mandatory MFA ${requiresMfa ? 'on' : 'off'}`, async ({ page, context }, info) => {
    const lang = info.project.metadata.locale;
    const account = await seedAccount(lang);
    const signInRoute = `/${lang}/account/passkeys`;
    const route = `${signInRoute}/manage`;
    if (requiresMfa) {
      const { recoveryCodes } = await seedEntity('account-mfa', {
        accountId: account.id,
        password: account.password,
      });
      await loginWithRecoveryCode(page, lang, account, recoveryCodes[0]);
    } else {
      await login(page, lang, account);
    }
    await expectSession(await cookie(context), account.id);
    await expect(page).toHaveURL(new RegExp(`/${lang}/$`));

    if (info.project.use.viewport.width <= 960) await page.locator('#portal-menu-toggle').click();
    await page.locator('[data-account-link]').click();
    const menu = page.getByRole('dialog');
    await expect(menu).toBeVisible();
    const passkeys = menu.locator(`a[href="${route}"]`);
    if (requiresMfa) {
      expect((await page.request.get(signInRoute, {maxRedirects: 0})).status()).toBe(403);
      expect((await page.request.get(route, { maxRedirects: 0 })).status()).toBe(403);
      await expect(passkeys).toHaveCount(0);
    } else {
      expect((await page.request.get(signInRoute, {maxRedirects: 0})).status()).toBe(200);
      await expect(passkeys).toHaveText(lang === 'pl' ? 'Klucze dostępu' : 'Passkeys');
      const navigation = page.waitForResponse(response =>
        response.request().isNavigationRequest() && new URL(response.url()).pathname === route
      );
      await passkeys.click();
      expect((await navigation).status()).toBe(200);
      await expect(page.locator('[data-passkey=register]')).toBeVisible();
    }
    expect((await page.goto(`/account?lang=${lang}`)).status()).toBe(200);
    const panel = page.locator('nav[data-panel="player"]');
    await expect(panel).toBeAttached();
    await expect(panel.locator(`a[href="${route}"]`)).toHaveCount(requiresMfa ? 0 : 1);
  });
}
