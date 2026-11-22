import { TOTP } from 'otpauth';
import { test, expect } from '../fixtures.mjs';
import { csrfRejected, login, submit } from '../helpers.mjs';
import { enrollMfa, query } from '../staff-rbac-helpers.mjs';
import { cookie, deviceCookie, expectSession, hash, seedAccount, seedEntity } from './auth-fixtures.mjs';

test('MFA rotates the session and remembered device, and rejects reused factors', async ({ page, context }, info) => {
  const lang = info.project.metadata.locale;
  const account = await seedAccount(lang);
  const seeded = await seedEntity('account-mfa', { accountId: account.id, password: account.password });
  const totp = new TOTP({
    secret: seeded.totpSecret,
    algorithm: seeded.algorithm,
    digits: seeded.digits,
    period: seeded.period,
  });
  const challenge = 'main form[action$="/mfa/challenge"]';
  const challengePath = new RegExp(`/${lang}/mfa/challenge$`);
  const home = new RegExp(`/${lang}/$`);

  await login(page, lang, account);
  await expect(page).toHaveURL(challengePath);
  const pending = await cookie(context);
  await expectSession(pending, null);
  await csrfRejected(page, `/${lang}/mfa/email`);

  const [{ last_step: lastStep }] = await query('SELECT last_step FROM account_mfa_totp WHERE account_id=$1', [account.id]);
  await expect.poll(() => Math.floor(Date.now() / (totp.period * 1000)), { timeout: 35_000 })
    .toBeGreaterThan(Number(lastStep));
  const code = totp.generate();
  await submit(page, challenge, { code, remember: true });
  await expect(page).toHaveURL(home);
  const authenticated = await cookie(context);
  expect(authenticated !== pending, 'MFA rotated the challenge session').toBe(true);
  await expectSession(pending);
  await expectSession(authenticated, account.id);

  const remembered = await cookie(context, deviceCookie);
  await login(page, lang, account);
  await expect(page).toHaveURL(home);
  await expectSession(await cookie(context), account.id);
  const replacement = await cookie(context, deviceCookie);
  expect(replacement !== remembered, 'trusted login rotated the device cookie').toBe(true);
  const devices = await query('SELECT token_hash FROM account_mfa_tokens WHERE account_id=$1 AND purpose=$2',
    [account.id, 'trusted']);
  expect(devices.length).toBe(1);
  expect(devices[0].token_hash.equals(hash(replacement)), 'only the replacement device is registered').toBe(true);

  await page.goto(`/${lang}/mfa`);
  await expect(page.locator('form[action$="/mfa/devices"]')).toHaveCount(1);
  await submit(page, 'form[action$="/mfa/devices"]');
  await expect(page.locator('form[action$="/mfa/devices"]')).toHaveCount(0);

  await login(page, lang, account);
  await expect(page).toHaveURL(challengePath);
  await submit(page, challenge, { code });
  await expect(page).toHaveURL(challengePath);
  await expect(page.locator('main [role=alert]')).toBeVisible();
  await expectSession(await cookie(context), null);

  const recovery = seeded.recoveryCodes[0];
  expect(Boolean(recovery), 'seeded recovery code exists').toBe(true);
  await submit(page, challenge, { code: recovery });
  await expect(page).toHaveURL(home);
  await expectSession(await cookie(context), account.id);

  await login(page, lang, account);
  await expect(page).toHaveURL(challengePath);
  await submit(page, challenge, { code: recovery });
  await expect(page).toHaveURL(challengePath);
  await expect(page.locator('main [role=alert]')).toBeVisible();
  await expectSession(await cookie(context), null);
});

test('MFA enrollment requires CSRF and a real authenticator confirmation', async ({ page, context }, info) => {
  const lang = info.project.metadata.locale;
  const account = await seedAccount(lang);
  await login(page, lang, account);
  const session = await cookie(context);
  await expectSession(session, account.id);
  await enrollMfa(page, lang, account);
  await expect(page.locator('main textarea[readonly]')).toHaveCount(8);
  const rows = await query('SELECT confirmed FROM account_mfa_totp WHERE account_id=$1', [account.id]);
  expect(rows).toEqual([{ confirmed: true }]);
  await expectSession(session);
});
