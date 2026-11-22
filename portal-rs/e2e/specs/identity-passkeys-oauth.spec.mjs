import { createHash, randomBytes } from 'node:crypto';
import { test, expect } from '../fixtures.mjs';
import { csrfRejected, password, submit, waitForLoginBudget } from '../helpers.mjs';
import { clickPost, localized, localizedPage, seedPlayer, signIn, waitForIdentityBudget } from '../identity-helpers.mjs';

test('passkeys refuse anonymous enrollment and wrong passwords, then enroll, sign in and remove', async ({ page }, info) => {
  test.setTimeout(240_000);
  const locale = info.project.metadata.locale;
  const t = (en, pl) => localized(locale, en, pl);
  const account = await seedPlayer(locale);
  const path = `/${locale}/account/passkeys`;
  const register = page.getByRole('button', { name: t('Add a passkey', 'Dodaj klucz dostępu') });
  const authenticate = page.getByRole('button', { name: t('Sign in with a passkey', 'Zaloguj kluczem dostępu') });
  const remove = page.getByRole('button', { name: t('Remove passkey', 'Usuń klucz dostępu') });
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('WebAuthn.enable');
  const { authenticatorId } = await cdp.send('WebAuthn.addVirtualAuthenticator', {
    options: {
      protocol: 'ctap2', transport: 'internal', hasResidentKey: true, hasUserVerification: true,
      isUserVerified: true, automaticPresenceSimulation: true,
    },
  });
  try {
    await page.goto(path);
    await localizedPage(page, locale, t('Passkeys', 'Klucze dostępu'));
    await expect(authenticate).toBeVisible();
    await expect(register).toHaveCount(0);
    const origin = new URL(page.url()).origin;
    const csrf = await page.locator('#passkeys').getAttribute('data-csrf');
    const request = data => page.request.post(`${path}/register/start`, data);
    expect((await request({ data: { password: account.password }, headers: { origin } })).status()).toBe(403);
    expect((await request({
      data: { password: account.password }, headers: { origin, 'x-csrf-token': csrf },
    })).status()).toBe(403);

    await signIn(page, locale, account);
    await page.goto(path);
    const currentPassword = page.locator('#passkey-password');
    await currentPassword.fill(password());
    expect((await clickPost(page, register, `${path}/register/start`)).status()).toBe(403);
    await expect(page.locator('#passkey-status')).toHaveText(t(
      'The request could not be completed. Reload the page and try again.',
      'Nie udało się wykonać żądania. Odśwież stronę i spróbuj ponownie.',
    ));
    await currentPassword.fill(account.password);
    expect((await clickPost(page, register, `${path}/register/finish`)).status()).toBe(200);
    await expect(authenticate).toBeVisible();
    const { credentials } = await cdp.send('WebAuthn.getCredentials', { authenticatorId });
    expect(credentials.length).toBe(1);

    expect((await clickPost(page, authenticate, `${path}/authenticate/finish`)).status()).toBe(200);
    await expect(remove).toBeVisible();
    const id = await remove.getAttribute('data-remove');
    await currentPassword.fill(account.password);
    expect((await clickPost(page, remove, `${path}/${id}/remove`)).status()).toBe(200);
    await expect(authenticate).toBeVisible();
    await signIn(page, locale, account);
    await page.goto(path);
    await expect(register).toBeVisible();
    await expect(remove).toHaveCount(0);
  } finally {
    await cdp.send('WebAuthn.removeVirtualAuthenticator', { authenticatorId });
    await cdp.detach();
  }
});

test('OAuth consent localizes, refuses invalid forms, and handles allow and cancel', async ({ page }, info) => {
  test.setTimeout(240_000);
  const locale = info.project.metadata.locale;
  const t = (en, pl) => localized(locale, en, pl);
  const account = await seedPlayer(locale);
  const verifier = randomBytes(32).toString('base64url');
  const redirect = new URL('/__oauth/callback', info.project.use.baseURL).href;
  const state = randomBytes(24).toString('hex');
  const path = '/oauth/authorize';
  const fields = {
    response_type: 'code', client_id: 'e2e-local', redirect_uri: redirect, scope: 'game:ticket', state,
    code_challenge: createHash('sha256').update(verifier).digest('base64url'),
    code_challenge_method: 'S256',
  };
  const authorize = `${path}?${new URLSearchParams(fields)}`;
  await waitForIdentityBudget(['identity-login-source'], 8);
  await page.goto(`/${locale}/`);
  await page.goto(authorize);
  await expect(page).toHaveURL(`/${locale}/login`);
  const origin = new URL(page.url()).origin;
  const post = form => page.request.post(path, { form, headers: { origin }, maxRedirects: 0 });
  await csrfRejected(page, path, { ...fields, decision: 'approve' });
  const anonymousCsrf = await page.locator('main [name=_csrf]').inputValue();
  const anonymous = await post({ ...fields, _csrf: anonymousCsrf, decision: 'approve' });
  expect(anonymous.status()).toBe(400);
  expect((await anonymous.json()).error).toBe('invalid_grant');
  await waitForLoginBudget();
  await submit(page, 'main form', { email: account.email, password: account.password });
  await expect(page).toHaveURL(/\/oauth\/authorize\?/);
  await localizedPage(page, locale, t('Authorize game sign-in', 'Autoryzuj logowanie do gry'));
  await expect(page.locator('main')).toContainText(t(
    'Allow the registered game client to request a short-lived game login ticket for your account (game:ticket)?',
    'Czy zezwolić zarejestrowanemu klientowi gry na uzyskanie krótkotrwałego biletu logowania dla Twojego konta (game:ticket)?',
  ));

  const csrf = await page.locator('main [name=_csrf]').inputValue();
  for (const invalid of [
    { ...fields, _csrf: csrf, decision: 'invalid' },
    { ...fields, _csrf: csrf, decision: 'approve', scope: 'invalid' },
  ]) {
    const result = await post(invalid);
    expect(result.status()).toBe(400);
    expect((await result.json()).error).toBe('invalid_request');
    expect(result.headers().location).toBeUndefined();
  }
  await page.route(`${redirect}?**`, route => route.fulfill({ status: 200, body: 'Local OAuth client callback' }));
  await page.getByRole('button', { name: t('Cancel', 'Anuluj'), exact: true }).click();
  await expect(page).toHaveURL(/\/__oauth\/callback\?/);
  let result = new URL(page.url()).searchParams;
  expect(result.get('state') === state).toBe(true);
  expect(result.get('error')).toBe('access_denied');
  expect(result.has('code')).toBe(false);
  await page.goto(`${path}?${new URLSearchParams({ ...fields, scope: 'game:account-services' })}`);
  await expect(page.locator('main')).toContainText(t(
    'Allow the registered game client to access account services, including coin purchases (game:account-services)?',
    'Czy zezwolić zarejestrowanemu klientowi gry na dostęp do usług konta, w tym zakup monet (game:account-services)?',
  ));
  await page.getByRole('button', { name: t('Allow', 'Zezwól'), exact: true }).click();
  await expect(page).toHaveURL(/\/__oauth\/callback\?/);
  result = new URL(page.url()).searchParams;
  expect(result.get('state') === state).toBe(true);
  expect(result.has('error')).toBe(false);
  expect(result.has('code')).toBe(true);
});
