import { readFileSync } from 'node:fs';
import { test, expect } from '../fixtures.mjs';
import { login, submit, waitForLoginBudget } from '../helpers.mjs';
import { cookie, expectSession, otherBrowser, seedAccount, seedEntity } from '../security/auth-fixtures.mjs';

test.use({ trace: 'off', screenshot: 'off' });

const labels = {
  pl: {
    passkey: 'Zaloguj kluczem dostępu',
    current: 'Ta sesja',
    revoked: 'Wylogowano sesję',
    passkeyError: 'Nie udało się wykonać żądania. Odśwież stronę i spróbuj ponownie.',
    codeError: 'Nie udało się potwierdzić kodu. Użyj aktualnego kodu z aplikacji, nieużytego kodu odzyskiwania albo poproś o kod e-mailem.',
  },
  en: {
    passkey: 'Sign in with a passkey',
    current: 'This session',
    revoked: 'Session signed out',
    passkeyError: 'The request could not be completed. Reload the page and try again.',
    codeError: 'The code could not be verified. Try a current authenticator code, an unused recovery code, or request a code by email.',
  },
};

async function expectFits(page, lang) {
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
}

async function accountDestination(page, path) {
  if (page.viewportSize().width <= 960) {
    const toggle = page.locator('#portal-menu-toggle');
    await expect(toggle).toBeVisible();
    if (await toggle.getAttribute('aria-expanded') === 'false') await toggle.click();
  }
  await page.locator('[data-account-link]').click();
  await page.getByRole('dialog').locator(`a[href="${path}"]`).click();
  await expect(page).toHaveURL(new RegExp(`${path}$`));
}

async function clickForPost(page, button, path, status) {
  const [response] = await Promise.all([
    page.waitForResponse(response =>
      response.request().method() === 'POST' && new URL(response.url()).pathname === path),
    button.click(),
  ]);
  expect(response.status()).toBe(status);
}

test('a passkey signs in, manages a second session, and can be removed', async ({ page, context, browser }, info) => {
  const lang = info.project.metadata.locale;
  const sessions = `/${lang}/account/sessions`;
  const passkeys = `/${lang}/account/passkeys`;
  const loginPath = `/${lang}/login`;
  const account = await seedAccount(lang);
  const cdp = await context.newCDPSession(page);
  await cdp.send('WebAuthn.enable');
  const { authenticatorId } = await cdp.send('WebAuthn.addVirtualAuthenticator', {
    options: {
      protocol: 'ctap2',
      transport: 'internal',
      hasResidentKey: true,
      hasUserVerification: true,
      isUserVerified: true,
      automaticPresenceSimulation: true,
    },
  });
  const otherContext = await otherBrowser(browser, info);

  try {
    await page.goto(loginPath);
    await expect(page.getByRole('link', { name: labels[lang].passkey })).toBeVisible();
    await waitForLoginBudget(11);
    await login(page, lang, account);
    await expect(page).toHaveURL(new RegExp(`/${lang}/$`));
    await accountDestination(page, passkeys);
    await expectFits(page, lang);
    await page.locator('#passkey-password').fill(account.password);
    await clickForPost(page, page.locator('[data-passkey=register]'), passkeys + '/register/finish', 200);
    await expect(page.locator('[data-passkey=authenticate]')).toBeVisible();

    await page.goto(loginPath);
    await page.getByRole('link', { name: labels[lang].passkey }).click();
    await expect(page).toHaveURL(new RegExp(`${passkeys}$`));
    const guest = await cookie(context);
    const start = await page.request.post(passkeys + '/authenticate/start', {
      headers: {
        origin: 'https://other.example.invalid',
        'x-csrf-token': await page.locator('#passkeys').getAttribute('data-csrf'),
      },
      data: {},
    });
    expect(start.status()).toBe(403);

    const finish = passkeys + '/authenticate/finish';
    const signIn = page.getByRole('button', { name: labels[lang].passkey });
    await page.route(url => url.pathname === finish, async route => {
      const body = route.request().postDataJSON();
      await route.continue({ postData: { ...body, credential: { ...body.credential, response: null } } });
    }, { times: 1 });
    await clickForPost(page, signIn, finish, 403);
    await expect(page.locator('#passkey-status')).toHaveText(labels[lang].passkeyError);
    await expectSession(guest, null);

    await clickForPost(page, signIn, finish, 200);
    const remove = page.locator('[data-remove]');
    await expect(remove).toHaveCount(1);
    const primarySession = await cookie(context);
    await expectSession(primarySession, account.id);
    await expectSession(guest);

    const other = await otherContext.newPage();
    await login(other, lang, account);
    const otherSession = await cookie(otherContext);
    await expectSession(otherSession, account.id);

    await page.goto(`/${lang}/`);
    await accountDestination(page, sessions);
    await expectFits(page, lang);
    const rows = page.locator('main li').filter({ has: page.locator('form') });
    await expect(rows).toHaveCount(2);
    const current = page.getByText(labels[lang].current, { exact: true });
    const revoke = rows.filter({ hasNot: current }).locator('form');
    await expect(revoke).toHaveCount(1);
    await clickForPost(page, revoke.getByRole('button'), await revoke.getAttribute('action'), 302);
    await expect(rows).toHaveCount(1);
    await expect(rows.filter({ has: current })).toHaveCount(1);
    await expect(page.getByRole('heading', { name: labels[lang].revoked })).toBeVisible();
    expect((await page.request.get(sessions, { maxRedirects: 0 })).status()).toBe(200);
    await expectSession(primarySession, account.id);
    await other.goto(sessions);
    await expect(other).toHaveURL(new RegExp(`${loginPath}$`));
    await expectSession(otherSession);

    await page.goto(passkeys);
    await page.locator('#passkey-password').fill(account.password);
    const id = await remove.getAttribute('data-remove');
    await clickForPost(page, remove, `${passkeys}/${id}/remove`, 200);
    await expect(signIn).toBeVisible();
    await expectSession(primarySession);
    await login(page, lang, account);
    await page.goto(passkeys);
    await expect(page.locator('[data-passkey=register]')).toBeVisible();
    await expect(remove).toHaveCount(0);
  } finally {
    await otherContext.close();
    await cdp.send('WebAuthn.removeVirtualAuthenticator', { authenticatorId });
    await cdp.detach();
  }
});

test('an unrecognized or used recovery code shows localized retry guidance', async ({ page, context }, info) => {
  const lang = info.project.metadata.locale;
  const challenge = `/${lang}/mfa/challenge`;
  const form = `main form[action="${challenge}"]`;
  const challengeUrl = new RegExp(`${challenge}$`);
  const home = new RegExp(`/${lang}/$`);
  const account = await seedAccount(lang);
  const { recoveryCodes: [used, fresh] } = await seedEntity('account-mfa', {
    accountId: account.id,
    password: account.password,
  });

  await login(page, lang, account);
  await expect(page).toHaveURL(challengeUrl);
  await submit(page, form, { code: used });
  await expect(page).toHaveURL(home);

  await login(page, lang, account);
  await expect(page).toHaveURL(challengeUrl);
  for (const code of ['invalid-code', used]) {
    await submit(page, form, { code });
    await expect(page).toHaveURL(challengeUrl);
    await expect(page.getByRole('alert')).toHaveText(labels[lang].codeError);
    await expect(page.locator('#code')).toHaveAttribute('aria-describedby', 'mfa-error');
    await expect(page.locator('#code')).toHaveValue('');
    await expectSession(await cookie(context), null);
    await expectFits(page, lang);
  }
  await submit(page, form, { code: fresh });
  await expect(page).toHaveURL(home);
  await expectSession(await cookie(context), account.id);
});

test('i18n-parity for sign-in and security journeys', () => {
  const keys = path => Array.from(
    readFileSync(new URL(path, import.meta.url), 'utf8').matchAll(/^([a-z][\w-]*)\s*=/gm),
    ([, key]) => key,
  ).sort();
  for (const module of ['login', 'mfa', 'passkeys']) {
    const directory = `../../templates/account/${module}`;
    expect(keys(`${directory}/pl.ftl`)).toEqual(keys(`${directory}/en.ftl`));
  }
  const sessions = '../../crates/identity/src/sessions';
  expect(keys(`${sessions}_pl.ftl`)).toEqual(keys(`${sessions}_en.ftl`));
});
