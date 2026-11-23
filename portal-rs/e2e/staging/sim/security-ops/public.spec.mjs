import { expect, freshPlayer, get, localized, navigate, status, submitFrom, test } from './helpers.mjs';

test('health and readiness remain available without a staff session', async ({ page, request }) => {
  await status(page, await navigate(page, '/healthz'), 200, 'liveness');
  await expect(page.locator('body')).toHaveText('ok');
  await status(page, (await page.reload()).status(), 200, 'liveness after reload');
  expect((await get(request, '/readyz')).status(), 'readiness').toBe(200);
  expect((await get(request, '/healthz', 'OPTIONS')).status(), 'unsupported health method').toBe(405);
  await status(page, await navigate(page, '/healthz/sim-security-ops'), 404, 'unknown health route');
});

test('downloads serve only published artifacts through the read-only origin', async ({ page, request }) => {
  const timestamp = '/updates/stable/metadata/timestamp.json';
  const response = await get(request, timestamp);
  if (response.status() === 404) {
    expect(response.headers()['cache-control'], 'unavailable releases are not cached').toBe('no-store');
    await status(page, await navigate(page, timestamp), 404, 'unavailable stable timestamp');
    expect((await get(request, '/updates/stable/policy-v1.json')).status(), 'stable policy unavailable').toBe(404);
    test.info().annotations.push({ type: 'staging limitation', description: 'Stable downloads unavailable' });
  } else {
    expect(response.status(), 'published timestamp').toBe(200);
    expect(response.headers()['content-type']).toBe('application/json');
    expect(response.headers()['cache-control']).toBe('no-store, no-transform');
    const metadata = await response.json();
    expect(metadata.signed._type).toBe('timestamp');
    await status(page, await navigate(page, timestamp), 200, 'published stable timestamp');
  }
  expect((await get(request, timestamp, 'OPTIONS')).status(), 'unsupported release method').toBe(405);
  await status(page, await navigate(page, '/updates/sim-invalid/metadata/timestamp.json'),
    400, 'invalid download channel');
  await status(page, await navigate(page, '/updates/stable/metadata/sim-security-ops-missing.json'),
    404, 'unpublished metadata');
});

test('registration reports mismatched passwords and switches language after navigating back', async ({ page }, info) => {
  const lang = info.project.metadata.language;
  const alt = lang === 'pl' ? 'en' : 'pl';
  const register = `/${lang}/register`;
  await status(page, await navigate(page, register), 200, 'registration form');
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(
    localized(lang, 'Utwórz konto', 'Create an account'));
  const account = freshPlayer();
  const response = await submitFrom(page, 'main form', {
    ...account, password_confirmation: `${account.password}x`, terms: true, privacy: true,
  });
  await status(page, response.status(), 422, 'registration validation');
  await expect(page.locator('#confirmation-error')).toHaveText(
    localized(lang, 'Hasła muszą być takie same.', 'Passwords must match.'));
  await expect(page.locator('main input[name="email"]')).toHaveValue(account.email);
  await expect(page.locator('main input[name="password"]')).toHaveValue('');
  await expect(page).toHaveURL(register);
  await status(page, await navigate(page, register), 200, 'reopened registration form');
  const switchLanguage = page.locator(`footer a[hreflang="${alt}"]`);
  await expect(switchLanguage).toHaveAttribute('href', `/${alt}/register`);
  await switchLanguage.click();
  await expect(page).toHaveURL(`/${alt}/register`);
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(
    localized(alt, 'Utwórz konto', 'Create an account'));
  await expect(page.locator('html')).toHaveAttribute('lang', alt);
  await page.goBack({ waitUntil: 'domcontentloaded' });
  await expect(page).toHaveURL(register);
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(
    localized(lang, 'Utwórz konto', 'Create an account'));
});

test('signed-out sessions redirect and invalid verification explains recovery', async ({ page, request }, info) => {
  const lang = info.project.metadata.language;
  const sessions = `/${lang}/account/sessions`;
  const response = await get(request, sessions);
  expect(response.status(), 'signed-out sessions redirect').toBe(303);
  expect(response.headers().location).toBe(`/${lang}/login`);
  await status(page, await navigate(page, sessions), 200, 'redirected sign-in page');
  await expect(page).toHaveURL(`/${lang}/login`);
  await expect(page.locator('main form[action$="/login"] input[type="password"]')).toBeVisible();
  await status(page, await navigate(page, `/${lang}/verify-email?token=invalid`), 422, 'invalid verification link');
  await expect(page.locator('main [role="status"]')).toHaveText(localized(lang,
    'Ten link jest nieprawidłowy lub wygasł. Poproś o nową wiadomość weryfikacyjną.',
    'This link is invalid or has expired. Request a new verification e-mail.'));
  const resend = `/${lang}/verify-email/resend`;
  await page.locator(`main a[href="${resend}"]`).click();
  await expect(page).toHaveURL(resend);
  await expect(page.locator('main form input[type="email"]')).toBeVisible();
});

test('password recovery refuses a missing link and a refresh clears the error', async ({ page }, info) => {
  const lang = info.project.metadata.language;
  const reset = `/${lang}/reset-password`;
  const forgot = `/${lang}/forgot-password`;
  const account = freshPlayer();
  await status(page, await navigate(page, forgot), 200, 'request password recovery form');
  await status(page, (await submitFrom(page, 'main form', { email: account.email })).status(),
    302, 'password recovery request redirect');
  await expect(page).toHaveURL(forgot);
  await expect(page.locator('main [role="status"]')).toHaveText(localized(lang,
    'Jeśli konto z tym adresem istnieje, wysłaliśmy link do zresetowania hasła.',
    'If an account exists for that email, a password reset link has been sent.'));
  await status(page, await navigate(page, reset), 200, 'password reset form');
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(
    localized(lang, 'Zresetuj hasło', 'Reset password'));
  const response = await submitFrom(page, 'main form', {
    ...account, password_confirmation: account.password,
  });
  await status(page, response.status(), 302, 'password reset refusal redirects');
  await expect(page.locator('main [role="alert"]')).toHaveText(localized(lang,
    'Link jest nieprawidłowy lub wygasł albo hasła nie są identyczne. W razie potrzeby poproś o nowy link.',
    'This password reset link is invalid or expired, or the passwords do not match. Request a new link if needed.'));
  await expect(page).toHaveURL(reset);
  await status(page, (await page.reload()).status(), 200, 'password reset reload');
  await expect(page.locator('main [role="alert"]')).toHaveCount(0);
  await page.locator(`main a[href="/${lang}/login"]`).click();
  await expect(page).toHaveURL(`/${lang}/login`);
});

test('signed-out visitors cannot open staff panels', async ({ page, request }, info) => {
  const lang = info.project.metadata.language;
  for (const path of ['/admin/roles', '/admin/players', '/admin/console/audit', '/admin/world',
    '/admin/moderation/enforcement']) {
    const response = await get(request, path);
    if (response.status() === 404) {
      await status(page, await navigate(page, path), 404, path);
      test.info().annotations.push({ type: 'staging limitation', description: `${path} unavailable` });
    } else {
      expect(response.status(), path).toBe(303);
      expect(response.headers().location).toBe('/login');
    }
  }
  await status(page, await navigate(page, `/${lang}/login`), 200, 'public sign-in remains accessible');
  await expect(page.locator('main form[action$="/login"] input[type="email"]')).toBeVisible();
});
