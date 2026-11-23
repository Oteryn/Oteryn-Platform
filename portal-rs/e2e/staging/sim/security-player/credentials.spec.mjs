import {
  authenticated, expect, login, mail, mailPath, mainForm, newEmail, newPassword,
  open, origin, signedOut, submit, sudo, test, totp, totpCounter, tr,
} from './helpers.mjs';
import { setTimeout as delay } from 'node:timers/promises';

const scope = info => info.project.metadata.locale;
const loginError = lang => tr(lang, 'Podane dane logowania są nieprawidłowe.', 'The provided credentials are invalid.');
const passwordError = lang => tr(lang, 'Sprawdź obecne hasło oraz pola nowego hasła.',
  'Check your current password and the new password fields.');
const factorError = lang => tr(lang, 'Nie udało się zweryfikować żądania. Sprawdź dane i spróbuj ponownie.',
  'The request could not be verified. Check your details and try again.');

async function activateKey(page, lang, player) {
  const route = `/${lang}/account/recovery-key`;
  await open(page, route);
  expect((await submit(mainForm(page), { current_password: player.password })).status()).toBe(200);
  const code = page.locator('main .op-code');
  await expect(code).toBeVisible();
  const key = (await code.innerText()).trim();
  expect(key.length).toBeGreaterThan(30);
  expect((await submit(mainForm(page), { current_password: player.password, recovery_key: key })).status()).toBe(200);
  await expect(page.getByText(tr(lang, 'Klucz odzyskiwania jest aktywny.', 'Your recovery key is active.'))).toBeVisible();
  await open(page, route);
  await expect(code).toHaveCount(0);
  return key;
}

test('password change rejects bad input, reuse and wrong credentials, then revokes both sessions', async ({ page, player, staging }, info) => {
  const lang = scope(info);
  const route = `/${lang}/password/change`;
  const other = await staging.openPage();
  await login(other, lang, player);
  await authenticated(other, lang);
  await open(page, route);
  const form = mainForm(page);
  const password = newPassword();
  for (const [current, next, confirmation] of [
    [player.password, password, newPassword()],
    [player.password, player.password, player.password],
    [newPassword(), password, password],
  ]) {
    expect((await submit(form, {
      current_password: current, password: next, password_confirmation: confirmation,
    })).status()).toBe(302);
    await expect(page.locator('main [role=alert]')).toHaveText(passwordError(lang));
  }

  const since = Date.now() - 2000;
  const response = await submit(form, {
    current_password: player.password, password, password_confirmation: password,
  });
  expect(response.status()).toBe(302);
  await expect(page).toHaveURL(/\/login$/);
  await signedOut(other, lang);
  await login(page, lang, player);
  await expect(page.locator('main [role=alert]')).toHaveText(loginError(lang));
  await login(page, lang, { email: player.email, password });
  await authenticated(page, lang);
  await mail(page.request, player.email, since, tr(lang, 'Zmieniono hasło', 'Password changed'),
    lang === 'pl' ? /Hasło Twojego konta zostało zmienione/ : /Your account password has been changed/);
});

test('email change confirms the new address, alerts the old one and revokes the old identity', async ({ page, player, staging }, info) => {
  const lang = scope(info);
  const route = `/${lang}/account/security/email`;
  const key = await activateKey(page, lang, player);
  await open(page, route);
  const email = newEmail();
  const form = mainForm(page);
  const input = {
    email, email_confirmation: email, current_password: player.password, proof: key, recovery: true,
  };
  const refused = tr(lang, 'Odmowa lub nieprawidłowe dane.', 'Request refused or invalid input.');
  for (const change of [{ email_confirmation: newEmail() }, { current_password: newPassword() }]) {
    expect((await submit(form, { ...input, ...change })).status()).toBe(302);
    await expect(page.locator('main [role=status]')).toHaveText(refused);
  }

  const since = Date.now() - 2000;
  expect((await submit(form, input)).status()).toBe(302);
  await expect(page.locator('main [role=status]')).toHaveText(tr(lang,
    'Sprawdź oba adresy e-mail, aby potwierdzić lub anulować zmianę.',
    'Check both email addresses for confirmation and cancellation links.'));

  const subject = tr(lang, 'Bezpieczeństwo adresu e-mail', 'Email security');
  const [confirmation, cancellation] = await Promise.all([
    mail(page.request, email, since, subject, /https:\/\/[^\s"'<>]+\/email-change\/confirm\/[a-f0-9]{64}/),
    mail(page.request, player.email, since, subject, /https:\/\/[^\s"'<>]+\/email-change\/revert\/[a-f0-9]{64}/),
  ]);
  const inbox = await staging.openPage();
  await open(inbox, mailPath(cancellation));
  await expect(inbox.locator('main')).toContainText(tr(lang, 'To nie ja', 'This was not me'));
  await open(inbox, mailPath(confirmation));
  await expect(inbox.locator('main')).toContainText(tr(lang, 'Potwierdź nowy adres e-mail', 'Confirm your new email address'));

  // Loading the link alone must leave the current session intact.
  await authenticated(page, lang);
  expect((await submit(mainForm(inbox))).status()).toBe(302);
  await expect(inbox).toHaveURL(`${origin}/${lang}/login`);
  await signedOut(page, lang);
  await login(page, lang, player);
  await expect(page.locator('main [role=alert]')).toHaveText(loginError(lang));
  await login(inbox, lang, { ...player, email });
  await authenticated(inbox, lang);
});

test('TOTP enrolls, rejects an incorrect and replayed code, and makes saved recovery codes single-use', async ({ page, player, staging }, info) => {
  const lang = scope(info);
  const settings = `/${lang}/mfa`;
  const enrollment = page.locator('main form[action$="/mfa/enroll"]');
  await open(page, settings);
  expect((await submit(enrollment, { password: player.password })).status()).toBe(200);
  const secret = (await page.locator('main .op-prose').innerText()).trim();
  const confirm = page.locator('main form[action$="/mfa/confirm"]');
  const wrongCode = () => {
    const counter = totpCounter();
    const valid = [-2, -1, 0, 1, 2].map(offset => totp(secret, counter + offset));
    return ['000000', '100000', '200000', '300000', '400000', '500000'].find(code => !valid.includes(code));
  };
  expect((await submit(confirm, { password: player.password, code: wrongCode() })).status()).toBe(302);
  await expect(page.locator('main [role=alert]')).toHaveText(factorError(lang));
  expect((await submit(enrollment, { password: player.password })).status()).toBe(200);
  const active = (await page.locator('main .op-prose').innerText()).trim();
  const enrolledStep = totpCounter();
  expect((await submit(confirm, { password: player.password, code: totp(active, enrolledStep) })).status()).toBe(200);
  const codes = await page.locator('main textarea[readonly]').evaluateAll(nodes => nodes.map(node => node.value));
  expect(codes.length).toBeGreaterThanOrEqual(2);
  expect(new Set(codes).size).toBe(codes.length);

  await login(page, lang, player);
  await expect(page).toHaveURL(/\/mfa\/challenge$/);
  const challenge = target => target.locator('main form[action$="/mfa/challenge"]');
  await page.reload();
  expect((await submit(challenge(page), { code: 'not-a-code' })).status()).toBe(302);
  await expect(page.locator('main [role=alert]')).toHaveText(factorError(lang));
  // Use the next time step because enrollment has already consumed its code.
  await delay(Math.max(0, (enrolledStep + 1) * 30_000 + 1000 - Date.now()));
  const code = totp(active);
  expect((await submit(challenge(page), { code })).status()).toBe(302);
  await authenticated(page, lang);
  await open(page, settings);
  await expect(enrollment).toHaveCount(0);
  await expect(page.locator('main textarea[readonly]')).toHaveCount(0);

  const recovery = await staging.openPage();
  await login(recovery, lang, player);
  await expect(recovery).toHaveURL(/\/mfa\/challenge$/);
  await submit(challenge(recovery), { code });
  await expect(recovery.locator('main [role=alert]')).toHaveText(factorError(lang));
  await submit(challenge(recovery), { code: codes[0] });
  await authenticated(recovery, lang);

  const replay = await staging.openPage();
  await login(replay, lang, player);
  await expect(replay).toHaveURL(/\/mfa\/challenge$/);
  await submit(challenge(replay), { code: codes[0] });
  await expect(replay.locator('main [role=alert]')).toHaveText(factorError(lang));
  await submit(challenge(replay), { code: codes[1] });
  await authenticated(replay, lang);

  // Discover the player-facing control from the settings UI. The current
  // product exposes trusted-device removal but has no self-service TOTP disable.
  await sudo(page, lang, player.password);
  await open(page, settings);
  await expect(page.getByRole('button', {
    name: /(?:Wyłącz.*(?:2FA|dwuskładnikowe)|Disable.*(?:MFA|two-factor))/i,
  }), 'player can initiate MFA disable after re-authentication').toBeVisible();
});

test('account recovery key changes the password once and invalidates an existing session', async ({ page, player, staging }, info) => {
  const lang = scope(info);
  const key = await activateKey(page, lang, player);
  const recovery = await staging.openPage();
  await open(recovery, `/${lang}/recovery-key`);
  const form = mainForm(recovery);
  const password = newPassword();
  const fields = { email: player.email, recovery_key: key, password, password_confirmation: password };
  expect((await submit(form, { ...fields, password_confirmation: newPassword() })).status()).toBe(302);
  await expect(recovery.locator('main')).toContainText(tr(lang,
    'Sprawdź e-mail, klucz i hasło.', 'Check your email, key and password.'));
  expect((await submit(form, fields)).status()).toBe(302);
  await expect(recovery).toHaveURL(/\/login$/);
  await signedOut(page, lang);

  await open(recovery, `/${lang}/recovery-key`);
  const replacement = newPassword();
  expect((await submit(form, { ...fields, password: replacement, password_confirmation: replacement })).status()).toBe(302);
  await expect(recovery.locator('main')).toContainText(tr(lang,
    'Dane są nieprawidłowe lub ta operacja jest niedostępna.', 'The credentials are invalid or this action is unavailable.'));
  await login(recovery, lang, { email: player.email, password });
  await authenticated(recovery, lang);
});
