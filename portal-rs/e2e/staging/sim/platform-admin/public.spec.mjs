import {
  colleague, expect, form, language, origin, pathOf, requestReset, submitRegistration, test, visit,
  waitForPath,
} from './helpers.mjs';

test.afterEach(async ({ page }, info) => {
  if (info.status === 'failed' && new URL(page.url()).origin === origin) {
    await page.screenshot({
      path: info.outputPath('public-failure.png'),
      mask: [page.locator('input, textarea')],
      timeout: 5_000,
    }).catch(() => {});
  }
});

test('colleague registration validates confirmation and reaches the inbox', async ({ page }, info) => {
  const { lang } = info.project.metadata;
  const account = colleague();
  await visit(page, '/' + lang + '/register');
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(language(lang,
    'Utwórz konto', 'Create an account'));
  const terms = page.locator('main a[href$="/legal/terms"]');
  const privacy = page.locator('main a[href$="/legal/privacy"]');
  await expect(terms).toHaveAttribute('href', '/' + lang + '/legal/terms');
  await expect(privacy).toHaveAttribute('href', '/' + lang + '/legal/privacy');
  const rejected = await submitRegistration(page, account, account.password + 'x');
  expect(rejected.status(), 'Mismatched confirmation response').toBe(422);
  await expect(page.locator('#confirmation-error')).toHaveText(language(lang,
    'Hasła muszą być takie same.', 'Passwords must match.'));
  const accepted = await submitRegistration(page, account);
  expect(accepted.status(), 'Accepted registration response').toBe(303);
  const inbox = '/' + lang + '/register/check-inbox';
  await waitForPath(page, inbox);
  await visit(page, inbox);
  await expect(page.getByRole('status')).toHaveText(language(lang,
    'Sprawdź skrzynkę odbiorczą — wysłaliśmy wiadomość z dalszymi krokami.',
    'Check your inbox for an e-mail with the next steps.'));
});

test('password recovery validates email and confirms the request after redirect', async ({ page }, info) => {
  const { lang } = info.project.metadata;
  const account = colleague();
  const recovery = '/' + lang + '/forgot-password';
  await visit(page, recovery);
  const email = page.locator('main input[name=email]');
  await email.fill('missing-domain');
  await page.locator('main form button').click();
  expect(await email.evaluate(input => input.validity.typeMismatch)).toBe(true);
  expect(pathOf(page)).toBe(recovery);
  await requestReset(page, lang, account.email);
  await page.reload({ waitUntil: 'domcontentloaded' });
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(language(lang,
    'Zresetuj hasło', 'Reset password'));
  await expect(page.getByRole('status')).toHaveCount(0);
});

test('sign-in refuses unknown credentials and localizes a fresh form', async ({ page }, info) => {
  const { lang } = info.project.metadata;
  const login = '/' + lang + '/login';
  const account = colleague();
  await visit(page, login);
  const email = page.locator('main input[name=email]');
  await email.fill('missing-domain');
  await page.locator('main form button').click();
  expect(await email.evaluate(input => input.validity.typeMismatch)).toBe(true);
  const refused = await form(page, 'main form[action$="/login"]', account, 'button', { sensitive: true });
  expect(refused.status(), 'Sign-in refusal response').toBe(302);
  await waitForPath(page, login);
  await expect(page.getByRole('alert')).toHaveText(language(lang,
    'Nie udało się zweryfikować żądania. Sprawdź dane i spróbuj ponownie.',
    'The request could not be verified. Check your details and try again.'));
  const other = lang === 'pl' ? 'en' : 'pl';
  await visit(page, '/' + other + '/login');
  await expect(page.locator('html')).toHaveAttribute('lang', other);
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(language(other, 'Zaloguj się', 'Sign in'));
  expect(new URL(page.url()).origin).toBe(origin);
});
