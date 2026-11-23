import { randomBytes } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';
import { expect, origin, test, waitUntilReady } from './helpers.mjs';

const copy = {
  en: {
    login: 'Log in',
    rejected: 'The provided credentials are invalid.',
    register: 'Create an account',
    inbox: 'Check your inbox for an e-mail with the next steps.',
    unavailable: 'Temporarily unavailable',
    alt: 'pl',
  },
  pl: {
    login: 'Logowanie',
    rejected: 'Podane dane logowania są nieprawidłowe.',
    register: 'Utwórz konto',
    inbox: 'Sprawdź skrzynkę odbiorczą — wysłaliśmy wiadomość z dalszymi krokami.',
    unavailable: 'Usługa chwilowo niedostępna',
    alt: 'en',
  },
};

test.beforeEach(async ({ request }) => waitUntilReady(request));

test('signed-out browser reaches login and both tabs see the staff host boundary', async ({
  page, context,
}, testInfo) => {
  const lang = testInfo.project.metadata.language;
  const login = `/${lang}/login`;
  expect((await page.goto(origin + login)).status()).toBe(200);
  const heading = page.getByRole('heading', { level: 1 });
  await expect(heading).toBeVisible();
  await expect.soft(heading).toHaveAccessibleName(copy[lang].login);
  if (testInfo.errors.length) {
    await page.screenshot({ path: testInfo.outputPath('login-heading.png') });
  }

  const otherTab = await context.newPage();
  expect((await otherTab.goto(origin + '/admin')).status()).toBe(404);
  await expect(otherTab).toHaveURL(origin + '/admin');
  expect((await otherTab.reload()).status()).toBe(404);

  expect((await page.goto(origin + `/admin/${lang}/party`)).status()).toBe(404);
  await page.goBack();
  await expect(page).toHaveURL(origin + login);
  await expect.soft(heading).toHaveAccessibleName(copy[lang].login);
});

test('localized registration validates empty input and reports admission outcome', async ({
  page,
}, testInfo) => {
  const lang = testInfo.project.metadata.language;
  const register = `/${lang}/register`;
  const email = `sim-m-admin-${randomBytes(8).toString('hex')}-1@example.test`;
  const password = randomBytes(27).toString('base64url');
  expect((await page.goto(origin + `/${lang}/login`)).status()).toBe(200);
  await page.locator(`main a[href="${register}"]`).click();
  await expect(page.getByRole('heading', { level: 1, name: copy[lang].register })).toBeVisible();

  const form = page.locator(`form[action="${register}"]`);
  await form.locator('button[type=submit]').click();
  const emailInput = form.locator('[name=email]');
  await expect(emailInput).toBeFocused();
  expect(await emailInput.evaluate((input) => input.validity.valueMissing)).toBe(true);
  await expect(page).toHaveURL(origin + register);
  await emailInput.fill('missing-domain');
  await form.locator('button[type=submit]').click();
  await expect(emailInput).toBeFocused();
  expect(await emailInput.evaluate((input) => input.validity.typeMismatch)).toBe(true);

  await page.locator(`a[href="/${copy[lang].alt}/register"]`).first().click();
  await expect(page.getByRole('heading', { level: 1, name: copy[copy[lang].alt].register }))
    .toBeVisible();
  await page.goBack();
  await expect(page).toHaveURL(origin + register);

  let status;
  let challengeStatus;
  let rateLimited;
  for (let attempt = 0; attempt < 3; attempt += 1) {
    if (attempt) {
      // Other staging simulations may use the same network source.
      await delay(65_000);
      expect((await page.goto(origin + register)).status()).toBe(200);
    }
    await emailInput.fill(email);
    await form.locator('[name=password]').fill(password);
    await form.locator('[name=password_confirmation]').fill(password);
    await form.locator('[name=terms]').check();
    await form.locator('[name=privacy]').check();
    challengeStatus = undefined;
    const submitted = page.waitForResponse((response) => {
      if (response.request().method() !== 'POST') return false;
      const path = new URL(response.url()).pathname;
      if (path === '/register/abuse/challenge') challengeStatus = response.status();
      return path === register;
    }, { timeout: 90_000 });
    await form.locator('button[type=submit]').click();
    status = (await submitted).status();
    rateLimited = status === 429 || challengeStatus === 429;
    if (!rateLimited) break;
  }
  test.skip(rateLimited, 'Registration source stayed rate-limited after two delayed attempts');
  if (status === 503) {
    // The deployment explicitly gates registration on mail delivery readiness.
    await expect(page).toHaveURL(origin + register);
    await expect(page.getByRole('heading', { level: 1, name: copy[lang].unavailable }))
      .toBeVisible();
    expect((await page.goto(origin + register)).status()).toBe(200);
    expect((await page.reload()).status()).toBe(200);
    await expect(page.getByRole('heading', { level: 1, name: copy[lang].register })).toBeVisible();
    return;
  }
  if (status !== 303) {
    await page.screenshot({
      path: testInfo.outputPath('registration-admission.png'),
      mask: [page.locator('input, textarea')],
    });
  }
  expect(status, `registration admission after challenge status ${challengeStatus ?? 'unknown'}`)
    .toBe(303);
  await expect(page).toHaveURL(origin + register + '/check-inbox');
  await expect(page.getByText(copy[lang].inbox, { exact: true })).toBeVisible();
  expect((await page.reload()).status()).toBe(200);
  await expect(page.getByText(copy[lang].inbox, { exact: true })).toBeVisible();

  expect((await page.goto(origin + `/${lang}/login`)).status()).toBe(200);
  const login = page.locator(`form[action="/${lang}/login"]`);
  await login.locator('[name=email]').fill(email);
  await login.locator('[name=password]').fill(password);
  const loginSubmitted = page.waitForResponse((response) =>
    new URL(response.url()).pathname === `/${lang}/login` && response.request().method() === 'POST',
  );
  await login.locator('button[type=submit]').click();
  expect((await loginSubmitted).status()).toBe(302);
  await expect(page.getByText(copy[lang].rejected, { exact: true })).toBeVisible();
  expect((await page.goto(origin + '/admin')).status()).toBe(404);
});
