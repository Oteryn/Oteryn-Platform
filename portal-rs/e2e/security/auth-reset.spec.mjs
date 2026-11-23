import { test, expect } from '../fixtures.mjs';
import { csrfRejected, login, mailLink, seedPasswordRange } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { cookie, emailAddress, expectSession, postForm, seedAccount } from './auth-fixtures.mjs';

test.use({ trace: 'off', screenshot: 'off' });

async function requestReset(page, lang, email) {
  const route = `/${lang}/forgot-password`;
  expect((await page.goto(route)).status()).toBe(200);
  await csrfRejected(page, route, { email });
  const response = await postForm(page, 'main form', { email });
  expect(response.status()).toBe(302);
  expect(response.headers().location).toBe(route);
  expect((await page.goto(route)).status()).toBe(200);
  const notice = page.locator('main [role=status]');
  await expect(notice).toBeVisible();
  return notice.innerText();
}

test('reset request has a generic localized confirmation and the token is bound, expiring and single-use', async ({ page, context }, info) => {
  const lang = info.project.metadata.locale;
  const owner = await seedAccount(lang);
  const other = await seedAccount(lang);
  const expired = await seedAccount(lang);
  const known = await requestReset(page, lang, owner.email);
  const unknown = await requestReset(page, lang, emailAddress());
  expect(known).toBe(unknown);
  expect(known).toBe(lang === 'pl'
    ? 'Jeśli konto z tym adresem istnieje, wysłaliśmy link do zresetowania hasła.'
    : 'If an account exists for that email, a password reset link has been sent.');

  const pattern = /https:\/\/localhost:18443\/(?:pl|en)\/reset-password\/[a-f0-9]+/;
  const link = await mailLink(page, owner.email, pattern);
  await requestReset(page, lang, other.email);
  const otherLink = await mailLink(page, other.email, pattern);
  expect(otherLink !== link, 'both accounts received distinct reset tokens').toBe(true);
  await requestReset(page, lang, expired.email);
  const expiredLink = await mailLink(page, expired.email, pattern);
  const [advanced] = await query(
    "UPDATE account_password_resets SET expires_at=clock_timestamp()-interval '1 second' WHERE account_id=$1 RETURNING account_id",
    [expired.id],
  );
  expect(advanced?.account_id).toBe(expired.id);

  const next = await seedPasswordRange();
  async function complete(url, email, accepted) {
    expect((await page.goto(url)).status()).toBe(200);
    const values = { email, password: next, password_confirmation: next };
    await csrfRejected(page, `/${lang}/reset-password`, values);
    const response = await postForm(page, 'main form', values);
    expect(response.status()).toBe(302);
    const location = response.headers().location;
    expect(location === (accepted ? `/${lang}/login` : new URL(url).pathname),
      'reset returned the expected destination').toBe(true);
    if (!accepted) {
      expect((await page.goto(location)).status()).toBe(200);
      await expect(page.locator('main [role=alert]')).toBeVisible();
    }
  }

  async function resetState(account) {
    const [row] = await query(`
      SELECT a.password_hash,r.token_hash,r.used_at,r.expires_at,
             r.expires_at>clock_timestamp() AS live
      FROM accounts a JOIN account_password_resets r ON r.account_id=a.id
      WHERE a.id=$1
    `, [account.id]);
    expect(row.live).toBe(true);
    expect(row.used_at).toBeNull();
    return row;
  }
  const accounts = [owner, other];
  const before = await Promise.all(accounts.map(resetState));
  await complete(link, other.email, false);
  const after = await Promise.all(accounts.map(resetState));
  for (const [index, row] of before.entries()) {
    expect(row.password_hash === after[index].password_hash, 'password was preserved').toBe(true);
    expect(row.token_hash.equals(after[index].token_hash), 'live reset token was preserved').toBe(true);
    expect(after[index].expires_at).toEqual(row.expires_at);
  }
  await complete(expiredLink, expired.email, false);
  await complete(link, owner.email, true);
  await complete(link, owner.email, false);
  const [{ used }] = await query('SELECT used_at IS NOT NULL AS used FROM account_password_resets WHERE account_id=$1', [owner.id]);
  expect(used).toBe(true);
  await login(page, lang, { ...owner, password: next });
  await expectSession(await cookie(context), owner.id);
});
