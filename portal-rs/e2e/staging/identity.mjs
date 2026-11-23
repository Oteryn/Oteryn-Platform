import { atPath, expect, submit, visit } from './fixtures.mjs';
import { mailLink } from './mail.mjs';

const copy = {
  pl: {
    inbox: 'Sprawdź skrzynkę odbiorczą — wysłaliśmy wiadomość z dalszymi krokami.',
    verified: 'Twój adres e-mail został zweryfikowany.',
    verifySubject: 'Potwierdź adres e-mail',
    loginError: 'Nie udało się zweryfikować żądania. Sprawdź dane i spróbuj ponownie.',
    resetSubject: 'Zresetuj hasło do Oteryn',
    resetNotice: 'Jeśli konto z tym adresem istnieje, wysłaliśmy link do zresetowania hasła.',
    resetError: 'Link jest nieprawidłowy lub wygasł albo hasła nie są identyczne. W razie potrzeby poproś o nowy link.',
    overview: 'Przegląd konta',
  },
  en: {
    inbox: 'Check your inbox for an e-mail with the next steps.',
    verified: 'Your e-mail has been verified.',
    verifySubject: 'Verify your email address',
    loginError: 'The request could not be verified. Check your details and try again.',
    resetSubject: 'Reset your Oteryn password',
    resetNotice: 'If an account exists for that email, a password reset link has been sent.',
    resetError: 'This password reset link is invalid or expired, or the passwords do not match. Request a new link if needed.',
    overview: 'Account overview',
  },
};
export { copy };

export function registrationFields(account, overrides = {}) {
  return { ...account, password_confirmation: account.password, terms: true, privacy: true, ...overrides };
}

export async function register(page, lang, account) {
  const path = `/${lang}/register`;
  await visit(page, path);
  await page.locator('[name=work_token]').waitFor({ state: 'attached' });
  await submit(page, path, 303, registrationFields(account));
  await atPath(page, `${path}/check-inbox`);
  const notice = page.locator('main [role=status]');
  await expect(notice, `GET ${path}/check-inbox`).toHaveText(copy[lang].inbox);
}

export async function verifyEmail(page, lang, account, mail) {
  const link = await mailLink(mail, account.email, copy[lang].verifySubject, lang, 'verify');
  await visit(page, link);
  await submit(page, link, 200);
  await expect(page.locator('main [role=status]')).toHaveText(copy[lang].verified);
}

export async function login(page, lang, account, outcome = '') {
  const path = `/${lang}/login`;
  await visit(page, path);
  await submit(page, path, 302, account);
  await atPath(page, `/${lang}/${outcome}`);
  if (outcome === 'login') {
    await expect(page.locator('main [role=alert]'), `POST ${path} generic refusal`).toHaveText(copy[lang].loginError);
  }
}

export async function logout(page, lang) {
  await visit(page, `/${lang}/`);
  const selector = 'form[action$="/logout"]';
  const menu = page.locator('#portal-menu-toggle');
  if (await menu.isVisible()) await menu.click();
  if (!await page.locator(selector).isVisible()) await page.locator('[data-account-link]').click();
  await submit(page, `/${lang}/logout`, 302, {}, selector);
  await atPath(page, `/${lang}/`);
}

export async function verifiedLogin(page, lang, account, mail) {
  await register(page, lang, account);
  await verifyEmail(page, lang, account, mail);
  await login(page, lang, account);
}
