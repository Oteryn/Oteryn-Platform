import { test, expect } from '../fixtures.mjs';
import { csrfRejected, mailLink, password, seedPasswordRange, submit, waitForLoginBudget } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { emailAddress, postForm, seedAccount } from './auth-fixtures.mjs';

test.use({ trace: 'off', screenshot: 'off' });

test('login gives a localized, identical refusal for wrong credentials and unknown accounts', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const account = await seedAccount(lang);
  await waitForLoginBudget(2);
  const messages = [];
  for (const email of [account.email, emailAddress()]) {
    await page.goto(`/${lang}/login`);
    const values = { email, password: password() };
    await csrfRejected(page, `/${lang}/login`, values);
    const response = await postForm(page, 'main form', values);
    expect(response.status()).toBe(302);
    expect(response.headers().location).toBe(`/${lang}/login`);
    expect((await page.goto(response.headers().location)).status()).toBe(200);
    const alert = page.locator('main [role=alert]');
    await expect(alert).toBeVisible();
    messages.push(await alert.innerText());
  }
  expect(messages[0]).toBe(messages[1]);
  expect(messages[0]).toBe(lang === 'pl'
    ? 'Nie udało się zweryfikować żądania. Sprawdź dane i spróbuj ponownie.'
    : 'The request could not be verified. Check your details and try again.');
});

test('registration gives the same confirmation for a new and an existing email', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const account = await seedAccount(lang);
  const newEmail = emailAddress();
  const confirmation = `/${lang}/register/check-inbox`;
  const messages = [];
  const pass = await seedPasswordRange('signup');
  for (const email of [newEmail, account.email]) {
    expect((await page.goto(`/${lang}/register`)).status()).toBe(200);
    await page.locator('[name=work_token]').waitFor({ state: 'attached' });
    const submitted = page.waitForResponse(response => response.request().method() === 'POST' &&
      response.status() !== 403 && new URL(response.url()).pathname === `/${lang}/register`);
    await submit(page, 'main form', {
      email, password: pass, password_confirmation: pass, terms: true, privacy: true,
    });
    const response = await submitted;
    expect(response.status()).toBe(303);
    expect(response.headers().location).toBe(confirmation);
    await expect(page).toHaveURL(new RegExp(`${confirmation}$`));
    messages.push(await page.locator('main').innerText());
  }
  expect(messages[0]).toBe(messages[1]);
  const [existing, created] = await Promise.all([account.email, newEmail].map(async email =>
    (await query('SELECT id,status FROM accounts WHERE email=$1', [email]))[0]));
  expect(existing).toEqual({ id: account.id, status: 'active' });
  expect(created?.status).toBe('pending_verification');

  const verificationLink = await mailLink(page, newEmail,
    /https:\/\/localhost:18443\/(?:pl|en)\/verify-email\?token=[a-f0-9]+/);
  expect((await page.goto(verificationLink)).status()).toBe(200);
  await csrfRejected(page, `/${lang}/verify-email/resend`);
  await submit(page, 'main form');
  const [verified] = await query('SELECT status FROM accounts WHERE id=$1', [created.id]);
  expect(verified?.status).toBe('active');
});
