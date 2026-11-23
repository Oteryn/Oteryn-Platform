import * as OTPAuth from 'otpauth';
import { test, expect } from '../fixtures.mjs';
import {
  csrfRejected, logout, mailLink, password, seedPasswordRange, submit, waitForLoginBudget,
} from '../helpers.mjs';
import {
  clickPost, identityQuery, localized, localizedPage,
  seedBreachedPassword, seedPlayer, signIn, waitForIdentityBudget,
} from '../identity-helpers.mjs';

const mfaError = locale => localized(locale,
  'The request could not be verified. Check your details and try again.',
  'Nie udało się zweryfikować żądania. Sprawdź dane i spróbuj ponownie.',
);

test('login rejects wrong and closed credentials and renders the rate limit', async ({ page }, info) => {
  test.setTimeout(240_000);
  const locale = info.project.metadata.locale;
  const t = (en, pl) => localized(locale, en, pl);
  const path = `/${locale}/login`;
  const account = await seedPlayer(locale);
  await signIn(page, locale, { ...account, password: password() });
  await expect(page.locator('main [role=alert]')).toHaveText(mfaError(locale));
  await signIn(page, locale, account);
  await expect(page).toHaveURL(`/${locale}/`);
  await logout(page, locale);
  await page.goto(`/${locale}/account/sessions`);
  await expect(page).toHaveURL(path);

  // Use a separate closed fixture so the rate window starts with zero attempts.
  const limited = await seedPlayer(locale);
  expect((await identityQuery("UPDATE accounts SET status = 'closed' WHERE id = $1", [limited.id])).rowCount).toBe(1);
  await csrfRejected(page, path, { email: limited.email, password: limited.password });
  const form = page.locator('main form');
  const button = form.getByRole('button', { name: t('Sign in', 'Zaloguj się') });
  const fields = {
    _csrf: await form.locator('[name=_csrf]').inputValue(), email: limited.email, password: limited.password,
  };
  const headers = { origin: new URL(page.url()).origin };
  await waitForLoginBudget(6);
  await expect.poll(async () => {
    const { rows } = await identityQuery('SELECT extract(second FROM clock_timestamp()) AS second');
    return Number(rows[0].second);
  }, { timeout: 65_000, intervals: [1000] }).toBeLessThan(25);
  // Send the first five attempts in the same fixed window, then exercise the
  // rate-limit page by submitting the button in the browser.
  for (let attempt = 1; attempt <= 5; attempt++) {
    const response = await page.request.post(path, { form: fields, headers, maxRedirects: 0 });
    expect(response.status()).toBe(302);
    expect(response.headers().location).toBe(path);
  }
  expect((await page.reload({ waitUntil: 'domcontentloaded' })).status()).toBe(200);
  await expect(page.locator('main [role=alert]')).toHaveText(mfaError(locale));
  await form.locator('[name=email]').fill(limited.email);
  await form.locator('[name=password]').fill(limited.password);
  const limitedResponse = await clickPost(page, button, path);
  expect(limitedResponse.status()).toBe(429);
  expect(Number(limitedResponse.headers()['retry-after'])).toBeGreaterThan(0);
  await localizedPage(page, locale, t('Too many requests', 'Zbyt wiele żądań'));
  await expect(page.getByText(t('Please wait before trying again.', 'Poczekaj przed kolejną próbą.'))).toBeVisible();
});

test('password reset validates the form, replaces the password and refuses reuse', async ({ page }, info) => {
  test.setTimeout(240_000);
  const locale = info.project.metadata.locale;
  const t = (en, pl) => localized(locale, en, pl);
  const account = await seedPlayer(locale);
  const next = password();
  await seedPasswordRange(next);
  const breached = await seedBreachedPassword();
  await waitForIdentityBudget(['identity-password-recovery-source'], 4);
  await page.goto(`/${locale}/forgot-password`);
  await localizedPage(page, locale, t('Reset password', 'Zresetuj hasło'));
  await submit(page, 'main form', { email: account.email });
  await expect(page.getByText(t(
    'If an account exists for that email, a password reset link has been sent.',
    'Jeśli konto z tym adresem istnieje, wysłaliśmy link do zresetowania hasła.',
  ))).toBeVisible();
  const link = await mailLink(page, account.email,
    /https:\/\/localhost:18443\/(?:pl|en)\/reset-password\/[a-f0-9]+/);
  const values = { email: account.email, password: next, password_confirmation: next };
  const error = t(
    'This password reset link is invalid or expired, or the passwords do not match. Request a new link if needed.',
    'Link jest nieprawidłowy lub wygasł albo hasła nie są identyczne. W razie potrzeby poproś o nowy link.',
  );
  await page.goto(link);
  await submit(page, 'main form', { ...values, password_confirmation: password() });
  await expect(page.locator('main [role=alert]')).toHaveText(error);
  await submit(page, 'main form', { ...values, password: breached, password_confirmation: breached });
  await expect(page.locator('main [role=alert]')).toHaveText(t(
    'This new password appears in known data breaches. Choose a different password. If your reset link has expired, request a new one.',
    'To nowe hasło występuje w znanych wyciekach danych. Wybierz inne hasło. Jeśli link resetujący wygasł, poproś o nowy.',
  ));
  await submit(page, 'main form', values);
  await expect(page).toHaveURL(/\/login$/);
  await signIn(page, locale, account);
  await expect(page.locator('main [role=alert]')).toHaveText(mfaError(locale));
  await signIn(page, locale, { ...account, password: next });
  await expect(page).toHaveURL(`/${locale}/`);
  await page.goto(`/${locale}/forgot-password`);
  await submit(page, 'main form', { email: account.email });
  await expect(page).toHaveURL(`/${locale}/`);
  await logout(page, locale);
  await page.goto(link);
  await submit(page, 'main form', values);
  await expect(page.locator('main [role=alert]')).toHaveText(error);
});

test('TOTP, recovery codes, trusted devices and recovery-key MFA reset', async ({ page }, info) => {
  // The source quota allows five recovery submissions per hour across all projects.
  // Wait for room for both the successful recovery and the replay before starting.
  test.setTimeout(4_025_000);
  const started = Date.now();
  await expect.poll(async () => {
    const { rows } = await identityQuery(
      `SELECT COALESCE(max(count), 0) AS used FROM web_rate_limits
       WHERE limiter_name = 'identity-recovery-key-use'
         AND window_start = date_trunc('hour', clock_timestamp())`,
    );
    return Number(rows[0].used);
  }, { timeout: 3_605_000, intervals: [5000] }).toBeLessThanOrEqual(3);
  test.setTimeout(Date.now() - started + 420_000);
  const locale = info.project.metadata.locale;
  const t = (en, pl) => localized(locale, en, pl);
  const settings = `/${locale}/mfa`;
  const challenge = 'main form[action$="/mfa/challenge"]';
  const enroll = 'main form[action$="/mfa/enroll"]';
  const account = await seedPlayer(locale);
  const next = password();
  await seedPasswordRange(next);
  await page.goto(settings);
  await expect(page).toHaveURL(`/${locale}/login`);
  await signIn(page, locale, account);
  await page.goto(`/${locale}/account/recovery-key`);
  await submit(page, 'main form', { current_password: account.password });
  const key = (await page.locator('main .op-code').innerText()).trim();
  await submit(page, 'main form', { current_password: account.password, recovery_key: key });
  await expect(page.locator('main [role=status]')).toHaveText(t(
    'Your recovery key is active.', 'Klucz odzyskiwania jest aktywny.',
  ));
  await page.goto(settings);
  await localizedPage(page, locale, t('Account security', 'Bezpieczeństwo konta'));
  await submit(page, enroll, { password: password() });
  await expect(page.locator('main [role=alert]')).toHaveText(mfaError(locale));
  await submit(page, enroll, { password: account.password });
  const confirm = 'main form[action$="/mfa/confirm"]';
  const provisioning = await page.locator('main .op-prose').innerText();
  const totp = provisioning.startsWith('otpauth:')
    ? OTPAuth.URI.parse(provisioning)
    : new OTPAuth.TOTP({ secret: provisioning, algorithm: 'SHA1', digits: 6, period: 30 });
  const enrollmentCode = totp.generate();
  await submit(page, confirm, { password: account.password, code: enrollmentCode });

  const codes = page.getByRole('textbox', { name: t('Recovery code', 'Kod odzyskiwania') });
  await expect(codes.first()).toBeVisible();
  const recovery = await codes.first().inputValue();
  await page.locator('main').getByRole('link', { name: t('Sign in', 'Zaloguj się') }).click();
  await signIn(page, locale, account);
  await expect(page).toHaveURL(`${settings}/challenge`);
  await submit(page, challenge, { code: 'invalid' });
  await expect(page.locator('main [role=alert]')).toHaveText(mfaError(locale));
  // Enrollment already consumed this TOTP time step.
  await expect.poll(() => totp.generate() !== enrollmentCode, { timeout: 35_000 }).toBe(true);
  await submit(page, challenge, { code: totp.generate(), remember: true });
  await expect(page).toHaveURL(`/${locale}/`);

  await logout(page, locale);
  await signIn(page, locale, account);
  await expect(page).toHaveURL(`/${locale}/`);
  await page.goto(settings);
  await expect(page.getByRole('heading', { name: t('Trusted devices', 'Zaufane urządzenia') })).toBeVisible();
  const remove = page.getByRole('button', { name: t('Remove device 1', 'Usuń urządzenie 1') });
  await expect(remove).toBeVisible();
  await submit(page, 'main form[action$="/mfa/devices"]');
  await expect(remove).toHaveCount(0);

  await logout(page, locale);
  await signIn(page, locale, account);
  await expect(page).toHaveURL(`${settings}/challenge`);
  await submit(page, challenge, { code: recovery });
  await expect(page).toHaveURL(`/${locale}/`);
  await logout(page, locale);
  await signIn(page, locale, account);
  await expect(page).toHaveURL(`${settings}/challenge`);
  await submit(page, challenge, { code: recovery });
  await expect(page.locator('main [role=alert]')).toHaveText(mfaError(locale));

  await waitForIdentityBudget(['identity-mfa-challenge'], 3);
  const fallbackForm = 'main form[action$="/mfa/email"]';
  await expect(page.locator(fallbackForm).getByRole('button', { name: t(
    'Send a fallback code by email', 'Wyślij kod zapasowy e-mailem',
  ) })).toBeVisible();
  await submit(page, fallbackForm);
  const emailCode = await mailLink(page, account.email, /(?:\r?\n){2}[a-f0-9]{64}(?=\r?\n|$)/);
  await submit(page, challenge, { code: emailCode.trim() });
  await expect(page).toHaveURL(`/${locale}/`);

  await logout(page, locale);
  const recover = `/${locale}/recovery-key`;
  await page.goto(recover);
  const reset = { email: account.email, recovery_key: key, password: next, password_confirmation: next };
  await submit(page, 'main form', reset);
  await expect(page).toHaveURL(`/${locale}/login`);
  await waitForIdentityBudget(['identity-login'], 4);
  await signIn(page, locale, { ...account, password: next });
  await expect(page).toHaveURL(`/${locale}/`);
  await page.goto(settings);
  await expect(page.locator(enroll)).toBeVisible();
  await expect(remove).toHaveCount(0);
  await logout(page, locale);
  await page.goto(recover);
  await submit(page, 'main form', reset);
  await expect(page).toHaveURL(recover);
  await expect(page.locator('main [role=status]')).toHaveText(t(
    'The credentials are invalid or this action is unavailable.',
    'Dane są nieprawidłowe lub ta operacja jest niedostępna.',
  ));
});
