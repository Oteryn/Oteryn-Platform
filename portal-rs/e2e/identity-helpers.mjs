import { execFileSync } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect } from './fixtures.mjs';
import { password, pool, seedPasswordRange, submit, waitForLoginBudget } from './helpers.mjs';

export const localized = (locale, en, pl) => locale === 'en' ? en : pl;
export const identityEmail = () => `identity-${randomUUID()}@example.invalid`;

export async function identityQuery(sql, parameters = []) {
  const db = pool();
  try {
    return await db.query(sql, parameters);
  } finally {
    await db.end();
  }
}

export async function seedPlayer(locale) {
  const db = pool();
  const account = { email: identityEmail(), password: password(), locale };
  const target = process.env.CARGO_TARGET_DIR || fileURLToPath(new URL('../target', import.meta.url));
  try {
    const result = execFileSync(
      process.env.PORTAL_E2E_BINARY || join(target, 'debug/oteryn-portal'),
      ['seed', 'account', JSON.stringify(account)],
      {
        env: { ...process.env, OTERYN_ENV: 'testing', OTERYN_DATABASE_URL: db.options.connectionString },
        stdio: ['ignore', 'pipe', 'ignore'],
      },
    );
    return { ...account, id: JSON.parse(result).accountId };
  } catch {
    throw new Error('The portal seed CLI could not create the disposable player');
  } finally {
    await db.end();
  }
}

export async function seedBreachedPassword(value) {
  await seedPasswordRange(value);
  const hash = createHash('sha1').update(value).digest('hex').toUpperCase();
  const result = await identityQuery(
    'UPDATE registration_password_ranges SET suffixes = ARRAY[$2::text], complete_xact = pg_current_xact_id() WHERE prefix = $1',
    [hash.slice(0, 5), hash.slice(5)],
  );
  expect(result.rowCount).toBe(1);
}

export async function waitForIdentityBudget(policies, remaining) {
  await expect.poll(async () => {
    const { rows } = await identityQuery(
      `SELECT COALESCE(max(count), 0) AS count FROM web_rate_limits
       WHERE limiter_name = ANY($1) AND window_start = date_trunc('minute', clock_timestamp())`,
      [policies],
    );
    return Number(rows[0].count);
  }, { timeout: 65_000, intervals: [1000] }).toBeLessThanOrEqual(remaining);
}

export async function clickPost(page, button, path) {
  const posted = page.waitForResponse(response => response.request().method() === 'POST'
    && new URL(response.url()).pathname === path);
  await button.click();
  return posted;
}

export async function localizedPage(page, locale, heading) {
  await expect(page.locator('html')).toHaveAttribute('lang', locale);
  await expect(page.getByRole('heading', { level: 1, name: heading, exact: true })).toBeVisible();
}

export async function signIn(page, locale, account) {
  await page.goto(`/${locale}/login`, { waitUntil: 'domcontentloaded' });
  await waitForLoginBudget();
  await submit(page, 'main form', { email: account.email, password: account.password });
}

export async function postWithPageCsrf(page, path, fields = {}) {
  return page.request.post(path, {
    headers: { origin: new URL(page.url()).origin },
    form: { ...fields, _csrf: await page.locator('main [name=_csrf]').first().inputValue() },
    maxRedirects: 0,
  });
}
