import * as OTPAuth from 'otpauth';
import { expect, test } from '../fixtures.mjs';
import { login, register, submit } from '../helpers.mjs';
import { audit, loginWithRecoveryCode, query, role } from '../staff-rbac-helpers.mjs';

test('registration renders and a verified player can log in', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const account = await register(page, lang);
  await login(page, lang, account);
  const overview = await page.goto('/account');
  expect(overview.status()).toBe(200);
  await expect(page).toHaveURL(/\/account\/?$/);
  await expect(page.locator('main')).toBeVisible();
});

test('MFA staff reaches the landing page and creates an audited role', async ({ page, context }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const staff = await register(page, lang);
  // Bootstrap the first staff actor the same way as the full browser harness.
  await query(
    "INSERT INTO account_roles(account_id, role_id) SELECT a.id, r.id FROM accounts a CROSS JOIN roles r WHERE a.email = $1 AND r.key = 'platform_admin'",
    [staff.email],
  );
  await login(page, lang, staff);
  expect((await page.goto(`/admin?lang=${lang}`)).status()).toBe(403);

  await page.goto(`/${lang}/mfa`);
  await submit(page, 'main form[action$="/mfa/enroll"]', { password: staff.password });
  const totp = new OTPAuth.TOTP({
    secret: await page.locator('main .op-prose').innerText(),
    algorithm: 'SHA1',
    digits: 6,
    period: 30,
  });
  await submit(page, 'main form[action$="/mfa/confirm"]', { password: staff.password, code: totp.generate() });
  const recovery = page.locator('textarea[readonly]').first();
  await expect(recovery).toBeVisible();
  const code = await recovery.inputValue();

  await context.clearCookies();
  await loginWithRecoveryCode(page, lang, staff, code);
  expect((await page.goto(`/admin?lang=${lang}`)).status()).toBe(200);
  await expect(page.locator('main')).toBeVisible();
  const created = await role(page);
  await audit(page, 'rbac.role.created', created);
});
