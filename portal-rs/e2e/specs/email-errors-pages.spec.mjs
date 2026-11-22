import { randomUUID } from 'node:crypto';
import { test, expect } from '../fixtures.mjs';
import { login, register, waitForLoginBudget } from '../helpers.mjs';

const titles = {
  400: ['Nieprawidłowe żądanie', 'Invalid request'],
  401: ['Wymagane logowanie', 'Sign-in required'],
  403: ['Brak dostępu', 'Access denied'],
  404: ['Nie znaleziono strony', 'Page not found'],
  405: ['Niedozwolona metoda', 'Method not allowed'],
  413: ['Zbyt duże żądanie', 'Request too large'],
  429: ['Zbyt wiele żądań', 'Too many requests'],
  503: ['Usługa chwilowo niedostępna', 'Temporarily unavailable'],
};

async function assertError(page, lang, status, response) {
  expect(response.status()).toBe(status);
  const headers = response.headers();
  expect(headers['content-language']).toBe(lang);
  expect(headers['content-type']).toMatch(/^text\/html\b/);
  const cache = headers['cache-control'].split(',').map(value => value.trim());
  expect(cache.sort()).toEqual(status === 403 || status === 429 ? ['no-store', 'private'] : ['no-store']);
  const body = await response.text();
  await page.setContent(body);
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  for (const landmark of ['header#portal-header', 'main#main', 'footer']) {
    await expect(page.locator(landmark).first()).toBeVisible();
  }
  await expect(page.locator('header #portal-navigation')).toHaveCount(1);
  await expect(page.locator('main h1')).toHaveText(titles[status][lang === 'pl' ? 0 : 1]);
  expect(/stack backtrace|panicked at|sqlstate|database error|\/rustc\/|traceback/i.test(body),
    'error page excludes internal diagnostics').toBe(false);
}

test('public browser errors use the localized shell and no-store', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const headers = { accept: 'text/html', 'accept-language': lang };
  const options = { headers, maxRedirects: 0 };
  await page.goto(`/${lang}/`);
  for (const [status, path] of [
    [400, '/language/unsupported'],
    [401, `/${lang}/atlas/bestiary/019a0000-0000-7000-8000-000000000001`],
    [404, `/${lang}/missing-${randomUUID()}`],
  ]) {
    await assertError(page, lang, status, await page.request.get(path, options));
  }
  const forbidden = await page.request.post(`/${lang}/login`, { ...options, form: {} });
  await assertError(page, lang, 403, forbidden);
  const unsupported = await page.request.post(`/${lang}/`, options);
  expect(unsupported.headers().allow.split(/,\s*/)).toEqual(expect.arrayContaining(['GET', 'HEAD']));
  await assertError(page, lang, 405, unsupported);
  const oversized = await page.request.post(`/${lang}/`, {
    ...options, data: Buffer.alloc(1024 * 1024 + 1),
  });
  await assertError(page, lang, 413, oversized);
});

test('login throttling and a cold account dependency use localized error pages', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const account = await register(page, lang);
  await login(page, lang, account);
  const unavailable = await page.goto(`/${lang}/account/dashboard`);
  await assertError(page, lang, 503, unavailable);

  await page.context().clearCookies();
  await page.goto(`/${lang}/login`);
  const token = await page.locator('[name=_csrf]').inputValue();
  const form = { _csrf: token, email: `limit-${randomUUID()}@local.test`, password: 'Wrong local password!' };
  const headers = { accept: 'text/html', origin: new URL(page.url()).origin };
  await waitForLoginBudget(6);
  for (let attempt = 1; attempt <= 6; attempt++) {
    const response = await page.request.post(`/${lang}/login`, { form, headers, maxRedirects: 0 });
    if (attempt <= 5) {
      expect(response.status()).toBe(302);
    } else {
      expect(Number(response.headers()['retry-after'])).toBeGreaterThan(0);
      await assertError(page, lang, 429, response);
    }
  }
});
