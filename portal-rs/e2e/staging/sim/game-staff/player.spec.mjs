import { randomUUID } from 'node:crypto';
import {
  baseURL, capture, expect, formFor, get, local, mailLink, mailReady, mailValue,
  password, ready, submit, submitPost, test, withStagingLock,
} from './helpers.mjs';

test.beforeEach(async ({ request }) => ready(request));

test('guest staff-panel boundary and password reset form', async ({ page }, info) => {
  const { lang } = info.project.metadata;
  const reset = `/${lang}/forgot-password`;
  expect((await page.goto(reset)).status()).toBe(200);
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(local(lang, 'Zresetuj hasło', 'Reset password'));
  const form = formFor(page, reset);
  await submit(form, { email: 'sim-invalid-address' });
  await expect(page).toHaveURL(`${baseURL}${reset}`);
  expect(await form.locator('[name=email]').evaluate(input => input.validity.typeMismatch)).toBe(true);
  await page.reload();
  expect(await submitPost(page, reset, {
    email: `sim-game-staff-${randomUUID()}-1@example.test`,
  })).toBe(302);
  await expect(page.getByRole('status')).toHaveText(local(
    lang,
    'Jeśli konto z tym adresem istnieje, wysłaliśmy link do zresetowania hasła.',
    'If an account exists for that email, a password reset link has been sent.',
  ));

  const panel = await get(page.request, '/admin');
  // The host boundary may hide the entire panel. A mounted panel redirects guests.
  expect([303, 404]).toContain(panel.status());
  if (panel.status() === 303) expect(panel.headers().location).toBe('/login');
  const analytics = await get(page.request, '/admin/analytics');
  expect([403, 404]).toContain(analytics.status());
  await info.attach('guest-statuses', {
    body: JSON.stringify({ '/admin': panel.status(), '/admin/analytics': analytics.status() }),
    contentType: 'application/json',
  });
});

const inboxText = lang => local(lang,
  'Sprawdź skrzynkę odbiorczą — wysłaliśmy wiadomość z dalszymi krokami.',
  'Check your inbox for an e-mail with the next steps.',
);

async function registerPlayer(page, lang, info, validateInput) {
  const register = `/${lang}/register`;
  const email = `sim-game-staff-${randomUUID()}-1@example.test`;
  const pass = password();
  const open = await page.goto(register, { waitUntil: 'domcontentloaded' });
  if (open.status() !== 200) await capture(page, info, 'registration-page');
  expect(open.status(), `GET ${register}`).toBe(200);
  await expect(page.getByRole('heading', { level: 1 })).toHaveText(local(lang, 'Utwórz konto', 'Create an account'));
  const valid = { email, password: pass, password_confirmation: pass, terms: true, privacy: true };
  if (validateInput) {
    const form = formFor(page, register);
    await submit(form, { ...valid, password_confirmation: `x${pass}`, privacy: false });
    expect(await form.locator('[name=privacy]').evaluate(input => input.validity.valueMissing)).toBe(true);
    await expect(page).toHaveURL(`${baseURL}${register}`);
    try {
      expect(await submitPost(page, register, { ...valid, password_confirmation: `x${pass}` })).toBe(422);
      await expect(page.locator('#confirmation-error')).toHaveText(local(
        lang, 'Hasła muszą być takie same.', 'Passwords must match.',
      ));
    } catch (error) {
      if (!error.constructor.name.includes('Skip')) await capture(page, info, 'registration-validation');
      throw error;
    }
  }
  const since = Date.now() - 2_000;
  try {
    // The validation response renders both consent checkboxes unchecked.
    expect(await submitPost(page, register, valid)).toBe(303);
    await expect(page).toHaveURL(`${baseURL}${register}/check-inbox`);
    await expect(page.getByRole('status')).toHaveText(inboxText(lang));
  } catch (error) {
    if (!error.constructor.name.includes('Skip')) await capture(page, info, 'registration-check-inbox');
    throw error;
  }
  return { email, since };
}

test('player registration, input validation and check-inbox', async ({ page }, info) => {
  const { lang } = info.project.metadata;
  await withStagingLock('registration', () => registerPlayer(page, lang, info, true));
  expect((await page.reload()).status()).toBe(200);
  await expect(page.getByRole('status')).toHaveText(inboxText(lang));
  await page.goBack();
  await expect(formFor(page, `/${lang}/register`)).toBeVisible();
  await page.goForward();
  await expect(page.getByRole('status')).toHaveText(inboxText(lang));
});

test('player email verification and used-link refusal', async ({ page }, info) => {
  const { lang } = info.project.metadata;
  await mailReady(page.request);
  const { email, since } = await withStagingLock('registration', () => registerPlayer(page, lang, info, false));
  const link = await mailValue(page.request, email, local(lang, 'Potwierdź adres e-mail', 'Verify your email address'), since,
    body => mailLink(body, /^\/(pl|en)\/verify-email$/));
  expect(Boolean(link), 'Verification mail arrived for this new account').toBe(true);
  try {
    expect((await page.goto(link)).status()).toBe(200);
    await submit(page.locator('main form'));
    await expect(page.getByRole('status')).toHaveText(local(lang,
      'Twój adres e-mail został zweryfikowany.', 'Your e-mail has been verified.'));
    expect((await page.goto(link)).status()).toBe(422);
    await expect(page.getByRole('status')).toHaveText(local(lang,
      'Ten link jest nieprawidłowy lub wygasł. Poproś o nową wiadomość weryfikacyjną.',
      'This link is invalid or has expired. Request a new verification e-mail.'));
  } catch {
    throw new Error('Verification did not show the confirmation and used-link refusal from the portal');
  } finally {
    await page.goto(`/${lang}/login`);
  }
});
