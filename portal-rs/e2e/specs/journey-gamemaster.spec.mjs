import {test,expect} from '../fixtures.mjs';
import {register,submit,waitForLoginBudget} from '../helpers.mjs';
import {query} from '../staff-rbac-helpers.mjs';
import {bootstrapAdmin,completeEnrollment,grantSeniorRole} from '../journey-gamemaster-helpers.mjs';

test('senior GM role assignment and MFA', async ({browser,page}, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  const admin = await bootstrapAdmin(browser, info);
  const get = (path, options = {}) => page.request.get(path, {
    maxRedirects:0,
    headers:{'accept-language':lang},
    ...options,
  });

  try {
    const account = await test.step('Register and verify the future GM in a separate browser context', async () => {
      const account = await register(page, lang);
      await expect(page.locator('html')).toHaveAttribute('lang', lang);
      await expect(page.locator('main [role=status]')).toHaveText(lang === 'pl'
        ? 'Twój adres e-mail został zweryfikowany.' : 'Your e-mail has been verified.');
      const response = await get('/admin');
      expect(response.status()).toBe(303);
      expect(response.headers().location).toBe('/login');
      return account;
    });

    await test.step('Platform admin grants only game_master_senior through the Roles form', async () => {
      const [{id}] = await query('SELECT id FROM accounts WHERE email=$1', [account.email]);
      expect((await admin.page.goto('/admin')).status()).toBe(200);
      await admin.page.getByRole('link', {name:lang === 'pl' ? 'Role' : 'Roles', exact:true}).click();
      await grantSeniorRole(admin.page, id);
    });

    await test.step('GM sees the localized mandatory MFA refusal, enrolls, and completes login', async () => {
      await page.locator('#portal-header a.om-brand').click();
      const signIn = page.locator(`a.om-login[href="/${lang}/login"]`);
      if (info.project.use.viewport.width === 390) {
        const menu = page.locator('#portal-header.is-enhanced #portal-menu-toggle');
        await expect(menu).toHaveAttribute('aria-expanded', 'false');
        await menu.click();
        await expect(menu).toHaveAttribute('aria-expanded', 'true');
      }
      await signIn.click();
      await waitForLoginBudget();
      await submit(page, 'main form', {email:account.email, password:account.password});
      await expect(page).toHaveURL(new RegExp(`/${lang}/$`));
      const response = await get('/admin');
      expect(response.status()).toBe(403);
      expect(await response.text()).toContain(lang === 'pl'
        ? 'Wymagane uwierzytelnianie wieloskładnikowe'
        : 'Multi-factor authentication required');
      await completeEnrollment(page, lang, account);
    });

    await test.step('GM cannot request Platform administration pages', async () => {
      for (const path of [
        '/admin/roles', '/admin/console/access', '/admin/console/approvals',
        '/admin/world', '/admin/news', '/admin/pages', '/admin/events',
        '/admin/media', '/admin/wiki', '/admin/commerce', `/admin/${lang}/bazaar`,
      ]) {
        expect.soft((await get(path)).status(), path).toBe(403);
      }
    });

  } finally {
    await admin.context.close();
  }
});
