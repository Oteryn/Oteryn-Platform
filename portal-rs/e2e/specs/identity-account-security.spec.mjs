import { test, expect } from '../fixtures.mjs';
import { mailLink, password, seedPasswordRange, submit } from '../helpers.mjs';
import {
  identityEmail, identityQuery, localized, localizedPage, postWithPageCsrf, seedBreachedPassword, seedPlayer, signIn,
} from '../identity-helpers.mjs';

test('sudo and password change reject wrong passwords, and change rejects mismatched fields', async ({ page }, info) => {
  test.setTimeout(240_000);
  const locale = info.project.metadata.locale;
  const t = (en, pl) => localized(locale, en, pl);
  const account = await seedPlayer(locale);
  const next = password();
  await seedPasswordRange(next);
  const breached = await seedBreachedPassword();
  const path = `/${locale}/password/change`;
  const sudo = `/${locale}/account/sudo`;
  const refused = t(
    'Check your current password and the new password fields.',
    'Sprawdź obecne hasło oraz pola nowego hasła.',
  );
  await page.goto(path);
  await expect(page).toHaveURL(/\/login$/);
  const values = { current_password: account.password, password: next, password_confirmation: next };
  expect((await postWithPageCsrf(page, path, values)).status()).toBe(401);
  expect((await postWithPageCsrf(page, sudo, { current_password: account.password })).status()).toBe(401);
  await signIn(page, locale, account);
  await page.goto(path);
  await localizedPage(page, locale, t('Change password', 'Zmień hasło'));
  await submit(page, 'main form', { ...values, current_password: password() });
  await expect(page.locator('main [role=alert]')).toHaveText(refused);

  await page.goto(sudo);
  await localizedPage(page, locale, t('Confirm your password', 'Potwierdź hasło'));
  await submit(page, 'main form', { current_password: password() });
  await expect(page.locator('main [role=alert]')).toHaveText(refused);
  await submit(page, 'main form', { current_password: account.password });
  await expect(page).not.toHaveURL(sudo);
  await page.goto(path);
  await submit(page, 'main form', { ...values, password_confirmation: password() });
  await expect(page.locator('main [role=alert]')).toHaveText(refused);
  await submit(page, 'main form', { ...values, password: breached, password_confirmation: breached });
  await expect(page.locator('main [role=alert]')).toHaveText(t(
    'This new password appears in known data breaches. Choose a different password.',
    'To nowe hasło występuje w znanych wyciekach danych. Wybierz inne hasło.',
  ));
  await submit(page, 'main form', values);
  await expect(page).toHaveURL(/\/login$/);
  await signIn(page, locale, account);
  await expect(page).toHaveURL(`/${locale}/login`);
  await signIn(page, locale, { ...account, password: next });
  await expect(page).toHaveURL(`/${locale}/`);
});

test('web sessions list the current session, revoke one, revoke others and revoke all', async ({ page, browser }, info) => {
  test.setTimeout(300_000);
  const locale = info.project.metadata.locale;
  const t = (en, pl) => localized(locale, en, pl);
  const account = await seedPlayer(locale);
  const path = `/${locale}/account/sessions`;
  const loginPath = `/${locale}/login`;
  const one = 'main form[action]:not([action$="/others"]):not([action$="/all"])';
  const { baseURL, viewport } = info.project.use;
  const other = await browser.newContext({ baseURL, viewport, locale, ignoreHTTPSErrors: true });
  try {
    const screen = await other.newPage();
    await page.goto(path);
    await expect(page).toHaveURL(loginPath);
    expect((await postWithPageCsrf(page, `${path}/others`)).status()).toBe(403);
    await signIn(page, locale, account);
    await signIn(screen, locale, account);
    await page.goto(path);
    await localizedPage(page, locale, t('Web sessions', 'Sesje web'));
    const current = t('This session', 'Ta sesja');
    await expect(page.getByText(current, { exact: true })).toBeVisible();
    await expect(page.locator(one)).toHaveCount(2);
    const foreign = page.locator(one).filter({ visible: true });
    const actions = await foreign.evaluateAll((forms, currentText) => forms
      .filter(form => !form.closest('li').textContent.includes(currentText))
      .map(form => form.getAttribute('action')), current);
    expect(actions).toHaveLength(1);
    await submit(page, `main form[action="${actions[0]}"]`);
    await expect(page.locator('main [role=status]')).toHaveText(t('Sessions signed out.', 'Sesje zostały wylogowane.'));
    await expect(page.locator(one)).toHaveCount(1);
    // A stale tab can still submit a revocation for the already removed session.
    const stale = await postWithPageCsrf(page, actions[0]);
    expect(stale.status()).toBe(302);
    expect(stale.headers().location).toBe(path);
    await page.reload();
    await expect(page.locator('main [role=status]')).toHaveText(t(
      'The selected session is inactive or unavailable.',
      'Wybrana sesja jest nieaktywna lub niedostępna.',
    ));
    await expect(page.locator(one)).toHaveCount(1);
    await screen.goto(path);
    await expect(screen).toHaveURL(loginPath);

    await signIn(screen, locale, account);
    await page.reload();
    await expect(page.locator(one)).toHaveCount(2);
    await submit(page, 'main form[action$="/others"]');
    await expect(page.getByRole('heading', { name: t('Other sessions signed out', 'Wylogowano pozostałe sesje') })).toBeVisible();
    await screen.goto(path);
    await expect(screen).toHaveURL(loginPath);

    await submit(page, 'main form[action$="/all"]');
    await expect(page).toHaveURL(loginPath);
    await page.goto(path);
    await expect(page).toHaveURL(loginPath);
  } finally {
    await other.close();
  }
});

test('email change checks recovery proof, confirms a new address, reverts it and rejects link reuse', async ({ page }, info) => {
  test.setTimeout(240_000);
  const locale = info.project.metadata.locale;
  const t = (en, pl) => localized(locale, en, pl);
  const account = await seedPlayer(locale);
  const email = identityEmail();
  const path = `/${locale}/account/security/email`;
  const loginPath = `/${locale}/login`;
  const state = async () => (await identityQuery('SELECT email FROM accounts WHERE id = $1', [account.id])).rows[0].email;
  await page.goto(path);
  await expect(page).toHaveURL(loginPath);
  const anonymous = await postWithPageCsrf(page, path, { email, email_confirmation: email });
  expect(anonymous.status()).toBe(302);
  expect(anonymous.headers().location).toBe(loginPath);
  await signIn(page, locale, account);
  await page.goto(`/${locale}/account/recovery-key`);
  await submit(page, 'main form', { current_password: account.password });
  const key = (await page.locator('.op-code').innerText()).trim();
  await submit(page, 'main form', { current_password: account.password, recovery_key: key });

  await page.goto(path);
  await localizedPage(page, locale, t('Email security', 'Bezpieczeństwo adresu e-mail'));
  const status = page.locator('main [role=status]');
  const refused = t('Request refused or invalid input.', 'Odmowa lub nieprawidłowe dane.');
  const values = { email, email_confirmation: email, current_password: account.password, proof: key, recovery: true };
  await submit(page, 'main form', { ...values, proof: 'invalid' });
  await expect(status).toHaveText(refused);
  expect(await state()).toBe(account.email);
  await submit(page, 'main form', values);
  await expect(status).toHaveText(t(
    'Check both email addresses for confirmation and cancellation links.',
    'Sprawdź oba adresy e-mail, aby potwierdzić lub anulować zmianę.',
  ));
  const confirm = await mailLink(page, email,
    /https:\/\/localhost:18443\/email-change\/confirm\/[a-f0-9]{64}/, 75_000);
  const revert = await mailLink(page, account.email,
    /https:\/\/localhost:18443\/email-change\/revert\/[a-f0-9]{64}/, 75_000);
  expect(await state()).toBe(account.email);
  await page.goto(confirm);
  await submit(page);
  await expect(page).toHaveURL(loginPath);
  expect(await state()).toBe(email);
  await signIn(page, locale, { ...account, email });
  await expect(page).toHaveURL(`/${locale}/`);
  await page.goto(revert);
  await submit(page);
  await expect(page).toHaveURL(loginPath);
  expect(await state()).toBe(account.email);

  for (const link of [confirm, revert]) {
    await page.goto(link);
    await submit(page);
    await expect(status).toHaveText(refused);
    expect(await state()).toBe(account.email);
  }
});
