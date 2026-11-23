import { atPath, expect, password, submit, test, visit } from './fixtures.mjs';
import { login, register, registrationFields, verifyEmail } from './identity.mjs';

test('1.1 Registration checks the inbox, verifies by email and permits login', { tag: '@mail' }, async ({ mail, page, lang, account }) => {
  await register(page, lang, account);
  await verifyEmail(page, lang, account, mail);
  await login(page, lang, account);
});

test('1.2 Duplicate registration shows the same inbox notice and grants no session', async ({ page, lang, account }) => {
  test.setTimeout(650_000);
  await register(page, lang, account);
  const notice = await page.locator('main').innerText();
  await register(page, lang, { ...account, password: password() });
  await expect(page.locator('main')).toHaveText(notice);
  await visit(page, `/${lang}/account`, 200, { destination: `/${lang}/login` });
  await atPath(page, `/${lang}/login`);
});

const rejections = [
  ['1.3', 'short password', { password: 'short', password_confirmation: 'short' }, '#password-error',
    'Użyj co najmniej 10 znaków i maksymalnie 256 bajtów.',
    'Use at least 10 characters and at most 256 bytes.'],
  ['1.4', 'password mismatch', { password_confirmation: 'Different!Password123' }, '#confirmation-error',
    'Hasła muszą być takie same.', 'Passwords must match.'],
  ['1.5', 'invalid email', { email: false }, '#email-error',
    'Podaj poprawny adres e-mail.', 'Enter a valid e-mail address.'],
  ['1.6', 'missing legal consent', { terms: false, privacy: false }, '#terms-error, #privacy-error',
    'Zaakceptuj aktualną wersję dokumentu.', 'Accept the current document version.'],
];

for (const [number, label, overrides, selector, pl, en] of rejections) {
  test(`${number} Registration refuses ${label}`, async ({ page, lang, account }) => {
    const path = `/${lang}/register`;
    await visit(page, path);
    if (number === '1.5' || number === '1.6') {
      // Let the malformed browser form reach server-side validation.
      await page.locator('main form').evaluate(form => { form.noValidate = true; });
    }
    const values = registrationFields(account, {
      ...overrides,
      email: overrides.email === false ? account.email.replace('@', '-') : account.email,
    });
    await submit(page, path, 422, values);
    await atPath(page, path);
    const errors = page.locator(selector);
    await expect(errors).toHaveCount(number === '1.6' ? 2 : 1);
    for (const error of await errors.all()) {
      await expect(error, `POST ${path} ${label} field error`).toHaveText(lang === 'pl' ? pl : en);
    }
    await expect(page.locator('main [role=alert]')).toHaveText(
      lang === 'pl' ? 'Popraw poniższe pola.' : 'Please correct the fields below.');
  });
}
