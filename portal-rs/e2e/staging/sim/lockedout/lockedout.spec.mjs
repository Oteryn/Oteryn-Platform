import { randomBytes } from 'node:crypto';
import {
  activePlayer, delay, expect, form, identity, mailAvailable, mailLink, messages,
  open, password, register, retrySharedLimit, submit, test, totp, verifyPlayer, waitForRetry,
} from './helpers.mjs';

const copy = {
  pl: {
    resetNotice: 'Jeśli konto z tym adresem istnieje, wysłaliśmy link do zresetowania hasła.',
    resetError: 'Link jest nieprawidłowy lub wygasł albo hasła nie są identyczne. W razie potrzeby poproś o nowy link.',
    resetSubject: 'Zresetuj hasło do Oteryn',
    loginError: 'Nie udało się zweryfikować żądania. Sprawdź dane i spróbuj ponownie.',
    plainLoginError: 'Podane dane logowania są nieprawidłowe.',
    sessions: 'Sesje web',
    limit: 'Zbyt wiele żądań',
    retry: 'Poczekaj przed kolejną próbą.',
    changeError: 'Sprawdź obecne hasło oraz pola nowego hasła.',
    keyActive: 'Klucz odzyskiwania jest aktywny.',
    keyInvalid: 'Sprawdź e-mail, klucz i hasło. Hasła muszą być identyczne i zawierać co najmniej 10 znaków.',
    keyRejected: 'Dane są nieprawidłowe lub ta operacja jest niedostępna.',
    mfaSetup: 'Skonfiguruj uwierzytelnianie dwuskładnikowe',
  },
  en: {
    resetNotice: 'If an account exists for that email, a password reset link has been sent.',
    resetError: 'This password reset link is invalid or expired, or the passwords do not match. Request a new link if needed.',
    resetSubject: 'Reset your Oteryn password',
    loginError: 'The request could not be verified. Check your details and try again.',
    plainLoginError: 'The provided credentials are invalid.',
    sessions: 'Web sessions',
    limit: 'Too many requests',
    retry: 'Please wait before trying again.',
    changeError: 'Check your current password and the new password fields.',
    keyActive: 'Your recovery key is active.',
    keyInvalid: 'Check your email, key and password. Passwords must match and contain at least 10 characters.',
    keyRejected: 'The credentials are invalid or this action is unavailable.',
    mfaSetup: 'Set up two-factor authentication',
  },
};

const alert = page => page.locator('main [role=alert]');
const notice = page => page.locator('main [role=status]');
const resetPattern = /https:\/\/(?:staging\.oteryn\.com|localhost|127\.0\.0\.1)(?::[0-9]+)?\/(?:pl|en)\/reset-password\/[a-f0-9]{64}(?:\?[^\s<>"\x27]*)?/;

test.afterEach(async ({ page }, info) => {
  if (info.status !== 'failed' && info.status !== 'timedOut') return;
  if (!page.url().startsWith('https://staging.oteryn.com/') || page.isClosed()) return;
  // Preserve visible errors while hiding passwords, keys, authenticator seeds and recovery codes.
  try {
    const path = info.outputPath('portal-error.png');
    await page.screenshot({
      path,
      timeout: 5000,
      mask: [page.locator('main input, main textarea, main .op-prose, main .op-code')],
    });
    await info.attach('Redacted portal screenshot', { path, contentType: 'image/png' });
    console.log(`Screenshot: ${path}`);
  } catch {
    // A failed navigation or closed browser might leave no screen to capture.
  }
});

async function requestReset(page, lang, email, measureLimit = false) {
  const attempt = async () => {
    const route = `/${lang}/forgot-password`;
    expect((await open(page, route)).status()).toBe(200);
    return submit(page, form(route), { email });
  };
  return measureLimit ? attempt() : retrySharedLimit(attempt);
}

async function resetReceipt(page, lang, response) {
  expect(response.status()).toBe(302);
  await expect(page).toHaveURL(`/${lang}/forgot-password`);
  await expect(notice(page)).toHaveText(copy[lang].resetNotice);
  return notice(page).innerText();
}

async function emailReset(page, request, lang, email) {
  const since = Date.now() - 5000;
  await resetReceipt(page, lang, await requestReset(page, lang, email));
  return mailLink(request, email, since, resetPattern, copy[lang].resetSubject);
}

async function changeFromLink(page, lang, path, email, next, confirmation = next, options) {
  return retrySharedLimit(async () => {
    expect((await open(page, path)).status(), 'Reset form opens').toBe(200);
    return submit(page, form(`/${lang}/reset-password`), {
      email, password: next, password_confirmation: confirmation,
    }, options);
  });
}

async function expectResetRefused(page, lang, response) {
  expect(response.status()).toBe(302);
  await expect(notice(page)).toHaveCount(0);
  await expect(alert(page)).toHaveText(copy[lang].resetError);
  expect(new URL(page.url()).pathname.startsWith(`/${lang}/reset-password`)).toBe(true);
}

async function login(page, lang, account, email = account.email, measureLimit = false) {
  const attempt = async () => {
    const route = `/${lang}/login`;
    expect((await open(page, route)).status()).toBe(200);
    return submit(page, form(route), { email, password: account.password });
  };
  return measureLimit ? attempt() : retrySharedLimit(attempt);
}

async function expectLoginRefused(page, lang, response) {
  expect(response.status()).toBe(302);
  await expect(page).toHaveURL(`/${lang}/login`);
  // The MFA and password-only routers render different localized errors.
  const error = page.locator('main #login-error');
  if (await error.count()) await expect(error).toHaveText(copy[lang].plainLoginError);
  else await expect(alert(page)).toHaveText(copy[lang].loginError);
}

async function expectSession(page, lang, count) {
  expect((await open(page, `/${lang}/account/sessions`)).status()).toBe(200);
  await expect(page).toHaveURL(`/${lang}/account/sessions`);
  await expect(page.getByRole('heading', { name: copy[lang].sessions })).toBeVisible();
  if (count !== undefined) await expect(page.locator('main li form')).toHaveCount(count);
}

async function expectSignedOut(page, lang) {
  await expect(page).toHaveURL(`/${lang}/login`);
  await expect(page.locator(form(`/${lang}/login`))).toBeVisible();
}

async function expectLimited(page, lang, response) {
  expect(response.status()).toBe(429);
  await expect(page.getByRole('heading', { name: copy[lang].limit })).toBeVisible();
  await expect(page.getByText(copy[lang].retry, { exact: true })).toBeVisible();
}

test('reset request keeps the same localized receipt for known and unknown addresses', async ({ page }, info) => {
  const { lang } = info.project.metadata;
  const unknown = identity().email;
  const route = `/${lang}/forgot-password`;
  expect((await open(page, route)).status()).toBe(200);
  const email = page.locator(`${form(route)} [name=email]`);
  await email.fill('no-at-sign');
  await page.locator(`${form(route)} button`).click();
  expect(await email.evaluate(input => input.validity.typeMismatch)).toBe(true);
  await expect(page).toHaveURL(route);
  await expect(notice(page)).toHaveCount(0);

  const anonymous = await resetReceipt(page, lang, await requestReset(page, lang, unknown));
  const other = lang === 'en' ? 'pl' : 'en';
  const alternate = page.locator(`footer a[hreflang="${other}"]`);
  await expect(alternate).toHaveAttribute('href', `/${other}/forgot-password`);
  await alternate.click();
  await expect(page).toHaveURL(`/${other}/forgot-password`);
  await expect(page.locator('html')).toHaveAttribute('lang', other);
  await page.goBack({ waitUntil: 'domcontentloaded' });
  await expect(page).toHaveURL(route);
  const account = await register(page, lang);
  const known = await resetReceipt(page, lang, await requestReset(page, lang, account.email));
  expect(anonymous).toBe(known);
});

test('unknown reset request delivers no mail', async ({ page, request }, info) => {
  const { lang } = info.project.metadata;
  await mailAvailable(request);
  const unknown = identity().email;
  const since = Date.now() - 5000;
  await resetReceipt(page, lang, await requestReset(page, lang, unknown));
  const until = Date.now() + 60_000;
  do {
    expect((await messages(request, unknown, since)).length, 'Mail for an unregistered test address').toBe(0);
    await delay(Math.min(5000, Math.max(0, until - Date.now())));
  } while (Date.now() < until);
  expect((await messages(request, unknown, since)).length).toBe(0);
});

test('repeated reset requests show the limit and reopen after the window', async ({ page }, info) => {
  const { lang } = info.project.metadata;
  const email = identity().email;
  for (let window = 0; window < 3; window++) {
    await startOfWindow(page);
    let response;
    for (let attempt = 0; attempt < 3; attempt++) {
      response = await requestReset(page, lang, email, true);
      if (response.status() === 429) break;
      await resetReceipt(page, lang, response);
    }
    if (response.status() === 429) {
      // A shared-source limit can fire before this address hits its own limit.
      await waitForRetry(response);
      continue;
    }
    const refused = await requestReset(page, lang, email, true);
    await expectLimited(page, lang, refused);
    await waitForRetry(refused);
    await resetReceipt(page, lang, await requestReset(page, lang, email));
    return;
  }
  test.skip(true, 'Shared staging egress was busy across three reset windows');
});

test('reset submission refuses a link with an invalid token', async ({ page }, info) => {
  const { lang } = info.project.metadata;
  const token = randomBytes(32).toString('hex');
  const response = await changeFromLink(page, lang, `/${lang}/reset-password/${token}`, identity().email, password());
  await expectResetRefused(page, lang, response);
});

test('emailed reset revokes other sessions and refuses mismatched, altered and reused links', async ({ page, request, context, isolatedPage }, info) => {
  const { lang } = info.project.metadata;
  const account = await activePlayer(page, request, lang);
  const client = await context.newPage();
  const reset = await isolatedPage();
  const device = await isolatedPage();
  for (const session of [client, device]) {
    expect((await login(session, lang, account)).status()).toBe(302);
    await expect(session).toHaveURL(`/${lang}/`);
    await expectSession(session, lang);
  }
  await expectSession(client, lang, 2);
  expect(await resetReceiptForAuthenticated(client, lang, account.email)).toBe(302);

  const { url } = await emailReset(reset, request, lang, account.email);
  const next = password();
  const altered = new URL(url);
  altered.pathname = url.pathname.slice(0, -1) + (url.pathname.endsWith('0') ? '1' : '0');
  await expectResetRefused(reset, lang, await changeFromLink(reset, lang, altered.href, account.email, next));
  await expectResetRefused(reset, lang, await changeFromLink(reset, lang, url.href, account.email, next, password()));
  await expectSession(device, lang, 2);
  const changed = await changeFromLink(reset, lang, url.href, account.email, next, next, { double: true });
  expect(changed.status()).toBe(302);
  await expectSignedOut(reset, lang);

  for (const session of [client, device]) {
    await session.reload({ waitUntil: 'domcontentloaded' });
    await expectSignedOut(session, lang);
  }
  await expectResetRefused(reset, lang, await changeFromLink(reset, lang, url.href, account.email, password()));
  await expectLoginRefused(reset, lang, await login(reset, lang, account));
  expect((await login(reset, lang, { ...account, password: next })).status()).toBe(302);
  await expect(reset).toHaveURL(`/${lang}/`);
  await expectSession(reset, lang, 1);
});

async function resetReceiptForAuthenticated(page, lang, email) {
  const response = await requestReset(page, lang, email);
  await expect(page).toHaveURL(`/${lang}/`);
  await expect(notice(page)).toHaveCount(0);
  return response.status();
}

test('a real unused reset link is refused after the advertised hour', async ({ page, request }, info) => {
  test.setTimeout(4_500_000);
  const { lang } = info.project.metadata;
  const account = await activePlayer(page, request, lang);
  const { url, created } = await emailReset(page, request, lang, account.email);
  expect((await open(page, url.href)).status()).toBe(200);
  await expect(page.locator(form(`/${lang}/reset-password`))).toBeVisible();
  await delay(Math.max(0, created + 3_610_000 - Date.now()));
  await expectResetRefused(page, lang, await changeFromLink(page, lang, url.href, account.email, password()));
  expect((await login(page, lang, account)).status()).toBe(302);
  await expect(page).toHaveURL(`/${lang}/`);
  await expectSession(page, lang, 1);
});

async function startOfWindow(page) {
  const response = await open(page, '/readyz');
  expect(response.status()).toBe(200);
  const serverTime = Date.parse(await response.headerValue('date'));
  expect(Number.isFinite(serverTime), 'Staging sends a server date').toBe(true);
  const elapsed = serverTime % 60_000;
  if (elapsed > 10_000) await delay(60_000 - elapsed + 1000);
}

test('repeated incorrect logins on the player account are limited and the window reopens', async ({ page, request }, info) => {
  const { lang } = info.project.metadata;
  const account = await register(page, lang);
  await mailAvailable(request);
  await verifyPlayer(page, request, lang, account);
  const mistaken = { ...account, password: password() };
  for (let window = 0; window < 3; window++) {
    await startOfWindow(page);
    let response;
    for (let attempt = 0; attempt < 5; attempt++) {
      response = await login(page, lang, mistaken, mistaken.email, true);
      if (response.status() === 429) break;
      await expectLoginRefused(page, lang, response);
    }
    if (response.status() === 429) {
      await waitForRetry(response);
      continue;
    }
    const limited = await login(page, lang, mistaken, mistaken.email, true);
    await expectLimited(page, lang, limited);
    await waitForRetry(limited);
    expect((await login(page, lang, account)).status()).toBe(302);
    await expect(page).toHaveURL(`/${lang}/`);
    await expectSession(page, lang, 1);
    return;
  }
  test.skip(true, 'Shared staging egress was busy across three login windows');
});

test('login accepts uppercase and pasted whitespace and refuses a typo', async ({ page, request, isolatedPage }, info) => {
  const { lang } = info.project.metadata;
  const account = await activePlayer(page, request, lang);
  await expectLoginRefused(page, lang, await login(page, lang, account, account.email.replace('@', '-typo@')));
  expect((await login(page, lang, account, account.email.toUpperCase())).status()).toBe(302);
  await expect(page).toHaveURL(`/${lang}/`);
  await expectSession(page, lang, 1);
  const second = await isolatedPage();
  expect((await login(second, lang, account, `  ${account.email.toUpperCase()}  `)).status()).toBe(302);
  await expect(second).toHaveURL(`/${lang}/`);
  await expectSession(second, lang, 2);
});

test('password change signs out a second tab and another browser', async ({ page, request, context, isolatedPage }, info) => {
  const { lang } = info.project.metadata;
  const account = await activePlayer(page, request, lang);
  const tab = await context.newPage();
  const other = await isolatedPage();
  for (const session of [page, other]) {
    expect((await login(session, lang, account)).status()).toBe(302);
    await expect(session).toHaveURL(`/${lang}/`);
  }
  await expectSession(tab, lang, 2);
  const sudo = `/${lang}/account/sudo`;
  expect((await open(page, sudo)).status()).toBe(200);
  expect((await submit(page, form(sudo), { current_password: account.password })).status()).toBe(302);
  await expect(page).toHaveURL(`/${lang}/account/security`);
  const route = `/${lang}/password/change`;
  expect((await open(page, route)).status()).toBe(200);
  const next = password();
  const changes = { current_password: account.password, password: next, password_confirmation: next };
  const rejected = await submit(page, form(route), { ...changes, password_confirmation: password() });
  expect(rejected.status()).toBe(302);
  await expect(page).toHaveURL(route);
  await expect(alert(page)).toHaveText(copy[lang].changeError);
  await expectSession(other, lang, 2);

  expect((await submit(page, form(route), changes)).status()).toBe(302);
  await expect(page).toHaveURL(/\/login$/);
  for (const session of [tab, other]) {
    await session.reload({ waitUntil: 'domcontentloaded' });
    await expectSignedOut(session, lang);
  }
  await expectLoginRefused(tab, lang, await login(tab, lang, account));
  expect((await login(tab, lang, { ...account, password: next })).status()).toBe(302);
  await expect(tab).toHaveURL(`/${lang}/`);
  await expectSession(tab, lang, 1);
});

async function enroll(page, lang, account) {
  const route = `/${lang}/mfa`;
  expect((await open(page, route)).status()).toBe(200);
  await expect(page.getByRole('button', { name: copy[lang].mfaSetup })).toBeVisible();
  const start = await submit(page, form(`${route}/enroll`), { password: account.password });
  expect(start.status()).toBe(200);
  const secret = await page.locator('main .op-prose').innerText();
  const step = Math.floor(Date.now() / 30_000);
  const confirmation = await submit(page, form(`${route}/confirm`), {
    password: account.password, code: totp(secret, step),
  });
  expect(confirmation.status()).toBe(200);
  const codes = page.locator('main textarea[readonly]');
  await expect(codes).toHaveCount(8);
  return { secret, step, recoveryCode: await codes.first().inputValue() };
}

test('lost authenticator recovery key restores access and allows MFA re-enrolment', async ({ page, request, isolatedPage }, info) => {
  const { lang } = info.project.metadata;
  const account = await activePlayer(page, request, lang);
  expect((await login(page, lang, account)).status()).toBe(302);
  await expect(page).toHaveURL(`/${lang}/`);
  const manage = `/${lang}/account/recovery-key`;
  expect((await open(page, manage)).status()).toBe(200);
  expect((await submit(page, form(manage), { current_password: account.password })).status()).toBe(200);
  const recoveryKey = await page.locator('main .op-code').innerText();
  expect(recoveryKey.length, 'A recovery key was shown').toBeGreaterThan(0);
  expect((await submit(page, form(`${manage}/confirm`), {
    current_password: account.password, recovery_key: recoveryKey,
  })).status()).toBe(200);
  await expect(notice(page)).toHaveText(copy[lang].keyActive);
  const oldMfa = await enroll(page, lang, account);

  const device = await isolatedPage();
  const challenge = `/${lang}/mfa/challenge`;
  expect((await login(device, lang, account)).status()).toBe(302);
  await expect(device).toHaveURL(challenge);
  expect((await submit(device, form(challenge), { code: 'wrong-code' })).status()).toBe(302);
  await expect(alert(device)).toHaveText(copy[lang].loginError);
  expect((await submit(device, form(challenge), { code: oldMfa.recoveryCode })).status()).toBe(302);
  await expect(device).toHaveURL(`/${lang}/`);
  await expectSession(device, lang);

  const lost = await isolatedPage();
  const route = `/${lang}/recovery-key`;
  const next = password();
  const recovery = {
    email: account.email, recovery_key: recoveryKey, password: next, password_confirmation: next,
  };
  expect((await open(lost, route)).status()).toBe(200);
  expect((await submit(lost, form(route), { ...recovery, password_confirmation: password() })).status()).toBe(302);
  await expect(lost).toHaveURL(route);
  await expect(notice(lost)).toHaveText(copy[lang].keyInvalid);
  expect((await submit(lost, form(route), recovery)).status()).toBe(302);
  await expectSignedOut(lost, lang);
  await device.reload({ waitUntil: 'domcontentloaded' });
  await expectSignedOut(device, lang);

  expect((await open(lost, route)).status()).toBe(200);
  expect((await submit(lost, form(route), recovery)).status()).toBe(302);
  await expect(lost).toHaveURL(route);
  await expect(notice(lost)).toHaveText(copy[lang].keyRejected);

  const recovered = { ...account, password: next };
  expect((await login(lost, lang, recovered)).status()).toBe(302);
  await expect(lost).toHaveURL(`/${lang}/`);
  const newMfa = await enroll(lost, lang, recovered);
  expect(newMfa.secret !== oldMfa.secret, 'Re-enrolment gives a new authenticator secret').toBe(true);
  expect((await login(lost, lang, recovered)).status()).toBe(302);
  await expect(lost).toHaveURL(challenge);
  expect((await submit(lost, form(challenge), { code: oldMfa.recoveryCode })).status()).toBe(302);
  await expect(alert(lost)).toHaveText(copy[lang].loginError);
  await expect.poll(() => Math.floor(Date.now() / 30_000), { timeout: 35_000 }).toBeGreaterThan(newMfa.step);
  expect((await submit(lost, form(challenge), { code: totp(newMfa.secret) })).status()).toBe(302);
  await expect(lost).toHaveURL(`/${lang}/`);
  await expectSession(lost, lang, 1);
});
