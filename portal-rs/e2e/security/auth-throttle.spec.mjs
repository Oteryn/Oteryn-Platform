import { test, expect } from '../fixtures.mjs';
import { password } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { emailAddress, postForm, seedAccount } from './auth-fixtures.mjs';

test.use({ trace: 'off', screenshot: 'off' });

async function startOfMinute() {
  await expect.poll(async () => {
    const [window] = await query('SELECT extract(second FROM clock_timestamp()) < 5 AS early');
    return window.early;
  }, { timeout: 65_000 }).toBe(true);
}

async function limited(response, lang) {
  expect(response.status()).toBe(429);
  const retry = Number(response.headers()['retry-after']);
  expect(retry).toBeGreaterThanOrEqual(1);
  expect(retry).toBeLessThanOrEqual(60);
  const body = await response.text();
  expect(body).toContain(lang === 'pl' ? 'Zbyt wiele żądań' : 'Too many requests');
  expect(body).toContain(lang === 'pl' ? 'Poczekaj przed kolejną próbą.' : 'Please wait before trying again.');
}

function attempt(page, source, email, password) {
  return postForm(page, 'main form', { email, password }, {
    headers: { 'x-portal-e2e-login-peer': source },
  });
}

async function expectLogin(page, lang, source, account) {
  const accepted = await attempt(page, source, account.email, account.password);
  expect(accepted.status()).toBe(302);
  expect(accepted.headers().location).toBe(`/${lang}/`);
}

test('the login identity budget is keyed by normalized email and source', async ({ page }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const account = await seedAccount(lang);
  const unknown = emailAddress();
  const wrong = password();
  await page.goto(`/${lang}/login`);
  await startOfMinute();

  for (let index = 0; index < 5; index++) {
    // Run the hashing for distinct identities concurrently within the same window.
    for (const response of await Promise.all([
      attempt(page, '127.0.0.2', account.email, wrong),
      attempt(page, '127.0.0.2', unknown, wrong),
    ])) {
      expect(response.status()).toBe(302);
      expect(response.headers().location).toBe(`/${lang}/login`);
    }
  }
  await limited(await attempt(page, '127.0.0.2', account.email.toUpperCase(), account.password), lang);
  await limited(await attempt(page, '127.0.0.2', unknown, wrong), lang);
  await expectLogin(page, lang, '127.0.0.3', account);
});

test('the login source budget charges malformed requests and isolates other peers', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const account = await seedAccount(lang);
  await page.goto(`/${lang}/login`);
  // This source shares its bucket with the previous test. Require a fresh minute.
  await expect.poll(async () => {
    const [window] = await query(`
      SELECT extract(second FROM clock_timestamp()) < 5 AS early,
             NOT EXISTS (SELECT 1 FROM web_rate_limits
                         WHERE limiter_name='identity-login-source'
                           AND window_start=date_trunc('minute',clock_timestamp())) AS unused
    `);
    return window.early && window.unused;
  }, { timeout: 65_000 }).toBe(true);

  for (let index = 0; index < 20; index++) {
    const response = await attempt(page, '127.0.0.2', '', '');
    expect(response.status()).toBe(302);
    expect(response.headers().location).toBe(`/${lang}/login`);
  }
  await limited(await attempt(page, '127.0.0.2', account.email, account.password), lang);
  await expectLogin(page, lang, '127.0.0.3', account);
});
