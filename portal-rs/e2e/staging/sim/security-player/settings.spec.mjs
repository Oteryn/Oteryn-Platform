import { devices } from '@playwright/test';
import {
  authenticated, expect, http, login, mail, mainForm, newPassword, open, origin, postForm,
  submit, sudo, test, tr,
} from './helpers.mjs';

test('signed-out visitor is sent to the localized login for account settings', async ({ page, staging }, info) => {
  const lang = info.project.metadata.locale;
  for (const route of ['account/privacy', 'account/sessions', 'account/security/wizard',
    'account/privacy/export', 'account/privacy/delete', 'account/security/email', 'account/recovery-key']) {
    const response = await http(staging.api, `/${lang}/${route}`);
    expect(response.status(), route).toBe(303);
    expect(response.headers().location, route).toBe(`/${lang}/login`);
    await response.dispose();
  }
  await open(page, `/${lang}/account/privacy`);
  await expect(page).toHaveURL(`${origin}/${lang}/login`);
  await expect(page.locator('main h1')).toHaveText(tr(lang, 'Zaloguj się', 'Sign in'));
});

test('a passkey can be registered and can sign the player in again', async ({ page, player }, info) => {
  const lang = info.project.metadata.locale;
  const route = `/${lang}/account/passkeys`;
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('WebAuthn.enable');
  const { authenticatorId } = await cdp.send('WebAuthn.addVirtualAuthenticator', {
    options: {
      protocol: 'ctap2', transport: 'internal', hasResidentKey: true,
      hasUserVerification: true, isUserVerified: true, automaticPresenceSimulation: true,
    },
  });
  const post = path => page.waitForResponse(response =>
    new URL(response.url()).pathname === route + path && response.request().method() === 'POST',
  { timeout: 20_000 });

  try {
    await open(page, route);
    const register = page.locator('[data-passkey=register]');
    const password = page.locator('#passkey-password');
    const controls = page.locator('#passkeys');
    const csrf = await controls.getAttribute('data-csrf');
    const malformed = await http(page.request, route + '/register/start', {
      method: 'POST', headers: { origin, 'x-csrf-token': csrf }, data: { password: 123 },
    });
    expect(malformed.status()).toBe(403);
    await password.fill(newPassword());
    const [refused] = await Promise.all([post('/register/start'), register.click()]);
    expect(refused.status()).toBe(403);
    await expect(page.locator('#passkey-status')).toHaveText(tr(lang,
      'Nie udało się wykonać żądania. Odśwież stronę i spróbuj ponownie.',
      'The request could not be completed. Reload the page and try again.'));

    await password.fill(player.password);
    const [started, finished] = await Promise.all([
      post('/register/start'), post('/register/finish'), register.click(),
    ]);
    expect(started.status()).toBe(200);
    expect(finished.status()).toBe(200);
    const authenticate = page.locator('[data-passkey=authenticate]');
    await expect(authenticate).toBeVisible();
    const [signedIn] = await Promise.all([post('/authenticate/finish'), authenticate.click()]);
    expect(signedIn.status()).toBe(200);
    await expect(page.locator('[data-remove]')).toHaveCount(1);
    await page.reload();
    await expect(page.locator('[data-remove]')).toHaveCount(1);
    await authenticated(page, lang);
  } finally {
    await cdp.send('WebAuthn.removeVirtualAuthenticator', { authenticatorId });
    await cdp.detach();
  }
});

test('the player signs out a second browser session and sees the change in history', async ({ page, player, staging }, info) => {
  const lang = info.project.metadata.locale;
  const route = `/${lang}/account/sessions`;
  const since = Date.now() - 2000;
  const other = await staging.openPage({ userAgent: devices['Pixel 7'].userAgent });
  await login(other, lang, player);
  await authenticated(other, lang);
  await open(page, route);
  const entries = page.locator('main li').filter({ has: page.locator('form') });
  await expect(entries).toHaveCount(2);
  const current = tr(lang, 'Ta sesja', 'This session');
  await expect(entries.getByText(current, { exact: true })).toHaveCount(1);
  const target = entries.filter({ hasNotText: current }).locator('form');
  const action = await target.getAttribute('action');
  expect((await postForm(target, {}, route + '/not-a-session')).status()).toBe(400);
  expect((await submit(target)).status()).toBe(302);
  await expect(page.locator('main [role=status]')).toHaveText(tr(lang,
    'Sesje zostały wylogowane.', 'Sessions signed out.'));
  await expect(entries).toHaveCount(1);
  await expect(page.getByRole('heading', { name: tr(lang, 'Wylogowano sesję', 'Session signed out') })).toBeVisible();
  await open(other, `/${lang}/account/privacy`);
  await expect(other).toHaveURL(`${origin}/${lang}/login`);

  expect((await postForm(entries.locator('form'), {}, action)).status()).toBe(302);
  await page.reload();
  await expect(page.locator('main [role=status]')).toHaveText(tr(lang,
    'Wybrana sesja jest nieaktywna lub niedostępna.', 'The selected session is inactive or unavailable.'));
  await expect(entries.getByText(current, { exact: true })).toBeVisible();
  await mail(page.request, player.email, since, tr(lang, 'Nowe urządzenie', 'New device'),
    lang === 'pl' ? /Logowanie z urządzenia:.*Android/ : /Device login:.*Android/);
});

test('the security wizard reports verified and incomplete protections', async ({ page, player, staging }, info) => {
  const lang = info.project.metadata.locale;
  const route = `/${lang}/account/security/wizard`;
  // Request the anonymous state before comparing it with the player's view.
  const anonymous = await http(staging.api, route);
  expect(anonymous.status()).toBe(303);
  await authenticated(page, lang);
  expect((await http(page.request, '/xx/account/security/wizard')).status()).toBe(404);
  await open(page, route);
  await expect(page.locator('main')).toContainText(tr(lang, 'Potwierdzone zabezpieczenia: 1/4', 'Verified protections: 1/4'));
  await expect(page.locator('main')).toContainText(tr(lang,
    'Potwierdź adres e-mail: Ukończono', 'Confirm your email address: Completed'));
  await expect(page.locator('main')).toContainText(tr(lang,
    'Włącz uwierzytelnianie dwuskładnikowe: Wymagane działanie', 'Enable two-factor authentication: Action needed'));
  const manage = page.getByRole('link', {
    name: tr(lang, 'Otwórz ustawienia bezpieczeństwa', 'Open security settings'),
  });
  await manage.click();
  await expect(page).toHaveURL(/\/password\/change$/);
  await page.goBack();
  await expect(page.locator('main')).toContainText(tr(lang, 'Potwierdzone zabezpieczenia: 1/4', 'Verified protections: 1/4'));
});

test('privacy validates preferences and persists the selected language and visibility', async ({ page, player }, info) => {
  const lang = info.project.metadata.locale;
  const other = tr(lang, 'en', 'pl');
  await authenticated(page, lang);
  const form = mainForm(page);
  const fields = {
    public_account_association: true, public_status_visible: true, locale: other,
  };
  const invalid = await postForm(form, { public_status_visible: 'invalid' });
  expect(invalid.status()).toBe(422);
  expect((await invalid.text()).includes(tr(lang,
    'Sprawdź ustawienia prywatności i wybierz język polski lub angielski.',
    'Check the privacy settings and select Polish or English.'))).toBe(true);
  expect((await submit(form, fields)).status()).toBe(303);
  await expect(page).toHaveURL(`${origin}/${other}/account/privacy`);
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('lang', other);
  await expect(form.locator('[name=locale]')).toHaveValue(other);
  for (const name of ['public_account_association', 'public_status_visible']) {
    await expect(form.locator(`[name=${name}]`)).toBeChecked();
  }
});

test('personal data export requires re-authentication and enforces repeat limits', async ({ page, player }, info) => {
  const lang = info.project.metadata.locale;
  const route = `/${lang}/account/privacy/export`;
  expect((await http(page.request, route)).status()).toBe(403);
  await open(page, `/${lang}/account/sudo`);
  expect((await submit(mainForm(page), { current_password: newPassword() })).status()).toBe(302);
  await expect(page.locator('main [role=alert]')).toBeVisible();
  expect((await http(page.request, route)).status()).toBe(403);
  await sudo(page, lang, player.password);
  await open(page, route);
  await expect(page.locator('main h1')).toHaveText(tr(lang, 'Pobierz dane konta', 'Download account data'));
  const form = mainForm(page);
  const malformed = await postForm(form, { _csrf: 'invalid' });
  expect(malformed.status()).toBe(403);
  for (let count = 0; count < 3; count++) {
    const result = await postForm(form);
    expect(result.status()).toBe(200);
    expect(result.headers()['content-disposition']).toBe('attachment; filename="account-data.json"');
    expect(result.headers()['cache-control']).toContain('no-store');
    expect((await result.json()).account.email === player.email, 'export is for the signed-in player').toBe(true);
    await result.dispose();
  }
  const throttled = await postForm(form);
  expect(throttled.status()).toBe(429);
  expect(Number(throttled.headers()['retry-after'])).toBeGreaterThan(0);
  expect((await throttled.text()).includes(tr(lang, 'Zbyt wiele żądań', 'Too many requests'))).toBe(true);
});

test('account closure validates confirmation and signs out the player when scheduled', async ({ page, player, staging }, info) => {
  const lang = info.project.metadata.locale;
  const route = `/${lang}/account/privacy/delete`;
  const closure = await http(page.request, route);
  if (closure.status() === 404) {
    // The handler suppresses the form when staging has no Game closure authority.
    await open(page, route, 404);
    await expect(mainForm(page).locator('[name=confirmation]')).toHaveCount(0);
    info.annotations.push({ type: 'staging limitation', description: 'Game-backed account closure is disabled' });
    return;
  }
  expect(closure.status()).toBe(200);
  await open(page, route);
  const form = mainForm(page);
  const correct = { current_password: player.password, confirmation: 'TERMINATE' };
  expect((await postForm(form, { ...correct, confirmation: 'CLOSE' })).status()).toBe(422);
  expect((await postForm(form, correct)).status()).toBe(403);
  await sudo(page, lang, player.password);
  await open(page, route);
  await expect(form.locator('[name=confirmation]')).toBeVisible();
  const other = await staging.openPage();
  await login(other, lang, player);
  await authenticated(other, lang);
  expect((await submit(form, correct)).status()).toBe(303);
  await expect(page).toHaveURL(`${origin}/${lang}/login`);
  await open(other, `/${lang}/account/privacy`);
  await expect(other).toHaveURL(`${origin}/${lang}/login`);

  // During the documented grace period the player can still sign in to cancel.
  await login(page, lang, player);
  await open(page, route);
  await expect(page.locator('main [role=status]')).toHaveText(tr(lang,
    'Usunięcie konta jest zaplanowane. Możesz je anulować przed zakończeniem.',
    'Deletion is scheduled. You can cancel before finalization.'));
});
