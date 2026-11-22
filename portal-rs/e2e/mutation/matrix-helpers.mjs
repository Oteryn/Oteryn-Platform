import { test, expect } from '../fixtures.mjs';
import { loginWithRecoveryCode } from '../staff-rbac-helpers.mjs';

export async function signInWithRecoveryCode(page, lang, account, code) {
  const challengePath = `/${lang}/mfa/challenge`;
  const [response] = await Promise.all([
    page.waitForResponse(candidate => candidate.request().method() === 'POST'
      && new URL(candidate.url()).pathname === challengePath, { timeout: 120_000 }),
    loginWithRecoveryCode(page, lang, account, code),
  ]);
  expect(response.status()).toBe(302);
  expect(response.headers().location).toBe(`/${lang}/`);
  await expect(page).toHaveURL(new RegExp(`/${lang}/$`), { timeout: 60_000 });
}

export async function prepare(page, selector, values) {
  const form = page.locator(selector);
  await expect(form).toHaveCount(1);
  const { url, fields } = await form.evaluate(element => ({
    url: new URL(element.getAttribute('action') || element.ownerDocument.URL, element.baseURI).href,
    fields: Object.fromEntries(new FormData(element)),
  }));
  expect(Boolean(fields._csrf), 'the rendered form supplies a CSRF token').toBe(true);
  return { url, fields: { ...fields, ...values } };
}

export async function sendForm(page, url, form, method = 'POST') {
  return page.request.fetch(url, {
    method, form, headers: {
      origin: new URL(url).origin,
      'accept-language': await page.locator('html').getAttribute('lang') || 'pl',
      // Each direct request uses a fresh socket to the harness TLS proxy.
      connection: 'close',
    }, maxRedirects: 0,
  });
}

export async function useFormMethod(page, url, method, submitForm, target = url) {
  if (method === 'POST' && target === url) {
    await submitForm();
    return;
  }
  const changeMethod = route => route.continue(
    route.request().method() === 'POST' ? { method, url: target } : {},
  );
  await page.route(url, changeMethod);
  try {
    await submitForm();
  } finally {
    await page.unroute(url, changeMethod);
  }
}

export async function rejected({ page, url, fields, snapshot, baseline, method = 'POST' }) {
  const missing = Object.fromEntries(Object.entries(fields).filter(
    ([key, value]) => key !== '_csrf' && key !== 'csrf' && value !== undefined,
  ));
  expect((await sendForm(page, url, missing, method)).status()).toBe(403);
  expect(await snapshot()).toEqual(baseline);
  expect((await sendForm(page, url, { ...fields, _csrf: 'invalid' }, method)).status()).toBe(403);
  expect(await snapshot()).toEqual(baseline);
}

export async function wrongActor({ page, lang, url, fields, status, snapshot, baseline, method = 'POST' }) {
  await page.goto(`/${lang}/login`);
  const csrf = await page.locator('main form [name=_csrf]').inputValue();
  expect(Boolean(csrf), 'anonymous actor has its own CSRF token').toBe(true);
  const response = await sendForm(page, url, { ...fields, _csrf: csrf }, method);
  expect(response.status()).toBe(status);
  expect(await snapshot()).toEqual(baseline);
}

export async function invalid({ page, url, fields, samples, message, snapshot, baseline, method = 'POST' }) {
  for (const { name, patch } of samples) {
    await test.step(name, async () => {
      const response = await sendForm(page, url, { ...fields, ...patch }, method);
      expect(response.status()).toBe(422);
      expect(await response.text()).toContain(message);
      expect(await snapshot()).toEqual(baseline);
    });
  }
}
