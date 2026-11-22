import { randomUUID } from 'node:crypto';
import * as OTPAuth from 'otpauth';
import { test, expect } from '../../fixtures.mjs';
import { csrfRejected, login, pool, register, submit } from '../../helpers.mjs';

async function post(page, path, token, fields) {
  return page.request.post(path, {
    form: { ...fields, _csrf: token },
    headers: { origin: new URL(page.url()).origin },
    maxRedirects: 0,
  });
}

const csrf = (page, path) => page.locator(`form[action="${path}"] [name=_csrf]`).first().inputValue();
const codes = page => page.locator('textarea[readonly]').allTextContents();
const totp = async page => new OTPAuth.TOTP({ secret: await page.locator('.op-prose').innerText() });
const codeAt = (factor, offset) => factor.generate({ timestamp: Date.now() + offset * 30_000 });

test('owner edits their profile and the server refuses foreign and invalid edits', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const path = `/${lang}/account/profile`;
  const anonymous = await page.request.get(path, { maxRedirects: 0 });
  expect(anonymous.headers().location).toBe(`/${lang}/login`);
  const account = await register(page, lang);
  await login(page, lang, account);
  await page.goto(`/${lang}/account/privacy`);
  await expect(page.locator(`a[href="${path}"]`).first()).toBeVisible();
  await page.locator(`a[href="${path}"]`).first().click();
  const token = await csrf(page, path);
  expect((await post(page, path, token, { display_name: 'x'.repeat(65) })).status()).toBe(400);
  expect((await post(page, path, token, { display_name: 'Other', account_id: randomUUID() })).status()).toBe(422);
  await submit(page, 'main form', { display_name: 'Żółty Lis' });
  await expect(page.locator('[name=display_name]')).toHaveValue('Żółty Lis');
  await page.reload();
  await expect(page.locator('[name=display_name]')).toHaveValue('Żółty Lis');
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
});

test('owner replaces and removes TOTP and rotates one-time recovery codes', async ({ page }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const mfa = `/${lang}/mfa`;
  const manage = `${mfa}/manage`;
  const replacement = `${mfa}/replace/confirm`;
  const account = await register(page, lang);
  const password = account.password;
  const anonymous = await page.request.get(mfa, { maxRedirects: 0 });
  expect(anonymous.headers().location).toBe(`/${lang}/login`);
  await login(page, lang, account);
  await page.goto(mfa);
  await submit(page, 'main form', { password });
  const original = await totp(page);
  await submit(page, `form[action="${mfa}/confirm"]`, { password, code: codeAt(original, -1) });
  const originalCodes = await codes(page);
  expect(originalCodes).toHaveLength(8);

  async function signInWithRecovery(code) {
    await login(page, lang, account);
    await expect(page).toHaveURL(/mfa\/challenge/);
    await submit(page, 'form[action$="/challenge"]', { code });
    await page.goto(mfa);
  }

  await signInWithRecovery(originalCodes[0]);
  await expect(page.locator('[data-recovery-count]')).toHaveText('7');
  const token = await csrf(page, manage);
  const fields = { action: 'replace', password, code: 'invalid' };
  await csrfRejected(page, manage, fields);
  expect((await post(page, manage, token, fields)).status()).toBe(400);
  expect((await post(page, manage, token, { ...fields, password: 'wrong', code: codeAt(original, 0) })).status()).toBe(403);
  await page.locator('[name=action]').selectOption('replace');
  await submit(page, `form[action="${manage}"]`, { password, code: codeAt(original, 0) });
  const next = await totp(page);
  const replacementToken = await csrf(page, replacement);
  expect((await post(page, replacement, replacementToken, { password, code: 'invalid' })).status()).toBe(400);
  expect((await post(page, replacement, replacementToken, { password: 'wrong', code: codeAt(next, -1) })).status()).toBe(403);
  await submit(page, `form[action="${replacement}"]`, { password, code: codeAt(next, -1) });
  const replacementCodes = await codes(page);
  expect(replacementCodes).toHaveLength(8);

  await signInWithRecovery(replacementCodes[0]);
  await page.locator('[name=action]').selectOption('rotate_codes');
  await submit(page, `form[action="${manage}"]`, { password, code: codeAt(next, 0) });
  const rotatedCodes = await codes(page);
  expect(rotatedCodes).toHaveLength(8);
  expect(rotatedCodes).not.toEqual(replacementCodes);
  await login(page, lang, account);
  await submit(page, 'form[action$="/challenge"]', { code: replacementCodes[1] });
  await expect(page).toHaveURL(/mfa\/challenge/);
  await submit(page, 'form[action$="/challenge"]', { code: rotatedCodes[0] });
  await page.goto(mfa);
  await expect(page.locator('[data-recovery-count]')).toHaveText('7');
  await page.locator('[name=action]').selectOption('remove');
  await submit(page, `form[action="${manage}"]`, { password, code: codeAt(next, 1) });
  await login(page, lang, account);
  await expect(page).not.toHaveURL(/mfa\/challenge/);
});

test('native device revocation requires recent authentication and family ownership', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const mfa = `/${lang}/mfa`;
  const action = `${mfa}/native-devices`;
  const account = await register(page, lang);
  const owned = randomUUID();
  const foreign = randomUUID();
  const db = pool();
  try {
    const accounts = await db.query("SELECT id,email,password_hash FROM accounts WHERE email=$1", [account.email]);
    const owner = accounts.rows[0];
    const other = await db.query("INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES($1,$2,'active',$3,now()) RETURNING id", [`${foreign}@local.test`, owner.password_hash, lang]);
    for (const [family, id] of [[owned, owner.id], [foreign, other.rows[0].id]]) {
      await db.query("INSERT INTO device_sessions(id,account_id,client_id,enrollment_token_id,game_generation,native_generation,absolute_expires_at,idle_expires_at) VALUES($1,$2,$3,$4,0,1,extract(epoch FROM now())::bigint+3600,extract(epoch FROM now())::bigint+1800)", [family, id, randomUUID(), family]);
    }
    await login(page, lang, account);
    await page.goto(mfa);
    await expect(page.locator(`input[value="${owned}"]`)).toHaveCount(1);
    await expect(page.locator(`input[value="${foreign}"]`)).toHaveCount(0);
    const token = await csrf(page, action);
    expect((await post(page, action, token, { family: owned })).status()).toBe(403);
    await page.goto(`/${lang}/account/sudo`);
    await submit(page, 'main form', { current_password: account.password });
    await page.goto(mfa);
    const fresh = await csrf(page, action);
    await csrfRejected(page, action, { family: owned });
    expect((await post(page, action, fresh, { family: 'malformed' })).status()).toBe(422);
    expect((await post(page, action, fresh, { family: foreign })).status()).toBe(403);
    await page.locator(`form[action="${action}"] button`).click();
    await expect(page.locator(`input[value="${owned}"]`)).toHaveCount(0);
    const state = await db.query("SELECT id, revoked_at IS NOT NULL AS revoked FROM device_sessions WHERE id=ANY($1::uuid[])", [[owned, foreign]]);
    expect(Object.fromEntries(state.rows.map(row => [row.id, row.revoked]))).toEqual({ [owned]: true, [foreign]: false });
  } finally {
    await db.end();
  }
});
