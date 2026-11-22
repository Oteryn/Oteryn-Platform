import { TOTP } from 'otpauth';
import { test, expect } from '../fixtures.mjs';
import { csrfRejected, login, submit } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { cookie, deviceCookie, expectSession, hash, otherBrowser, postForm, seedAccount, seedEntity } from './auth-fixtures.mjs';

test.use({ trace: 'off', screenshot: 'off' });

test('MFA rotates the session and remembered device, and rejects reused factors', async ({ page, context, browser }, info) => {
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
  const fresh = await otherBrowser(browser, info);
  try {
    const replay = await fresh.newPage();
    await login(replay, lang, account);
    await expect(replay).toHaveURL(challengePath);
    await expectSession(await cookie(fresh), null);

    const stepMs = totp.period * 1000;
    const timestamp = Date.now();
    const code = totp.generate({ timestamp });
    await submit(page, challenge, { code, remember: true });
    await expect(page).toHaveURL(home);

    const response = await postForm(replay, challenge, { code });
    expect(Math.floor(Date.now() / stepMs) - Math.floor(timestamp / stepMs),
      'replayed TOTP is still in the server acceptance window').toBeLessThanOrEqual(1);
    expect(response.status()).toBe(302);
    expect(response.headers().location).toBe(`/${lang}/mfa/challenge`);
    await replay.goto(response.headers().location);
    await expect(replay.locator('main [role=alert]')).toBeVisible();
    await expectSession(await cookie(fresh), null);
  } finally {
    await fresh.close();
  }

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
  await page.goto(`/${lang}/mfa`);
  await submit(page, 'form[action$="/mfa/enroll"]', { password: account.password });
  const totp = new TOTP({ secret: await page.locator('main .op-prose').innerText() });
  const confirm = 'form[action$="/mfa/confirm"]';
  // Include the adjacent step on either side of the acceptance window for a clock tick.
  const now = Date.now();
  const nearby = new Set([-2, -1, 0, 1, 2].map(offset =>
    totp.generate({ timestamp: now + offset * totp.period * 1000 })));
  const wrong = ['000000', '111111', '222222', '333333', '444444', '555555']
    .find(candidate => !nearby.has(candidate));
  const refused = await postForm(page, confirm, { password: account.password, code: wrong });
  expect(refused.status()).toBe(302);
  expect(refused.headers().location).toBe(`/${lang}/mfa`);
  expect(await query('SELECT confirmed FROM account_mfa_totp WHERE account_id=$1', [account.id]))
    .toEqual([{ confirmed: false }]);
  expect(await query('SELECT 1 FROM account_recovery_codes WHERE account_id=$1', [account.id]))
    .toHaveLength(0);
  await submit(page, confirm, { password: account.password, code: totp.generate() });
  await expect(page.locator('main textarea[readonly]')).toHaveCount(8);
  const rows = await query('SELECT confirmed FROM account_mfa_totp WHERE account_id=$1', [account.id]);
  expect(rows).toEqual([{ confirmed: true }]);
  await expectSession(session);
});
