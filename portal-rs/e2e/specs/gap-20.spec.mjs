import * as OTPAuth from 'otpauth';
import {test, expect} from '../fixtures.mjs';
import {login, pool, register, submit} from '../helpers.mjs';

const operatorPath = '/admin/account-closure';
const closureState = page => page.locator('[data-closure-status]');

async function gameMode(accountId, mode) {
  const response = await fetch(`${process.env.PORTAL_E2E_GAME_CLOSURE_CONTROL_URL}/fault`, {
    method: 'POST',
    headers: {'content-type': 'application/json'},
    body: JSON.stringify({account_id: accountId, mode}),
  });
  expect(response.status).toBe(200);
}

async function confirmSecurity(page, lang, account) {
  await page.goto(`/${lang}/account/sudo?return_to=closure`);
  await submit(page, 'main form', {current_password: account.password});
}

test('the player sees pending and finalized requests stay visible to staff', async ({page, browser}, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const path = `/${lang}/account/privacy/delete`;
  const account = await register(page, lang);
  const db = pool();
  const staffContext = await browser.newContext({
    baseURL: info.project.use.baseURL ?? process.env.PORTAL_E2E_ORIGIN ?? 'https://localhost:18443',
    ignoreHTTPSErrors: true,
    locale: lang,
    viewport: info.project.use.viewport,
  });
  try {
    await login(page, lang, account);
    await page.goto(`/${lang}/account/privacy`);
    await page.locator(`main a[href="${path}"]`).click();
    await expect(page).toHaveURL(new RegExp(`${path}$`));
    await confirmSecurity(page, lang, account);
    await submit(page, 'main form', {current_password: account.password, confirmation: 'TERMINATE'});
    await expect(page).toHaveURL(/\/login$/);

    const {id} = (await db.query('SELECT id FROM accounts WHERE email=$1', [account.email])).rows[0];
    await gameMode(id, 'hold');
    await login(page, lang, account);
    await page.goto(path);
    await expect(closureState(page)).toHaveAttribute('data-closure-status', 'closure-scheduled');
    const reference = await closureState(page).locator('code').textContent();
    expect((await page.request.get(`${operatorPath}?reference=${reference}`)).status()).toBe(403);

    // Advance the disposable request's grace deadline through the same test clock as account-closure.spec.mjs.
    await db.query(`UPDATE account_terminations SET created_at=now()-interval '15 days',
                    scheduled_for=now()-interval '1 day' WHERE id=$1`, [reference]);
    await page.reload();
    await expect(closureState(page)).toHaveAttribute('data-closure-status', 'closure-pending');
    await expect(page.locator('[data-closure-game="closure-game-unavailable"]')).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);

    const staff = await staffContext.newPage();
    const operator = await register(staff, lang);
    await db.query(`INSERT INTO account_roles(account_id,role_id)
                    SELECT a.id,r.id FROM accounts a CROSS JOIN roles r
                    WHERE a.email=$1 AND r.key='platform_admin'`, [operator.email]);
    await login(staff, lang, operator);
    expect((await staff.goto(operatorPath)).status()).toBe(403);
    await staff.goto(`/${lang}/mfa`);
    await submit(staff, 'main form[action$="/mfa/enroll"]', {password: operator.password});
    const secret = await staff.locator('main .op-prose').innerText();
    const totp = new OTPAuth.TOTP({secret, algorithm: 'SHA1', digits: 6, period: 30});
    await submit(staff, 'main form[action$="/mfa/confirm"]', {password: operator.password, code: totp.generate()});
    const recovery = await staff.locator('main textarea[readonly]').first().textContent();
    await login(staff, lang, operator);
    await expect(staff).toHaveURL(/mfa\/challenge/);
    await submit(staff, 'main form[action$="/mfa/challenge"]', {code: recovery});

    await staff.goto('/admin');
    await staff.locator(`main nav a[href="${operatorPath}"]`).click();
    await staff.locator('input[name="reference"]').fill(reference);
    await staff.locator('main form button[type=submit]').click();
    await expect(closureState(staff)).toHaveAttribute('data-closure-status', 'closure-pending');
    expect((await staff.request.get(`${operatorPath}?reference=invalid`)).status()).toBe(422);

    await gameMode(id, 'valid');
    await expect.poll(async () => {
      await staff.reload();
      return closureState(staff).getAttribute('data-closure-status');
    }, {timeout: 90_000}).toBe('closure-finalized');
    await expect.poll(async () => {
      await staff.reload();
      return staff.locator('[data-closure-game="closure-game-finalized"]').count();
    }, {timeout: 90_000}).toBe(1);
    expect(await staff.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    expect((await page.goto(path)).status()).toBe(403);
  } finally {
    await staffContext.close();
    await db.end();
  }
});

test('a cancelled request remains visible after the player returns to Privacy', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const path = `/${lang}/account/privacy/delete`;
  const account = await register(page, lang);
  await login(page, lang, account);
  await confirmSecurity(page, lang, account);
  await submit(page, 'main form', {current_password: account.password, confirmation: 'TERMINATE'});
  await login(page, lang, account);
  await page.goto(path);
  const cancel = `${path}/cancel`;
  const csrf = await page.locator('main input[name="_csrf"]').inputValue();
  const stale = await page.request.post(cancel, {
    form: {_csrf: csrf, current_password: account.password},
    headers: {origin: new URL(page.url()).origin},
    maxRedirects: 0,
  });
  expect(stale.status()).toBe(403);
  await confirmSecurity(page, lang, account);
  await submit(page, `main form[action="${cancel}"]`, {current_password: account.password});
  await expect(closureState(page)).toHaveAttribute('data-closure-status', 'closure-cancelled');
  await page.goto(`/${lang}/account/privacy`);
  await page.locator(`main a[href="${path}"]`).click();
  await expect(closureState(page)).toHaveAttribute('data-closure-status', 'closure-cancelled');
  await expect(page.locator('[data-closure-game]')).toHaveCount(0);
});
