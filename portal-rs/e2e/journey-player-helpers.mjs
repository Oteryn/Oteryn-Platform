import crypto from 'node:crypto';
import { expect } from './fixtures.mjs';
import { csrfRejected, mailLink, password, seedPasswordRange, submit, waitForLoginBudget } from './helpers.mjs';

export const localized = (lang, pl, en) => lang === 'pl' ? pl : en;

export async function visibleLink(page, href) {
  await page.waitForLoadState('load');
  const links = page.locator(`a[href="${href}"]:not([data-account-link])`);
  if (!(await links.filter({ visible: true }).count())) {
    const fallback = page.locator('header .om-fallback-menu:not([open]) > summary:visible');
    if (await fallback.count()) {
      await fallback.click();
    } else if (page.viewportSize().width <= 960) {
      const mobile = page.locator('#portal-menu-toggle:visible[aria-expanded=false]');
      if (await mobile.count()) await mobile.click();
    }
    for (const menu of ['world', 'community']) {
      const button = page.locator(`#portal-${menu}-button:enabled[aria-expanded=false]:visible`);
      if (await page.locator(`#portal-${menu}-panel a[href="${href}"]`).count() && await button.count()) {
        await button.click();
      }
    }
    const account = page.locator('header .om-fallback-account:not([open]) > summary:visible, [data-account-link]:visible');
    if (!(await links.filter({ visible: true }).count()) && await account.count()) await account.first().click();
  }

  return links.filter({ visible: true }).first();
}

export async function follow(page, href, status = 200, label) {
  const link = await visibleLink(page, href);
  if (label) await expect(link).toHaveText(label);
  if (href.includes('#')) {
    await link.click();
    await expect(page).toHaveURL(url => url.hash === '#' + href.split('#')[1]);
    return;
  }
  const destination = new URL(href, page.url());
  await expect(link).toBeVisible();
  const [response] = await Promise.all([
    page.waitForNavigation({ waitUntil: 'domcontentloaded' }),
    link.click(),
  ]);
  expect(page.url()).toBe(destination.href);
  expect(response.status(), destination.pathname).toBe(status);
  const locale = /^\/(en|pl)\//.exec(destination.pathname)?.[1];
  if (locale) await expect(page.locator('html')).toHaveAttribute('lang', locale);
  if (status === 503) expect(response.headers()['cache-control']).toContain('no-store');
  return response;
}

export async function signIn(page, account) {
  await waitForLoginBudget();
  await submit(page, 'main form', { email: account.email, password: account.password });
}

export async function newPlayer(page, lang, { showErrors = false } = {}) {
  const account = { email: `journey-${crypto.randomUUID()}@local.test`, password: password() };
  await seedPasswordRange(account.password);
  await follow(page, `/${lang}/register`);
  const form = page.locator('main form');
  const values = {
    email: account.email,
    password: account.password,
    password_confirmation: account.password,
    terms: true,
    privacy: true,
  };
  await form.locator('[name=work_token]').waitFor({ state: 'attached' });
  if (showErrors) {
    await submit(page, 'main form', { ...values, password_confirmation: 'Different local password' });
    await expect(form.locator('#password_confirmation')).toHaveAttribute('aria-invalid', 'true', { timeout: 90_000 });
    await expect(page.getByRole('alert')).toHaveText(localized(lang,
      'Popraw poniższe pola.', 'Please correct the fields below.'));
    await expect(form.locator('#confirmation-error')).toHaveText(localized(lang,
      'Hasła muszą być takie same.', 'Passwords must match.'));
  }
  await submit(page, 'main form', values);
  await expect(page).toHaveURL(new RegExp(`/${lang}/register/check-inbox$`), { timeout: 90_000 });
  return account;
}

export async function verifyPlayer(page, lang, email) {
  const link = await mailLink(page, email, /https:\/\/localhost:18443\/(?:pl|en)\/verify-email\?token=[a-f0-9]+/);
  expect((await page.goto(link)).status()).toBe(200);
  if (await page.locator('main form').count()) await submit(page);
  await expect(page.locator('main [role=status]')).toHaveText(localized(lang,
    'Twój adres e-mail został zweryfikowany.', 'Your e-mail has been verified.'));
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
}

export async function topUp(page, lang, action) {
  await follow(page, `/${lang}/shop/coins`);
  await submit(page, 'main form[action$="/shop/coins"]');
  await expect(page).toHaveURL(/\/shop\/test-checkout\/[a-f0-9-]+$/);
  await expect(page.locator('main')).toContainText(localized(lang,
    'TRYB TESTOWY – żadne prawdziwe pieniądze nie są pobierane',
    'TEST MODE – no real money is charged'));
  await csrfRejected(page, page.url(), { action });
  await page.locator(`main button[name=action][value=${action}]`).click();
  await expect(page).toHaveURL(/\/shop\/coins\?checkout=/);
}
