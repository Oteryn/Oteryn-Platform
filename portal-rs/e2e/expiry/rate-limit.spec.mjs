import { createHash } from 'node:crypto';
import { waitForLoginBudget } from '../helpers.mjs';
import { expect, formPost, one, redirected, sweep, test, text } from './fixtures.mjs';

function loginBucket(email) {
  const digest = createHash('sha256');
  for (const part of ['identity-login', email, '127.0.0.2']) {
    const length = Buffer.alloc(8);
    length.writeBigUInt64BE(BigInt(Buffer.byteLength(part)));
    digest.update(length).update(part);
  }
  return digest.digest();
}

test('the exhausted login window stops charging new requests after expiry', async ({ page, lang, db, seed }) => {
  const account = await seed();
  const path = `/${lang}/login`;
  expect((await page.goto(path)).status()).toBe(200);
  const login = password => formPost(page, 'main form', { email: account.email, password },
    { 'x-portal-e2e-login-peer': '127.0.0.2' });
  const sessions = () => one(db, 'SELECT count(*)::int AS count FROM web_sessions WHERE account_id=$1', [account.id]);
  const key = loginBucket(account.email);
  const buckets = async () => (await db.query(`SELECT window_start,count::int AS count
    FROM web_rate_limits WHERE limiter_name='identity-login' AND key_hash=$1 ORDER BY window_start`, [key])).rows;
  await waitForLoginBudget(7);
  for (let count = 1; count <= 5; count += 1) {
    redirected(await login('Incorrect local password!'), 302, path);
  }
  const blocked = await login(account.password);
  expect(blocked.status()).toBe(429);
  expect(blocked.headers()['content-language']).toBe(lang);
  expect(blocked.headers()['cache-control']).toBe('no-store, private');
  expect(Number(blocked.headers()['retry-after'])).toBeGreaterThan(0);
  expect(Number(blocked.headers()['retry-after'])).toBeLessThanOrEqual(60);
  expect(await blocked.text()).toContain(text(lang, 'Too many requests', 'Zbyt wiele żądań'));
  expect(await sessions()).toEqual({ count: 0 });
  const spent = await buckets();
  expect(spent).toEqual([{ window_start: expect.any(Date), count: 6 }]);

  // Move the real exhausted bucket back exactly one window. The next production
  // request sees the same state that it would see on the next minute boundary.
  const past = await one(db, `UPDATE web_rate_limits SET window_start=date_trunc('minute',clock_timestamp())-interval '60 seconds'
    WHERE limiter_name='identity-login' AND key_hash=$1 AND window_start=$2 RETURNING window_start`, [key, spent[0].window_start]);
  redirected(await login(account.password), 302, `/${lang}/`);
  expect(await sessions()).toEqual({ count: 1 });
  expect((await page.goto(`/${lang}/account/sessions`)).status()).toBe(200);
  const recovered = await buckets();
  expect(recovered).toEqual([{ ...past, count: 6 }, { window_start: expect.any(Date), count: 1 }]);

  // Request-window expiry and the worker's one-hour storage retention differ.
  await one(db, `UPDATE web_rate_limits SET window_start=date_trunc('minute',clock_timestamp())-interval '61 minutes'
    WHERE limiter_name='identity-login' AND key_hash=$1 AND window_start=$2 RETURNING window_start`, [key, past.window_start]);
  await sweep(db, 'cleanup.rate-limits', buckets, [recovered[1]]);
});
