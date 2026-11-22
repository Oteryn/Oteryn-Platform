import { test, expect } from '../fixtures.mjs';
import { password } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { emailAddress, postForm, seedAccount } from './auth-fixtures.mjs';

test('login throttles the normalized identity plus source and the source aggregate', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const route = `/${lang}/login`;
  const account = await seedAccount(lang);
  const unknown = emailAddress();
  const wrong = password();

  await page.goto(route);
  // A full source budget must fit in a single real database minute.
  await expect.poll(async () => {
    const [window] = await query(`
      SELECT extract(second FROM clock_timestamp()) < 8 AS early,
             NOT EXISTS (SELECT 1 FROM web_rate_limits
                         WHERE limiter_name='identity-login-source'
                           AND window_start=date_trunc('minute',clock_timestamp())) AS unused
    `);
    return window.early && window.unused;
  }, { timeout: 65_000 }).toBe(true);
  async function attempt(email, pass = wrong) {
    return postForm(page, 'main form', { email, password: pass });
  }
  async function limited(response) {
    expect(response.status()).toBe(429);
    const retry = Number(response.headers()['retry-after']);
    expect(retry).toBeGreaterThanOrEqual(1);
    expect(retry).toBeLessThanOrEqual(60);
    const body = await response.text();
    expect(body).toContain(lang === 'pl' ? 'Zbyt wiele żądań' : 'Too many requests');
    expect(body).toContain(lang === 'pl' ? 'Poczekaj przed kolejną próbą.' : 'Please wait before trying again.');
  }
  for (let attemptNumber = 0; attemptNumber < 5; attemptNumber += 1) {
    expect((await attempt(account.email)).status()).toBe(302);
    expect((await attempt(unknown)).status()).toBe(302);
  }
  await limited(await attempt(account.email.toUpperCase(), account.password));
  await limited(await attempt(unknown));
  for (let attemptNumber = 0; attemptNumber < 8; attemptNumber += 1) {
    expect((await attempt(emailAddress())).status()).toBe(302);
  }
  await limited(await attempt(emailAddress()));
});
