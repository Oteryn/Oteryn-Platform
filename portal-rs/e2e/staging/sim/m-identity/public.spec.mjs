import {
  test, expect, http, nav, open, origin, player, postForm, publicScreenshot, registrationChallenge,
  submit, submitWithBudget,
} from './helpers.mjs';

const strings = {
  en: {
    login: 'Sign in',
    loginError: 'The request could not be verified. Check your details and try again.',
    register: 'Create an account',
    invalidEmail: 'Enter a valid e-mail address.',
    invalidPassword: 'Use at least 10 characters and at most 256 bytes.',
    invalidConfirmation: 'Passwords must match.',
    inbox: 'Check your inbox for an e-mail with the next steps.',
    invalidVerification: 'This link is invalid or has expired. Request a new verification e-mail.',
    resend: 'Resend verification e-mail',
    reset: 'Reset password',
    resetNotice: 'If an account exists for that email, a password reset link has been sent.',
    resetError: 'This password reset link is invalid or expired, or the passwords do not match. Request a new link if needed.',
    unsubscribe: 'Unsubscribe',
    recovery: 'Account recovery key',
    passkeys: 'Passkeys',
  },
  pl: {
    login: 'Zaloguj się',
    loginError: 'Nie udało się zweryfikować żądania. Sprawdź dane i spróbuj ponownie.',
    register: 'Utwórz konto',
    invalidEmail: 'Podaj poprawny adres e-mail.',
    invalidPassword: 'Użyj co najmniej 10 znaków i maksymalnie 256 bajtów.',
    invalidConfirmation: 'Hasła muszą być takie same.',
    inbox: 'Sprawdź skrzynkę odbiorczą — wysłaliśmy wiadomość z dalszymi krokami.',
    invalidVerification: 'Ten link jest nieprawidłowy lub wygasł. Poproś o nową wiadomość weryfikacyjną.',
    resend: 'Wyślij ponownie wiadomość weryfikacyjną',
    reset: 'Zresetuj hasło',
    resetNotice: 'Jeśli konto z tym adresem istnieje, wysłaliśmy link do zresetowania hasła.',
    resetError: 'Link jest nieprawidłowy lub wygasł albo hasła nie są identyczne. W razie potrzeby poproś o nowy link.',
    unsubscribe: 'Wypisz się',
    recovery: 'Klucz odzyskiwania konta',
    passkeys: 'Klucze dostępu',
  },
};

test('Public sign-in and language navigation', async ({ page }, info) => {
  const { locale } = info.project.metadata;
  const copy = strings[locale];
  await page.goto(`/${locale}/`, { waitUntil: 'domcontentloaded' });
  await nav(page, `/${locale}/login`);
  await expect(page).toHaveURL(`/${locale}/login`);
  await expect(page.locator('main h1')).toHaveText(copy.login);
  const form = page.locator('main form');
  await submit(form);
  await expect(form.locator('input[name=email]:invalid')).toBeVisible();
  await submitWithBudget(form, { email: player().email, password: 'Incorrect test password' });
  await expect(page.locator('main [role=alert]')).toHaveText(copy.loginError);
  await page.reload({ waitUntil: 'domcontentloaded' });
  await expect(page.locator('main [role=alert]')).toHaveCount(0);
  const other = locale === 'pl' ? 'en' : 'pl';
  const localeLink = page.locator(`a[href="/${other}/login"]`).first();
  await expect(localeLink).toBeVisible();
  await localeLink.click();
  await expect(page.locator('main h1')).toHaveText(strings[other].login);
  await page.goBack({ waitUntil: 'domcontentloaded' });
  await expect(page.locator('main h1')).toHaveText(copy.login);
  const signedOut = await postForm(page, `/${locale}/logout`, {});
  expect(signedOut.status()).toBe(302);
  expect(signedOut.headers().location).toBe(`/${locale}/login`);
});

test('Registration and inbox handoff', async ({ page }, info) => {
  const { locale } = info.project.metadata;
  const copy = strings[locale];
  const account = player();
  await page.goto(`/${locale}/login`, { waitUntil: 'domcontentloaded' });
  await page.locator(`main a[href="/${locale}/register"]`).click();
  await expect(page.locator('main h1')).toHaveText(copy.register);
  const form = page.locator('main form');
  await submit(form);
  await expect(form.locator('input[name=email]:invalid')).toBeVisible();
  await form.locator('input[name=email]').fill(account.email);
  await form.locator('input[name=password]').fill(account.password);
  await form.locator('input[name=password_confirmation]').fill(account.password);
  await form.locator('input[name=terms]').check();
  // The browser blocks the form until both documents are accepted.
  await submit(form);
  await expect(form.locator('input[name=privacy]:invalid')).toBeVisible();
  const registration = await submitWithBudget(form, {
    ...account, password_confirmation: account.password, terms: true, privacy: true,
  }, true);
  const challenge = registrationChallenge(page);
  test.skip(registration.status() === 429 || challenge?.status === 429,
    'Shared registration budget remained exhausted after two waits.');
  if (registration.status() !== 303 || challenge?.status !== 200) {
    await page.locator('main').getByText(String(registration.status()), { exact: true })
      .waitFor({ state: 'visible', timeout: 5_000 }).catch(() => {});
    await publicScreenshot(page, info, 'registration-result').catch(() => {
      info.annotations.push({ type: 'evidence', description: 'Browser screenshot could not be saved.' });
    });
  }
  expect.soft(registration.status(), 'registration form response').toBe(303);
  expect(challenge?.status, 'registration challenge').toBe(200);
  expect(challenge.bits, 'registration challenge difficulty').toBe(16);
  if (registration.status() !== 303) return;
  await expect(page).toHaveURL(`/${locale}/register/check-inbox`, { timeout: 90_000 });
  await expect(page.locator('main [role=status]')).toHaveText(copy.inbox);
  await page.reload({ waitUntil: 'domcontentloaded' });
  await expect(page.locator('main [role=status]')).toHaveText(copy.inbox);
  const onlooker = await page.context().newPage();
  try {
    await onlooker.goto(`/${locale}/login`, { waitUntil: 'domcontentloaded' });
    await submitWithBudget(onlooker.locator('main form'), account);
    await expect(onlooker.locator('main [role=alert]')).toHaveText(copy.loginError);
  } finally {
    await onlooker.close();
  }
});

test('Registration returns localized field errors', async ({ page }, info) => {
  const { locale } = info.project.metadata;
  const copy = strings[locale];
  const path = `/${locale}/register`;
  await open(page, path);
  const response = await postForm(page, path, {
    email: '<sample>',
    password: 'x'.repeat(257),
    password_confirmation: 'different',
  });
  test.skip(response.status() === 429, 'Shared registration budget remained exhausted after two waits.');
  expect(response.status()).toBe(422);
  const result = await response.text();
  for (const message of [copy.invalidEmail, copy.invalidPassword, copy.invalidConfirmation]) {
    expect(result.includes(message), message).toBe(true);
  }
  await page.reload({ waitUntil: 'domcontentloaded' });
  await expect(page.locator('main [name=email]')).toHaveValue('');
});

test('Verification links and resend form', async ({ page }, info) => {
  const { locale } = info.project.metadata;
  const copy = strings[locale];
  const target = `/${locale}/verify-email`;
  const response = await open(page, target);
  expect(response.status()).toBe(422);
  await expect(page.locator('main [role=status]')).toHaveText(copy.invalidVerification);
  await page.getByRole('link', { name: copy.resend, exact: true }).click();
  await expect(page.locator('main h1')).toHaveText(copy.resend);
  await submit(page.locator('main form'));
  await expect(page.locator('main input[name=email]:invalid')).toBeVisible();
  const invalid = await postForm(page, `${target}?token=invalid`, {});
  expect(invalid.status()).toBe(422);
  expect((await invalid.text()).includes(copy.invalidVerification)).toBe(true);
  await submitWithBudget(page.locator('main form'), { email: player().email });
  await expect(page).toHaveURL(`/${locale}/register/check-inbox`);
  await expect(page.locator('main [role=status]')).toHaveText(copy.inbox);
});

test('Password reset privacy and invalid link', async ({ page }, info) => {
  const { locale } = info.project.metadata;
  const copy = strings[locale];
  const path = `/${locale}/forgot-password`;
  await page.goto(`/${locale}/login`, { waitUntil: 'domcontentloaded' });
  const link = page.locator(`main a[href="${path}"]`);
  if (await link.count()) await link.click();
  else await page.goto(path, { waitUntil: 'domcontentloaded' });
  await expect(page.locator('main h1')).toHaveText(copy.reset);
  const email = player().email;
  const requested = await submitWithBudget(page.locator('main form'), { email });
  expect(requested.status()).toBe(302);
  await expect(page.locator('main [role=status]')).toHaveText(copy.resetNotice);
  await page.reload({ waitUntil: 'domcontentloaded' });
  await expect(page.locator('main [role=status]')).toHaveCount(0);
  const reset = `/${locale}/reset-password`;
  const invalid = await page.goto(`${reset}/invalid`, { waitUntil: 'domcontentloaded' });
  expect(invalid.status()).toBe(200);
  await expect(page.locator('main [name=token]')).toHaveValue('invalid');
  await submitWithBudget(page.locator('main form'), {
    email, password: 'Sim!InvalidLink625', password_confirmation: 'Sim!InvalidLink625',
  });
  await expect(page).toHaveURL(reset);
  await expect(page.locator('main [role=alert]')).toHaveText(copy.resetError);
});

test('Signed-out security and notification pages', async ({ page, request }, info) => {
  const { locale } = info.project.metadata;
  const copy = strings[locale];
  const routes = [
    ['account/security', 303], ['password/change', 302, '/login'], ['account/sudo', 303],
    ['account/security/email', 303], ['account/security/wizard', 303], ['account/sessions', 303],
    ['account/consents', 303], ['account/notifications', 303], ['account/privacy', 303],
    ['account/privacy/delete', 303], ['account/privacy/export', 303],
    ['account/recovery-key', 303], ['account/recovery-key/confirm', 303],
    ['mfa', 302], ['mfa/challenge', 302],
  ];
  for (const [path, status, destination = `/${locale}/login`] of routes) {
    const response = await http(request, 'GET', `/${locale}/${path}`);
    expect.soft(response.status(), path).toBe(status);
    expect.soft(response.headers().location, path).toBe(destination);
  }
  for (const [path, title] of [
    ['account/passkeys', copy.passkeys], ['recovery-key', copy.recovery], ['unsubscribe', copy.unsubscribe],
  ]) {
    const response = await page.goto(`/${locale}/${path}`, { waitUntil: 'domcontentloaded' });
    expect.soft(response.status(), path).toBe(200);
    await expect.soft(page.locator('main h1'), path).toHaveText(title);
  }
  const unsubscribe = `/${locale}/unsubscribe`;
  const malformed = await http(request, 'GET', `${unsubscribe}?token=${'x'.repeat(513)}`);
  expect(malformed.status()).toBe(422);
  const refusal = await http(request, 'POST', unsubscribe, { form: { 'List-Unsubscribe': 'One-Click' } });
  expect(refusal.status()).toBe(422);
  if (info.errors.length) await publicScreenshot(page, info, 'signed-out-pages');
});

test('Signed-out form submissions are refused', async ({ request }, info) => {
  const { locale } = info.project.metadata;
  const forms = [
    ['POST', 'login'], ['POST', 'mfa/login'], ['POST', 'logout'],
    ['POST', 'mfa/challenge'], ['POST', 'mfa/email'], ['POST', 'mfa/enroll'],
    ['POST', 'mfa/confirm'], ['POST', 'mfa/devices'],
    ['POST', 'register'], ['POST', 'register/abuse/challenge'],
    ['POST', 'verify-email'], ['POST', 'verify-email/resend'],
    ['POST', 'forgot-password'], ['POST', 'reset-password'],
    ['POST', 'recovery-key'], ['POST', 'account/recovery-key'], ['DELETE', 'account/recovery-key'],
    ['POST', 'account/recovery-key/confirm'], ['POST', 'account/recovery-key/revoke'],
    ['POST', 'password/change'], ['PUT', 'password/change'], ['POST', 'account/sudo'],
    ['POST', 'account/security/email'], ['POST', 'email-change/confirm/invalid'],
    ['POST', 'email-change/revert/invalid'], ['DELETE', 'account/sessions'],
    ['POST', 'account/sessions/others'], ['POST', 'account/sessions/all'],
    ['POST', 'account/sessions/invalid'], ['DELETE', 'account/sessions/invalid'],
    ['POST', 'account/privacy'], ['PUT', 'account/privacy'], ['POST', 'account/privacy/export'],
    ['POST', 'account/privacy/delete/cancel'], ['POST', 'account/consents'], ['POST', 'account/notifications'],
  ];
  for (const [method, path] of forms) {
    const response = await http(request, method, `/${locale}/${path}`, { form: {}, headers: { origin } });
    expect.soft(response.status(), `${method} ${path}`).toBe(403);
  }
  if (info.project.name === 'pl-1440') {
    const localizedOnly = new Set([
      'account/privacy', 'account/privacy/export', 'account/privacy/delete/cancel',
      'account/consents', 'account/notifications',
    ]);
    for (const [method, path] of forms) {
      if (localizedOnly.has(path)) continue;
      const response = await http(request, method, `/${path}`, { form: {}, headers: { origin } });
      expect.soft(response.status(), `unprefixed ${method} ${path}`).toBe(403);
    }
  }
  for (const suffix of ['register/start', 'register/finish', 'authenticate/start', 'authenticate/finish', 'invalid/remove']) {
    const response = await http(request, 'POST', `/${locale}/account/passkeys/${suffix}`, {
      data: {}, headers: { origin },
    });
    expect.soft(response.status(), suffix).toBe(403);
  }
});

test('Public identity aliases and email-change links', async ({ page, request }, info) => {
  const { locale } = info.project.metadata;
  const copy = strings[locale];
  const reads = [
    ['/register', 200], ['/register/check-inbox', 200], ['/login', 200], ['/mfa/login', 200],
    ['/register/abuse.js', 200], ['/verify-email/resend', 200], ['/reset-password', 200],
    ['/forgot-password', 301, '/pl/forgot-password'], ['/reset-password/invalid', 301, '/pl/forgot-password'],
    ['/recovery-key', 200],
  ];
  if (info.project.name === 'pl-1440') {
    reads.push(
      ['/verify-email', 422], ['/mfa', 302, '/pl/login'], ['/mfa/challenge', 302, '/pl/login'],
      ['/account/recovery-key', 303, '/pl/login'], ['/account/recovery-key/confirm', 303, '/pl/login'],
      ['/password/change', 302, '/login'], ['/account/sudo', 303, '/pl/login'],
      ['/account/security', 303, '/pl/login'], ['/account/security/email', 303, '/pl/login'],
      ['/account/sessions', 303, '/pl/login'],
      ['/email-change/confirm/invalid', 200], ['/email-change/revert/invalid', 200],
    );
  }
  for (const [path, status, redirect] of reads) {
    const response = await http(request, 'GET', path, { headers: { 'accept-language': locale } });
    expect.soft(response.status(), path).toBe(status);
    if (redirect) expect.soft(response.headers().location, path).toBe(redirect);
    if (path.endsWith('.js')) expect.soft(response.headers()['content-type'], path).toContain('text/javascript');
  }
  for (const operation of ['confirm', 'revert']) {
    const path = `/${locale}/email-change/${operation}/invalid`;
    const response = await page.goto(path, { waitUntil: 'domcontentloaded' });
    expect(response.status(), operation).toBe(200);
    await submitWithBudget(page.locator('main form'), {});
    await expect(page).toHaveURL(path);
    await expect(page.locator('main [role=status]')).toHaveText(locale === 'en'
      ? 'Request refused or invalid input.' : 'Odmowa lub nieprawidłowe dane.');
  }
  const viaAlias = await page.goto('/login', { waitUntil: 'domcontentloaded' });
  expect(viaAlias.status()).toBe(200);
  await expect(page.locator('main h1')).toHaveText(copy.login);
});

test('OAuth is unavailable on staging', async ({ request }) => {
  for (const [method, path] of [
    ['GET', '/oauth/authorize'], ['POST', '/oauth/authorize'], ['POST', '/oauth/token'],
  ]) {
    const response = await http(request, method, path, method === 'POST'
      ? { headers: { origin }, form: {} } : {});
    expect(response.status(), `${method} ${path}`).toBe(404);
    expect(await response.text()).toBe('');
  }
});

test('Private identity services are unavailable on the public listener', async ({ request }) => {
  const routes = [
    '/internal/v1/identity/security-revocations/stream/read',
    '/internal/v1/game-auth/account-closure/lifecycle/read',
    '/internal/v1/game-auth/character-commands/pending',
    '/internal/v1/game-auth/character-commands/read',
    '/internal/v1/game-auth/character-commands/receipt',
    '/internal/v1/game-auth/account-closure',
    '/internal/v1/game-auth/account-closure/watermark',
  ];
  for (const path of routes) {
    const response = await http(request, 'POST', path, { data: {} });
    expect.soft(response.status(), path).toBe(404);
  }
});
