import { test, expect } from '../fixtures.mjs';
import { csrfRejected, login, logout, password, seedPasswordRange, submit } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { cookie, expectSession, hash, otherBrowser, seedAccount } from './auth-fixtures.mjs';

test('login and sudo rotate sessions, and logout removes the active session', async ({ page, context, browser }, info) => {
  const lang = info.project.metadata.locale;
  const account = await seedAccount(lang);
  const exportPath = `/${lang}/account/privacy/export`;
  const second = await otherBrowser(browser, info);
  try {
    await page.goto(`/${lang}/login`);
    const anonymous = await cookie(context);
    await expectSession(anonymous, null);
    await submit(page, 'main form', { email: account.email, password: account.password });
    await expect(page).toHaveURL(new RegExp(`/${lang}/$`));
    const signedIn = await cookie(context);
    expect(signedIn !== anonymous, 'login rotated the guest session').toBe(true);
    await expectSession(signedIn, account.id);
    await expectSession(anonymous);
    expect((await page.request.get(exportPath, { maxRedirects: 0 })).status()).toBe(403);

    const other = await second.newPage();
    await login(other, lang, account);
    await expectSession(await cookie(second), account.id);

    await page.goto(`/${lang}/account/sudo`);
    await submit(page, 'main form', { current_password: account.password });
    await expect(page).toHaveURL(new RegExp(`/${lang}/password/change$`));
    const elevated = await cookie(context);
    expect(elevated !== signedIn, 'step-up rotated the session').toBe(true);
    await expectSession(elevated, account.id);
    await expectSession(signedIn);
    expect((await page.request.get(exportPath)).status()).toBe(200);
    expect((await other.request.get(exportPath, { maxRedirects: 0 })).status()).toBe(403);
    const expired = await query(
      "UPDATE web_sessions SET sudo_until=clock_timestamp()-interval '1 second' WHERE id_hash=$1 AND account_id=$2 RETURNING account_id",
      [hash(elevated), account.id],
    );
    expect(expired).toEqual([{ account_id: account.id }]);
    expect((await page.request.get(exportPath, { maxRedirects: 0 })).status()).toBe(403);


    await logout(page, lang);
    await expectSession(elevated);
    await page.goto(`/${lang}/account/sessions`);
    await expect(page).toHaveURL(/\/login$/);
  } finally {
    await second.close();
  }
});

test('session list and targeted revoke preserve the current browser, then password change revokes it', async ({ page, context, browser }, info) => {
  const lang = info.project.metadata.locale;
  const account = await seedAccount(lang);
  const sessionsPath = `/${lang}/account/sessions`;
  const second = await otherBrowser(browser, info);
  try {
    await login(page, lang, account);
    const primary = await cookie(context);
    await expectSession(primary, account.id);
    const other = await second.newPage();
    await login(other, lang, account);
    const secondary = await cookie(second);
    await expectSession(secondary, account.id);
    const [{ session_id: id }] = await query(
      'SELECT session_id FROM web_sessions WHERE id_hash=$1 AND account_id=$2',
      [hash(secondary), account.id],
    );

    expect((await page.goto(sessionsPath)).status()).toBe(200);
    await expect(page.locator('main li form')).toHaveCount(2);
    await csrfRejected(page, `${sessionsPath}/others`);
    await csrfRejected(page, `${sessionsPath}/all`);
    await submit(page, `form[action="${sessionsPath}/${id}"]`);
    await expect(page.locator('main li form')).toHaveCount(1);
    await other.goto(sessionsPath);
    await expect(other).toHaveURL(/\/login$/);
    await expectSession(secondary);
    await expectSession(primary, account.id);

    await login(other, lang, account);
    const concurrent = await cookie(second);
    await expectSession(concurrent, account.id);
    await page.goto(`/${lang}/account/sudo`);
    await submit(page, 'main form', { current_password: account.password });
    const beforeChange = await cookie(context);
    await expectSession(beforeChange, account.id);
    const next = password();
    await seedPasswordRange(next);
    await page.goto(`/${lang}/password/change`);
    await submit(page, 'main form', {
      current_password: account.password, password: next, password_confirmation: next,
    });
    await expect(page).toHaveURL(/\/login$/);
    await expectSession(beforeChange);
    await expectSession(concurrent);
    expect(await cookie(context) !== beforeChange, 'password change replaced the active session').toBe(true);
    await login(page, lang, { ...account, password: next });
    await expectSession(await cookie(context), account.id);
  } finally {
    await second.close();
  }
});
