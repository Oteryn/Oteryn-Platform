import { test, expect } from '../fixtures.mjs';
import { csrfRejected, login } from '../helpers.mjs';
import { cookie, expectSession, otherBrowser, postForm, seedAccount } from './auth-fixtures.mjs';

test('auth form POST aliases reject missing CSRF and login rejects another browser’s token', async ({ page, context, browser }, info) => {
  const lang = info.project.metadata.locale;
  const account = await seedAccount(lang);
  const loginPath = `/${lang}/login`;
  const unissuedToken = '0'.repeat(64);
  const guestRoutes = [
    '/login', '/mfa/login', '/mfa/challenge', '/mfa/email', '/register',
    '/verify-email', '/verify-email/resend', '/forgot-password', '/reset-password',
    '/recovery-key', `/email-change/confirm/${unissuedToken}`,
    `/email-change/revert/${unissuedToken}`,
  ];
  const accountRoutes = [
    '/logout', '/account/sudo', '/password/change', '/account/sessions/others',
    '/account/sessions/all', '/mfa/enroll', '/mfa/confirm', '/mfa/devices',
    '/account/recovery-key', '/account/recovery-key/confirm',
    '/account/recovery-key/revoke', '/account/security/email',
  ];

  expect((await page.goto(loginPath)).status()).toBe(200);
  for (const prefix of ['', `/${lang}`]) {
    for (const route of guestRoutes) await csrfRejected(page, prefix + route);
  }

  const guest = await cookie(context);
  const second = await otherBrowser(browser, info);
  try {
    const other = await second.newPage();
    expect((await other.goto(loginPath)).status()).toBe(200);
    const foreignCsrf = await other.locator('main form [name=_csrf]').inputValue();
    const response = await postForm(page, 'main form', {
      _csrf: foreignCsrf,
      email: account.email,
      password: account.password,
    });
    expect(response.status()).toBe(403);
    await expectSession(guest, null);
  } finally {
    await second.close();
  }

  await login(page, lang, account);
  const authenticated = await cookie(context);
  await expectSession(authenticated, account.id);
  for (const prefix of ['', `/${lang}`]) {
    for (const route of accountRoutes) await csrfRejected(page, prefix + route);
  }
  await expectSession(authenticated, account.id);
});
