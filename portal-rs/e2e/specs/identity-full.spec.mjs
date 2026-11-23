import { createHash } from 'node:crypto';
import { test, expect } from '../fixtures.mjs';
import { csrfRejected, logout, mailLink, password, seedPasswordRange, submit } from '../helpers.mjs';
import {
  clickPost, identityEmail, identityQuery, localized, localizedPage,
  seedBreachedPassword, signIn, waitForIdentityBudget,
} from '../identity-helpers.mjs';

const verifyURL = /https:\/\/localhost:18443\/(?:pl|en)\/verify-email\?token=(?:3D)?[a-f0-9]{64}/;
const registrationPolicies = [
  'identity-registration', 'identity-registration-attempt', 'identity-registration-challenge',
];

test('registration validates every field, current consent versions and breached passwords', async ({ page }, info) => {
  test.setTimeout(180_000);
  const locale = info.project.metadata.locale;
  const t = (en, pl) => localized(locale, en, pl);
  const path = `/${locale}/register`;
  const email = identityEmail();
  const breached = await seedBreachedPassword();
  await waitForIdentityBudget(registrationPolicies, 2);
  await page.goto(path);
  await localizedPage(page, locale, t('Create an account', 'Utwórz konto'));
  const main = page.locator('main');
  for (const label of ['E-mail', t('Password', 'Hasło'), t('Confirm password', 'Powtórz hasło')]) {
    await expect(main.getByLabel(label, { exact: true })).toBeVisible();
  }
  await expect(main.getByLabel(t('I accept the current terms', 'Akceptuję aktualny regulamin'))).toBeVisible();
  await expect(main.getByLabel(t('I accept the current privacy policy', 'Akceptuję aktualną politykę prywatności'))).toBeVisible();
  await csrfRejected(page, `/${locale}/register/abuse/challenge`);
  await expect(page.locator('#password-hint')).toHaveText(t(
    'At least 10 characters, at most 256 bytes.', 'Co najmniej 10 znaków, maksymalnie 256 bajtów.',
  ));
  for (const [document, label] of [
    ['terms', t('Read the terms', 'Przeczytaj regulamin')],
    ['privacy', t('Read the privacy policy', 'Przeczytaj politykę prywatności')],
  ]) {
    const destination = `/${locale}/legal/${document}`;
    const loaded = page.waitForResponse(response =>
      response.request().isNavigationRequest() && new URL(response.url()).pathname === destination);
    await page.getByRole('link', { name: label, exact: true }).click();
    expect((await loaded).status()).toBe(200);
    await expect(page).toHaveURL(destination);
    await page.goBack();
  }
  const form = page.locator('main form');
  const button = form.getByRole('button', { name: t('Register', 'Zarejestruj się'), exact: true });
  // Disable the browser's built-in validation to exercise the server's field errors.
  await form.evaluate(element => { element.noValidate = true; });
  await submit(page, 'main form', { email: 'invalid', password: 'short', password_confirmation: 'different' });
  await expect(page.getByRole('alert')).toHaveText(t('Please correct the fields below.', 'Popraw poniższe pola.'));
  const consentError = t('Accept the current document version.', 'Zaakceptuj aktualną wersję dokumentu.');
  for (const [id, message] of [
    ['email', t('Enter a valid e-mail address.', 'Podaj poprawny adres e-mail.')],
    ['password', t('Use at least 10 characters and at most 256 bytes.', 'Użyj co najmniej 10 znaków i maksymalnie 256 bajtów.')],
    ['confirmation', t('Passwords must match.', 'Hasła muszą być takie same.')],
    ['terms', consentError],
    ['privacy', consentError],
  ]) {
    await expect(page.locator(`#${id}-error`)).toHaveText(message);
  }
  for (const name of ['terms', 'privacy']) {
    const consent = form.locator(`[name=${name}]`);
    await consent.check();
    await consent.evaluate(element => { element.value = 'obsolete-version'; });
  }
  await form.locator('[name=email]').fill(email);
  await form.locator('[name=password]').fill(breached);
  await form.locator('[name=password_confirmation]').fill(breached);
  expect((await clickPost(page, button, path)).status()).toBe(422);
  await expect(page.locator('#terms-error')).toHaveText(consentError);
  await expect(page.locator('#privacy-error')).toHaveText(consentError);

  await waitForIdentityBudget(registrationPolicies, 3);
  const incomplete = await page.request.post(path, {
    headers: { origin: new URL(page.url()).origin },
    form: {
      _csrf: await form.locator('[name=_csrf]').inputValue(),
      email,
      password: breached,
      password_confirmation: breached,
      terms: await form.locator('[name=terms]').inputValue(),
      privacy: await form.locator('[name=privacy]').inputValue(),
    },
  });
  expect(incomplete.status()).toBe(422);
  expect((await incomplete.text()).includes(t(
    'Verification failed or expired. Return to the form and try again.',
    'Weryfikacja nie powiodła się lub wygasła. Wróć do formularza i spróbuj ponownie.',
  ))).toBe(true);

  await waitForIdentityBudget(registrationPolicies, 4);
  await submit(page, 'main form', {
    email, password: breached, password_confirmation: breached, terms: true, privacy: true,
  });
  await expect(page.locator('main').getByText(t(
    'This password appears in a data breach. Choose a different password.',
    'To hasło występuje w wycieku danych. Wybierz inne hasło.',
  ), { exact: true })).toBeVisible();
  expect((await identityQuery('SELECT 1 FROM accounts WHERE email = $1', [email])).rowCount).toBe(0);
  await page.locator('main').getByRole('link', { name: t('Back to home', 'Wróć na stronę główną') }).click();
  await expect(page).toHaveURL(`/${locale}/`);
});

test('verification expires, resends and activates while repeated registration remains private', async ({ page }, info) => {
  test.setTimeout(240_000);
  const locale = info.project.metadata.locale;
  const t = (en, pl) => localized(locale, en, pl);
  const account = { email: identityEmail(), password: password() };
  await seedPasswordRange(account.password);
  await waitForIdentityBudget(registrationPolicies, 4);
  await page.goto(`/${locale}/register`);
  const documents = await page.locator('main input[type=checkbox]').evaluateAll(
    inputs => inputs.map(input => ({ document_kind: input.name, document_version: input.value })),
  );
  await submit(page, 'main form', {
    ...account, password_confirmation: account.password, terms: true, privacy: true,
  });
  await expect(page).toHaveURL(`/${locale}/register/check-inbox`, { timeout: 90_000 });
  const inbox = t(
    'Check your inbox for an e-mail with the next steps.',
    'Sprawdź skrzynkę odbiorczą — wysłaliśmy wiadomość z dalszymi krokami.',
  );
  await expect(page.locator('main [role=status]')).toHaveText(inbox);
  const { rows: consents } = await identityQuery(
    `SELECT document_kind, document_version FROM account_consents
       JOIN accounts ON accounts.id = account_consents.account_id WHERE accounts.email = $1`,
    [account.email],
  );
  expect(consents).toEqual(expect.arrayContaining(documents));
  expect(consents).toHaveLength(2);
  const expired = await mailLink(page, account.email, verifyURL);
  await signIn(page, locale, account);
  await expect(page).toHaveURL(`/${locale}/login`);
  await expect(page.locator('main [role=alert]')).toHaveText(t(
    'The request could not be verified. Check your details and try again.',
    'Nie udało się zweryfikować żądania. Sprawdź dane i spróbuj ponownie.',
  ));

  const token = new URL(expired).searchParams.get('token');
  const adjusted = await identityQuery(
    `UPDATE account_email_tokens SET expires_at = now() - interval '1 second',
       updated_at = now() - interval '61 seconds' WHERE token_hash = $1 AND purpose = 'verify_email'`,
    [createHash('sha256').update(token).digest()],
  );
  expect(adjusted.rowCount).toBe(1);
  await waitForIdentityBudget(['identity-verification-source'], 5);
  const invalid = t(
    'This link is invalid or has expired. Request a new verification e-mail.',
    'Ten link jest nieprawidłowy lub wygasł. Poproś o nową wiadomość weryfikacyjną.',
  );
  expect((await page.goto(expired)).status()).toBe(422);
  await expect(page.locator('main [role=status]')).toHaveText(invalid);
  await expect(page.locator('main form')).toHaveCount(0);
  await expect(page.locator('main')).toContainText(t(
    'Sign in to request a new verification e-mail.',
    'Zaloguj się, aby poprosić o nową wiadomość weryfikacyjną.',
  ));
  const resend = t('Resend verification e-mail', 'Wyślij ponownie wiadomość weryfikacyjną');
  await page.locator('main').getByRole('link', { name: resend, exact: true }).click();
  await localizedPage(page, locale, resend);
  await waitForIdentityBudget(['identity-registration'], 4);
  await submit(page, 'main form', { email: account.email });
  await expect(page.locator('main [role=status]')).toHaveText(inbox);
  const replacement = await mailLink(page, account.email, new RegExp(
    verifyURL.source.replace('[a-f0-9]{64}', `(?!${token})[a-f0-9]{64}`),
  ));
  expect(replacement !== expired).toBe(true);
  expect((await page.goto(replacement)).status()).toBe(200);
  await expect(page.getByRole('button', {
    name: t('Verify e-mail address', 'Potwierdź adres e-mail'),
  })).toBeVisible();
  await submit(page);
  await expect(page.locator('main [role=status]')).toHaveText(t(
    'Your e-mail has been verified.', 'Twój adres e-mail został zweryfikowany.',
  ));
  expect((await page.goto(replacement)).status()).toBe(422);
  await expect(page.locator('main [role=status]')).toHaveText(invalid);
  await page.locator('main').getByRole('link', { name: t('Sign in', 'Zaloguj się'), exact: true }).click();
  await expect(page).toHaveURL(`/${locale}/login`);
  const replay = await page.request.post(replacement, {
    headers: { origin: new URL(page.url()).origin },
    form: { _csrf: await page.locator('main [name=_csrf]').inputValue() },
  });
  expect(replay.status()).toBe(422);
  expect((await replay.text()).includes(invalid)).toBe(true);
  await signIn(page, locale, account);
  await expect(page).toHaveURL(`/${locale}/`);
  await page.goto(`/${locale}/register`);
  await expect(page).toHaveURL(/\/account(?:\/|$)/);
  await waitForIdentityBudget(['identity-registration'], 4);
  await page.goto(`/${locale}/verify-email/resend`);
  await expect(page.locator('main [name=email]')).toHaveCount(0);
  await submit(page, 'main form[action$="/verify-email/resend"]');
  await expect(page.locator('main [role=status]')).toHaveText(inbox);

  await logout(page, locale);
  await waitForIdentityBudget(registrationPolicies, 4);
  await page.goto(`/${locale}/register`);
  await submit(page, 'main form', {
    ...account, password_confirmation: account.password, terms: true, privacy: true,
  });
  await expect(page.locator('main [role=status]')).toHaveText(inbox, { timeout: 90_000 });
  await mailLink(page, account.email, new RegExp(`https://localhost:18443/${locale}/login`));
  const { rows: existing } = await identityQuery(
    'SELECT status, email_verified_at IS NOT NULL AS verified FROM accounts WHERE email = $1',
    [account.email],
  );
  expect(existing).toEqual([{ status: 'active', verified: true }]);
});
