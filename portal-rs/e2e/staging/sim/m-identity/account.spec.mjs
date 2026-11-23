import {
  accountNav, expect, http, mailLink, origin, ownMail, password, player, postForm, publicScreenshot, requireMail,
  routeStaging, signIn, submit, submitWithBudget, test, totp, verifiedPlayer,
} from './helpers.mjs';

const localized = (locale, en, pl) => locale === 'en' ? en : pl;
const home = locale => `/${locale}/`;
const security = locale => `/${locale}/account/security`;
const pageOptions = { waitUntil: 'domcontentloaded' };
const emailChangeSubject = /^(Email security|Bezpieczeństwo adresu e-mail)$/;

async function confirmPassword(page, locale, account) {
  await page.goto(`/${locale}/account/sudo`, pageOptions);
  const confirmed = await submitWithBudget(page.locator('main form'), { current_password: account.password });
  expect(confirmed.status()).toBe(302);
  expect(confirmed.headers().location).toBe(security(locale));
  await expect(page).toHaveURL(`/${locale}/password/change`);
}

test.describe('Verified account', () => {
  test.beforeEach(async ({ request }) => requireMail(request));

  test('Preferences persist across locale and reload', async ({ page }, info) => {
    const { locale } = info.project.metadata;
    await verifiedPlayer(page, locale);
    await accountNav(page, `/${locale}/account/notifications`);
    await expect(page.locator('main h1')).toHaveText(localized(locale,
      'Notification preferences', 'Preferencje powiadomień'));
    const notifications = `/${locale}/account/notifications`;
    const invalid = await postForm(page, notifications, { news_email: 'invalid' });
    expect(invalid.status()).toBe(422);
    const prefs = { news_email: true, news_portal: false, support_email: true, support_portal: false };
    await submitWithBudget(page.locator('main form'), prefs, true);
    await page.reload(pageOptions);
    await expect(page.locator('main [name=news_email]')).toBeChecked();
    await expect(page.locator('main [name=news_portal]')).not.toBeChecked();
    await expect(page.locator('main [name=support_email]')).toBeChecked();
    await expect(page.locator('main [name=support_portal]')).not.toBeChecked();

    const privacy = `/${locale}/account/privacy`;
    await page.goto(privacy, pageOptions);
    const alternate = localized(locale, 'pl', 'en');
    const malformed = await postForm(page, privacy, { locale: 'invalid' });
    expect(malformed.status()).toBe(422);
    const changed = await postForm(page, privacy, {
      locale: alternate, public_account_association: '0', public_status_visible: '0',
    }, 'PUT');
    expect(changed.status()).toBe(303);
    expect(changed.headers().location).toBe(`/${alternate}/account/privacy`);
    await page.goto(`/${alternate}/account/privacy`, pageOptions);
    await expect(page.locator('main [name=locale]')).toHaveValue(alternate);
    await expect(page.locator('main [name=public_account_association]')).not.toBeChecked();
    await expect(page.locator('main [name=public_status_visible]')).not.toBeChecked();
    await page.locator('main select[name=locale]').selectOption(locale);
    await submitWithBudget(page.locator('main form'), {});
    await expect(page).toHaveURL(privacy);

    const wizard = await page.goto(`/${locale}/account/security/wizard`, pageOptions);
    expect(wizard.status()).toBe(200);
    await expect(page.locator('main h1')).toHaveText(localized(locale, 'Account security', 'Bezpieczeństwo konta'));
    const consents = `/${locale}/account/consents`;
    const consentPage = await page.goto(consents, pageOptions);
    expect(consentPage.status()).toBe(200);
    const first = page.locator('main form').first();
    const kind = await first.locator('[name=kind]').inputValue();
    const version = await first.locator('[name=version]').inputValue();
    expect((await postForm(page, consents, { kind, version: 'outdated', granted: 'true' })).status()).toBe(409);
    await submitWithBudget(first, { granted: true });
    await page.reload(pageOptions);
    await expect(first.locator('[name=granted]')).toBeChecked();
    await submitWithBudget(first, { granted: false });
    await page.reload(pageOptions);
    await expect(first.locator('[name=granted]')).not.toBeChecked();
  });

  test('Password change, export confirmation, and sign-out', async ({ page }, info) => {
    const { locale } = info.project.metadata;
    const account = await verifiedPlayer(page, locale);
    const target = `/${locale}/password/change`;
    await accountNav(page, target);
    const next = password();
    const values = { current_password: account.password, password: next, password_confirmation: next };
    await submitWithBudget(page.locator('main form'), values);
    await expect(page.locator('main [role=alert]')).toHaveText(localized(locale,
      'Check your current password and the new password fields.',
      'Sprawdź obecne hasło oraz pola nowego hasła.'));
    await confirmPassword(page, locale, account);
    const exported = `/${locale}/account/privacy/export`;
    await page.goto(exported, pageOptions);
    await expect(page.locator('main h1')).toHaveText(localized(locale, 'Download account data', 'Pobierz dane konta'));
    const download = await postForm(page, exported, {});
    expect(download.status()).toBe(200);
    expect(download.headers()['content-disposition']).toBe('attachment; filename="account-data.json"');
    const ownData = await download.text();
    expect(ownData.includes(account.email), 'the export identifies the signed-in player').toBe(true);
    await page.goto(target, pageOptions);
    await submitWithBudget(page.locator('main form'), { ...values, password_confirmation: next + 'mismatch' });
    await expect(page.locator('main [role=alert]')).toBeVisible();
    const changed = await postForm(page, target, values, 'PUT');
    expect(changed.status()).toBe(302);
    expect(changed.headers().location).toBe('/login');
    await signIn(page, locale, account);
    await expect(page.locator('main [role=alert]')).toBeVisible();
    await signIn(page, locale, { ...account, password: next });
    await expect(page).toHaveURL(home(locale));
    const sibling = await page.context().newPage();
    try {
      await sibling.goto(`/${locale}/account/sessions`, pageOptions);
      await expect(sibling.locator('main form[action$="/sessions/all"]')).toBeVisible();
      const toggle = page.locator('#portal-menu-toggle');
      if (await toggle.isVisible()) await toggle.click();
      await page.locator('header [data-account-link]').click();
      await submit(page.locator('#podglad-konta form[action$="/logout"]'));
      await expect(page).toHaveURL(home(locale));
      await sibling.reload(pageOptions);
      await expect(sibling).toHaveURL(`/${locale}/login`);
    } finally {
      await sibling.close();
    }
  });

  test('Sessions from another browser and repeated revocation', async ({ page }, info) => {
    const { locale } = info.project.metadata;
    const account = await verifiedPlayer(page, locale);
    const path = `/${locale}/account/sessions`;
    await accountNav(page, path);
    const contexts = [];
    try {
      const secondary = await page.context().browser().newContext();
      contexts.push(secondary);
      // Use the same configured request transport with credentials scoped by the helper.
      const visitor = await secondary.newPage();
      await routeStaging(secondary);
      await signIn(visitor, locale, account);
      await expect(visitor).toHaveURL(home(locale));
      await page.reload(pageOptions);
      const row = page.locator(`main li:has(form[action^="${path}/"])`)
        .filter({ hasNotText: localized(locale, 'This session', 'Ta sesja') });
      await expect(row).toHaveCount(1);
      const single = row.locator('form');
      const action = await single.getAttribute('action');
      expect((await postForm(page, `${path}/invalid`, {})).status()).toBe(400);
      await submitWithBudget(single, {});
      await expect(single).toHaveCount(0);
      expect((await postForm(page, action, {}, 'DELETE')).status()).toBe(302);
      await page.reload(pageOptions);
      await expect(page.locator('main [role=status]')).toHaveText(localized(locale,
        'The selected session is inactive or unavailable.', 'Wybrana sesja jest nieaktywna lub niedostępna.'));
      await visitor.goto(path, pageOptions);
      await expect(visitor).toHaveURL(`/${locale}/login`);
      await signIn(visitor, locale, account);
      await submitWithBudget(page.locator('main form[action$="/sessions/others"]'), {});
      await visitor.goto(path, pageOptions);
      await expect(visitor).toHaveURL(`/${locale}/login`);
      expect((await postForm(page, path, {}, 'DELETE')).status()).toBe(302);
      await submitWithBudget(page.locator('main form[action$="/sessions/all"]'), {});
      await expect(page).toHaveURL(`/${locale}/login`);
    } finally {
      await Promise.all(contexts.map(context => context.close()));
    }
  });

  test('Recovery key and email change confirmations', async ({ page }, info) => {
    const { locale } = info.project.metadata;
    const account = await verifiedPlayer(page, locale);
    const manage = `/${locale}/account/recovery-key`;
    await accountNav(page, manage);
    await submitWithBudget(page.locator('main form'), { current_password: account.password });
    const key = await page.locator('main .op-code').innerText();
    const confirmForm = page.locator('main form[action$="/confirm"]');
    await submitWithBudget(confirmForm, { current_password: account.password, recovery_key: key });
    await expect(page.locator('main [role=status]')).toHaveText(localized(locale,
      'Your recovery key is active.', 'Klucz odzyskiwania jest aktywny.'));
    expect((await page.goto(`${manage}/confirm`, pageOptions)).status()).toBe(200);
    await expect(page.locator('main .op-code')).toHaveCount(0);

    const address = player().email;
    const change = `/${locale}/account/security/email`;
    await page.goto(change, pageOptions);
    const changeForm = page.locator('main form');
    const details = {
      email: address, email_confirmation: address, current_password: account.password, proof: key, recovery: true,
    };
    const refused = localized(locale, 'Request refused or invalid input.', 'Odmowa lub nieprawidłowe dane.');
    await submitWithBudget(changeForm, { ...details, email_confirmation: player().email });
    await expect(page.locator('main [role=status]')).toHaveText(refused);
    const since = Date.now() - 1_000;
    await submitWithBudget(changeForm, details);
    await expect(page.locator('main [role=status]')).toHaveText(localized(locale,
      'Check both email addresses for confirmation and cancellation links.',
      'Sprawdź oba adresy e-mail, aby potwierdzić lub anulować zmianę.'));
    const confirmation = await mailLink(page.request, address, emailChangeSubject,
      /\/email-change\/confirm\/[a-f0-9]{64}$/, since);
    const reversal = await mailLink(page.request, account.email, emailChangeSubject,
      /\/email-change\/revert\/[a-f0-9]{64}$/, since);
    await page.goto(confirmation.href, pageOptions);
    await submitWithBudget(page.locator('main form'), {});
    await expect(page).toHaveURL(`/${locale}/login`);
    await signIn(page, locale, { ...account, email: address });
    await expect(page).toHaveURL(home(locale));
    await page.goto(reversal.href, pageOptions);
    await submitWithBudget(page.locator('main form'), {});
    await expect(page).toHaveURL(`/${locale}/login`);
    await page.goto(reversal.href, pageOptions);
    await submitWithBudget(page.locator('main form'), {});
    await expect(page.locator('main [role=status]')).toHaveText(refused);
    await signIn(page, locale, account);
    await expect(page).toHaveURL(home(locale));
    await page.goto(manage, pageOptions);
    const inactiveKey = await postForm(page, manage, { current_password: account.password }, 'DELETE');
    expect(inactiveKey.status()).toBe(302);
    expect(inactiveKey.headers().location).toBe(manage);
    await page.reload(pageOptions);
    await expect(page.locator('main [role=status]')).toHaveText(localized(locale,
      'The credentials are invalid or this action is unavailable.',
      'Dane są nieprawidłowe lub ta operacja jest niedostępna.'));
  });

  test('Recovery key revocation and one-time use', async ({ page, browser }, info) => {
    const { locale } = info.project.metadata;
    const account = await verifiedPlayer(page, locale);
    const manage = `/${locale}/account/recovery-key`;
    const recover = `/${locale}/recovery-key`;
    const current = { current_password: account.password };
    await accountNav(page, manage);
    await submitWithBudget(page.locator('main form'), current);
    const first = await page.locator('main .op-code').innerText();
    await submitWithBudget(page.locator('main form'), { ...current, recovery_key: first });
    await page.goto(manage, pageOptions);
    const revoked = await postForm(page, `${manage}/revoke`, current);
    expect(revoked.status()).toBe(302);
    expect(revoked.headers().location).toBe(manage);
    await page.reload(pageOptions);
    await submitWithBudget(page.locator('main form'), current);
    const nextKey = await page.locator('main .op-code').innerText();
    await submitWithBudget(page.locator('main form'), { ...current, recovery_key: nextKey });
    const independent = await browser.newContext();
    try {
      await routeStaging(independent);
      const guest = await independent.newPage();
      const nextPassword = password();
      const details = { email: account.email, password: nextPassword, password_confirmation: nextPassword };
      const form = guest.locator('main form');
      await guest.goto(recover, pageOptions);
      await submitWithBudget(form, { ...details, recovery_key: first });
      await expect(guest.locator('main [role=status]')).toHaveText(localized(locale,
        'The credentials are invalid or this action is unavailable.',
        'Dane są nieprawidłowe lub ta operacja jest niedostępna.'));
      await submitWithBudget(form, { ...details, recovery_key: nextKey });
      await expect(guest).toHaveURL(`/${locale}/login`);
      await page.goto(`/${locale}/account/sessions`, pageOptions);
      await expect(page).toHaveURL(`/${locale}/login`);
      await signIn(guest, locale, { ...account, password: nextPassword });
      await expect(guest).toHaveURL(home(locale));
    } finally {
      await independent.close();
    }
  });

  test('Authenticator enrolment and trusted device', async ({ page }, info) => {
    const { locale } = info.project.metadata;
    const account = await verifiedPlayer(page, locale);
    const { secret, code, recoveryCode } = await enrollAuthenticator(page, locale, account);
    await signIn(page, locale, account);
    const challenge = `/${locale}/mfa/challenge`;
    await expect(page).toHaveURL(challenge);
    await page.reload(pageOptions);
    const form = page.locator('main form[action$="/mfa/challenge"]');
    await submitWithBudget(form, { code: 'invalid' });
    await expect(page.locator('main [role=alert]')).toBeVisible();
    await expect.poll(() => totp(secret) !== code, { timeout: 32_000, intervals: [1_000] }).toBe(true);
    await submitWithBudget(form, { code: totp(secret), remember: true });
    await expect(page).toHaveURL(home(locale));
    await page.goto(`/${locale}/mfa`, pageOptions);
    const remove = page.locator('main form[action$="/mfa/devices"]');
    await expect(remove).toHaveCount(1);
    await submitWithBudget(remove, {});
    await page.reload(pageOptions);
    await expect(remove).toHaveCount(0);

    const fresh = await page.context().browser().newContext();
    try {
      await routeStaging(fresh);
      const other = await fresh.newPage();
      await signIn(other, locale, account);
      await emailChallenge(other, locale, account);
    } finally {
      await fresh.close();
    }

    const recoveryBrowser = await page.context().browser().newContext();
    try {
      await routeStaging(recoveryBrowser);
      const other = await recoveryBrowser.newPage();
      await signIn(other, locale, account);
      await submitWithBudget(other.locator('main form[action$="/mfa/challenge"]'), { code: recoveryCode });
      await expect(other).toHaveURL(home(locale));
    } finally {
      await recoveryBrowser.close();
    }
  });

  test('Two players keep independent notification preferences', async ({ page, browser }, info) => {
    const { locale } = info.project.metadata;
    const path = `/${locale}/account/notifications`;
    await verifiedPlayer(page, locale);
    await accountNav(page, path);
    const news = page.locator('main [name=news_email]');
    const original = await news.isChecked();
    const second = await browser.newContext();
    try {
      await routeStaging(second);
      const other = await second.newPage();
      await verifiedPlayer(other, locale);
      await accountNav(other, path);
      const otherNews = other.locator('main [name=news_email]');
      await expect(otherNews).toBeChecked({ checked: original });
      await submitWithBudget(page.locator('main form'), { news_email: !original });
      await page.reload(pageOptions);
      await expect(news).toBeChecked({ checked: !original });
      await other.reload(pageOptions);
      await expect(otherNews).toBeChecked({ checked: original });
    } finally {
      await second.close();
    }
  });

  test('Passkey registration, sign-in, and removal', async ({ page, context }, info) => {
    const { locale } = info.project.metadata;
    const account = await verifiedPlayer(page, locale);
    const path = `/${locale}/account/passkeys`;
    await accountNav(page, path);
    const node = page.locator('#passkeys');
    const csrf = await node.getAttribute('data-csrf');
    const refused = await http(page.request, 'POST', `${path}/register/start`, {
      headers: { origin, 'x-csrf-token': csrf }, data: { password: 'Incorrect test password' },
    });
    expect(refused.status()).toBe(403);
    const device = await context.newCDPSession(page);
    await device.send('WebAuthn.enable');
    const { authenticatorId } = await device.send('WebAuthn.addVirtualAuthenticator', {
      options: {
        protocol: 'ctap2', transport: 'internal', hasResidentKey: true,
        hasUserVerification: true, isUserVerified: true, automaticPresenceSimulation: true,
      },
    });
    try {
      const finish = async (button, operation) => {
        const response = page.waitForResponse(result =>
          new URL(result.url()).pathname === `${path}/${operation}`
            && result.request().method() === 'POST');
        await button.click();
        expect((await response).status(), operation).toBe(200);
      };
      await page.locator('#passkey-password').fill(account.password);
      await finish(page.locator('[data-passkey=register]'), 'register/finish');
      await expect(page.locator('[data-passkey=authenticate]')).toBeVisible();
      await finish(page.locator('[data-passkey=authenticate]'), 'authenticate/finish');
      const remove = page.locator('[data-remove]');
      await expect(remove).toHaveCount(1);
      const name = await remove.getAttribute('data-remove');
      await page.locator('#passkey-password').fill(account.password);
      await finish(remove, `${name}/remove`);
      await expect(page.locator('[data-passkey=authenticate]')).toBeVisible();
      await signIn(page, locale, account);
      await page.goto(path, pageOptions);
      await expect(remove).toHaveCount(0);
    } finally {
      await device.send('WebAuthn.removeVirtualAuthenticator', { authenticatorId });
      await device.detach();
    }
  });

  test('Account deletion request and cancellation', async ({ page }, info) => {
    const { locale } = info.project.metadata;
    const account = await verifiedPlayer(page, locale);
    const path = `/${locale}/account/privacy/delete`;
    const settings = await page.goto(path, pageOptions);
    if (settings.status() === 404) {
      const unavailable = await http(page.request, 'POST', path, { form: {}, headers: { origin } });
      expect(unavailable.status()).toBe(405);
      await expect(page.locator('main')).toContainText('404');
      return;
    }
    expect(settings.status()).toBe(200);
    const values = { current_password: account.password, confirmation: 'TERMINATE' };
    expect((await postForm(page, path, values)).status()).toBe(403);
    await confirmPassword(page, locale, account);
    await page.goto(path, pageOptions);
    expect((await postForm(page, path, { ...values, confirmation: 'NO' })).status()).toBe(422);
    await submitWithBudget(page.locator('main form'), values);
    await expect(page).toHaveURL(`/${locale}/login`);
    await signIn(page, locale, account);
    await confirmPassword(page, locale, account);
    await page.goto(path, pageOptions);
    const cancel = page.locator('main form[action$="/cancel"]');
    await expect(cancel).toBeVisible();
    await submitWithBudget(cancel, { current_password: account.password });
    await page.reload(pageOptions);
    await expect(cancel).toHaveCount(0);
    expect((await postForm(page, `${path}/cancel`, { current_password: account.password })).status()).toBe(404);
  });
});

async function enrollAuthenticator(page, locale, account) {
  await page.goto(`/${locale}/mfa`, pageOptions);
  const form = page.locator('main form[action$="/mfa/enroll"]');
  await submitWithBudget(form, { password: 'Incorrect test password' });
  await expect(page).toHaveURL(`/${locale}/mfa`);
  await submitWithBudget(form, { password: account.password });
  const secret = (await page.locator('main .op-prose').innerText()).trim();
  const code = totp(secret);
  await submitWithBudget(page.locator('main form[action$="/mfa/confirm"]'), {
    password: account.password, code,
  });
  const recovery = page.locator('main textarea[readonly]').first();
  await expect(recovery).toBeVisible();
  return { secret, code, recoveryCode: await recovery.inputValue() };
}

async function emailChallenge(page, locale, account) {
  const since = Date.now() - 1_000;
  await submitWithBudget(page.locator('main form[action$="/mfa/email"]'), {});
  const delivery = await ownMail(page.request, account.email,
    /^(Your Oteryn verification code|Twój kod weryfikacyjny Oteryn)$/, since);
  const fallback = delivery.Text?.match(/^\s*(\d{6})\s*$/m)?.[1];
  if (!fallback) throw new Error('The MFA email did not contain an authenticator code.');
  await submitWithBudget(page.locator('main form[action$="/mfa/challenge"]'), { code: fallback });
  await expect(page).toHaveURL(home(locale));
}

test('Staff password reset and authenticator sign-in', async ({ page, request }, info) => {
  await requireMail(request);
  const { locale } = info.project.metadata;
  const roles = {
    'pl-1440': 'platform-admin', 'pl-390': 'security-admin',
    'en-1440': 'support', 'en-390': 'game-viewer',
  };
  const account = { email: `staff-${roles[info.project.name]}@example.invalid`, password: password() };
  const link = await requestReset(page, locale, account);
  await finishPasswordReset(page, account, link);
  await signIn(page, locale, account);
  const challenge = `/${locale}/mfa/challenge`;
  await expect(page).toHaveURL(url => [home(locale), challenge].includes(url.pathname));
  if (new URL(page.url()).pathname === challenge) {
    await emailChallenge(page, locale, account);
  } else {
    const { secret, code } = await enrollAuthenticator(page, locale, account);
    await signIn(page, locale, account);
    await expect(page).toHaveURL(challenge);
    await expect.poll(() => totp(secret) !== code, { timeout: 32_000, intervals: [1_000] }).toBe(true);
    await submitWithBudget(page.locator('main form[action$="/mfa/challenge"]'), { code: totp(secret) });
  }
  await expect(page).toHaveURL(home(locale));
  const route = await page.goto(security(locale), pageOptions);
  expect(route.status()).toBe(200);
  await expect(page.locator('main h1')).toHaveText(localized(locale, 'Change password', 'Zmień hasło'));
});

test('Player password reset invalidates the earlier password', async ({ page, request, browser }, info) => {
  await requireMail(request);
  const { locale } = info.project.metadata;
  const account = await verifiedPlayer(page, locale);
  const visitor = await browser.newContext();
  try {
    await routeStaging(visitor);
    const anonymous = await visitor.newPage();
    const next = { ...account, password: password() };
    const link = await requestReset(anonymous, locale, account);
    await finishPasswordReset(anonymous, next, link);
    await page.goto(`/${locale}/account/sessions`, pageOptions);
    await expect(page).toHaveURL(`/${locale}/login`);
    await signIn(anonymous, locale, account);
    await expect(anonymous.locator('main [role=alert]')).toBeVisible();
    await signIn(anonymous, locale, next);
    await expect(anonymous).toHaveURL(home(locale));
  } finally {
    await visitor.close();
  }
});

async function requestReset(page, locale, account) {
  await page.goto(`/${locale}/forgot-password`, pageOptions);
  const since = Date.now() - 1_000;
  await submitWithBudget(page.locator('main form'), { email: account.email });
  await expect(page.locator('main [role=status]')).toHaveText(localized(locale,
    'If an account exists for that email, a password reset link has been sent.',
    'Jeśli konto z tym adresem istnieje, wysłaliśmy link do zresetowania hasła.'));
  return mailLink(page.request, account.email,
    /^(Reset your Oteryn password|Zresetuj hasło do Oteryn)$/,
    /^\/(en|pl)\/reset-password\/[a-f0-9]{64}$/, since);
}

async function finishPasswordReset(page, account, link) {
  const locale = link.pathname.split('/')[1];
  await page.goto(link.href, pageOptions);
  const form = page.locator('main form');
  const details = { ...account, password_confirmation: account.password };
  const reset = await submitWithBudget(form, details);
  if (reset.status() >= 500) {
    await page.locator('main').getByText(String(reset.status()), { exact: true })
      .waitFor({ state: 'visible', timeout: 5_000 }).catch(() => {});
    await publicScreenshot(page, test.info(), 'password-reset-result').catch(() => {});
  }
  expect.soft(reset.status(), 'password reset submission').toBe(302);
  await expect(page).toHaveURL(`/${locale}/login`);
  await page.goto(link.href, pageOptions);
  await submitWithBudget(form, details);
  await expect(page.locator('main [role=alert]')).toBeVisible();
}
