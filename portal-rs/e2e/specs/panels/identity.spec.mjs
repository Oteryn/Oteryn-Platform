import { createHash, randomBytes, randomUUID } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { request as httpsRequest } from 'node:https';
import * as OTPAuth from 'otpauth';
import { test, expect } from '../../fixtures.mjs';
import { csrfRejected, login, pool, submit } from '../../helpers.mjs';
import { seedAccount, seedEntity } from '../../security/auth-fixtures.mjs';

test.use({ trace: 'off', screenshot: 'off' });

async function post(page, path, token, fields) {
  return page.request.post(path, {
    form: { ...fields, _csrf: token },
    headers: { origin: new URL(page.url()).origin },
    maxRedirects: 0,
  });
}

const csrf = (page, path) => page.locator(`form[action="${path}"] [name=_csrf]`).first().inputValue();
async function codes(page) {
  const fields = page.locator('main textarea[readonly]');
  await expect(fields).toHaveCount(8);
  return fields.allTextContents();
}
const totp = async page => new OTPAuth.TOTP({ secret: await page.locator('main .op-prose').innerText() });
const codeAt = (factor, offset) => factor.generate({ timestamp: Date.now() + offset * 30_000 });

async function confirmRecentAuth(page, lang, password) {
  const path = `/${lang}/account/sudo`;
  await page.goto(path);
  const [response] = await Promise.all([
    page.waitForResponse(response => response.request().isNavigationRequest()
      && response.request().method() === 'POST' && new URL(response.url()).pathname === path),
    submit(page, 'main form', { current_password: password }),
  ]);
  expect(response.status()).toBe(302);
  expect(response.headers().location).toBe(`/${lang}/account/security`);
  await expect(page).toHaveURL(new RegExp(`/${lang}/password/change$`));
}

const usedSteps = new WeakMap();
const currentStep = () => Math.floor(Date.now() / 30_000);

async function submitProof(page, path, password, factor, { action, status = 200, afterCsrf } = {}) {
  const form = page.locator(`form[action="${path}"]`);
  if (action) await form.locator('[name=action]').selectOption(action);
  if (!afterCsrf) await csrfRejected(page, path, { action, password, code: '000000' });
  await form.locator('[name=password]').fill(password);
  const firstUnused = (usedSteps.get(factor) ?? -1) + 1;
  await expect.poll(() => currentStep() + 1, { timeout: 35_000 }).toBeGreaterThanOrEqual(firstUnused);
  const step = Math.max(currentStep(), firstUnused);
  const code = factor.generate({ timestamp: step * 30_000 });
  if (afterCsrf) {
    await csrfRejected(page, path, { action, password, code });
    await afterCsrf();
  }
  await form.locator('[name=code]').fill(code);
  const [response] = await Promise.all([
    page.waitForResponse(response => response.request().isNavigationRequest()
      && response.request().method() === 'POST' && new URL(response.url()).pathname === path),
    form.locator('button').click(),
  ]);
  expect(response.status()).toBe(status);
  usedSteps.set(factor, step);
}

async function expectProofRefusal(response, lang, status, { confirm = false } = {}) {
  expect(response.status()).toBe(status);
  const html = await response.text();
  const retryLabel = lang === 'pl' ? 'Wróć do ustawień bezpieczeństwa' : 'Back to security settings';
  expect(html.includes('role="alert"') && html.includes(retryLabel)).toBe(true);
  expect(html.includes(`href="/${lang}/mfa"`)).toBe(true);
  if (confirm) {
    expect(html.includes(`action="/${lang}/mfa/replace/confirm"`)).toBe(true);
    expect(html.includes('class="op-prose"')).toBe(false);
  }
}

async function seedRememberedFamily(db, accountId, clientId) {
  const id = randomUUID();
  const credential = `otd1.${randomBytes(32).toString('base64url')}`;
  const seeded = await db.query(
    `INSERT INTO device_sessions(id,account_id,client_id,enrollment_token_id,game_generation,native_generation,absolute_expires_at,idle_expires_at)
     SELECT $1,id,$2,$1::uuid::text,game_auth_generation,native_security_generation,
            extract(epoch FROM now())::bigint+3600,extract(epoch FROM now())::bigint+1800
     FROM accounts WHERE id=$3`, [id, clientId, accountId]);
  expect(seeded.rowCount).toBe(1);
  await db.query('INSERT INTO device_session_credentials(secret_hash,family_id,sequence) VALUES($1,$2,0)',
    [createHash('sha256').update(credential).digest(), id]);
  return { id, credential };
}

async function requestNativeTicket(credential, clientId) {
  const origin = new URL(process.env.PORTAL_E2E_NATIVE_DEVICE_ORIGIN);
  expect([origin.protocol, origin.hostname]).toEqual(['https:', 'localhost']);
  const body = JSON.stringify({ client_id: clientId, protocol_version: 1 });
  return new Promise((resolve, reject) => {
    const request = httpsRequest(new URL('/api/v1/game-auth/device-sessions/tickets', origin), {
      method: 'POST',
      ca: readFileSync(process.env.SSL_CERT_FILE),
      minVersion: 'TLSv1.3',
      headers: {
        authorization: `OterynDevice ${credential}`,
        'content-type': 'application/json',
        'content-length': Buffer.byteLength(body),
      },
    }, response => {
      const chunks = [];
      response.on('data', chunk => chunks.push(chunk));
      response.on('error', reject);
      response.on('end', () => {
        try { resolve({ status: response.statusCode, data: JSON.parse(Buffer.concat(chunks)) }); }
        catch { reject(new Error('Native ticket route returned invalid JSON')); }
      });
    });
    request.on('error', reject);
    request.setTimeout(10_000, () => request.destroy(new Error('Native ticket request timed out')));
    request.end(body);
  });
}

test('owner edits their profile and the server refuses foreign and invalid edits', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const path = `/${lang}/account/profile`;
  const anonymous = await page.request.get(path, { maxRedirects: 0 });
  expect(anonymous.headers().location).toBe(`/${lang}/login`);
  const account = await seedAccount(lang);
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
  await page.goto(`/${lang}/account/privacy`);
  const exportPath = `/${lang}/account/privacy/export`;
  const [stepUp] = await Promise.all([
    page.waitForResponse(response => new URL(response.url()).pathname === exportPath),
    page.locator(`[data-panel="player"] a[href="${exportPath}"]`).click(),
  ]);
  expect(stepUp.status()).toBe(303);
  expect(stepUp.headers().location).toBe(`/${lang}/account/sudo?return_to=export`);
  await expect(page).toHaveURL(new RegExp(`/${lang}/account/sudo\\?return_to=export$`));
  const [download] = await Promise.all([
    page.waitForEvent('download'),
    submit(page, 'main form', { current_password: account.password }),
  ]);
  expect(download.suggestedFilename()).toBe('account-data.json');
  expect((await page.request.get(exportPath)).status()).toBe(200);
  const data = JSON.parse(readFileSync(await download.path(), 'utf8'));
  expect(data.account.id).toBe(account.id);
  expect(data.account.display_name).toBe('Żółty Lis');
  await page.goto(path);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
});

test('owner replaces and removes TOTP and rotates one-time recovery codes', async ({ page, browser }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const mfa = `/${lang}/mfa`;
  const manage = `${mfa}/manage`;
  const replacement = `${mfa}/replace/confirm`;
  const account = await seedAccount(lang);
  const password = account.password;
  const anonymous = await page.request.get(mfa, { maxRedirects: 0 });
  expect(anonymous.headers().location).toBe(`/${lang}/login`);
  await login(page, lang, account);
  await page.goto(`/${lang}/account/privacy`);
  await page.locator(`a[href="${mfa}"]`).first().click();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
  await submit(page, 'main form', { password });
  const original = await totp(page);
  await submitProof(page, `${mfa}/confirm`, password, original);
  const originalCodes = await codes(page);
  expect(originalCodes.length).toBe(8);

  async function signInWithRecovery(code, screen = page) {
    await login(screen, lang, account);
    await expect(screen).toHaveURL(/mfa\/challenge/);
    await submit(screen, 'form[action$="/challenge"]', { code });
    await expect(screen).not.toHaveURL(/mfa\/challenge/);
    await screen.goto(mfa);
  }

  await signInWithRecovery(originalCodes[0]);
  await expect(page.locator('[data-recovery-count]')).toHaveText('7');
  const currentCodeLabel = lang === 'pl' ? 'Kod z obecnej aplikacji uwierzytelniającej' : 'Current authenticator code';
  await expect(page.getByLabel(currentCodeLabel)).toBeVisible();
  const token = await csrf(page, manage);
  const fields = { action: 'replace', password, code: 'invalid' };
  await csrfRejected(page, manage, fields);
  await expectProofRefusal(await post(page, manage, token, fields), lang, 400);
  await expectProofRefusal(await post(page, manage, token, {
    ...fields, password: 'wrong', code: codeAt(original, 0),
  }), lang, 403);
  const peer = await browser.newContext({
    baseURL: new URL(page.url()).origin, ignoreHTTPSErrors: true,
    locale: lang, viewport: info.project.use.viewport,
  });
  const db = pool();
  let next;
  try {
    const other = await peer.newPage();
    await signInWithRecovery(originalCodes[1], other);
    await expect(other.locator('[data-recovery-count]')).toHaveText('6');
    await submitProof(page, manage, password, original, { action: 'replace' });
    const newCodeLabel = lang === 'pl' ? 'Kod z nowej aplikacji uwierzytelniającej' : 'New authenticator code';
    await expect(page.getByLabel(newCodeLabel)).toBeVisible();
    next = await totp(page);
    const replacementToken = await csrf(page, replacement);
    await expectProofRefusal(await post(page, replacement, replacementToken, {
      password, code: 'invalid',
    }), lang, 400, { confirm: true });
    await expectProofRefusal(await post(page, replacement, replacementToken, {
      password, code: codeAt(next, -3),
    }), lang, 403, { confirm: true });
    await expectProofRefusal(await post(page, replacement, replacementToken, {
      password: 'wrong', code: codeAt(next, 0),
    }), lang, 403, { confirm: true });
    const state = async () => (await db.query(
      `SELECT generation, last_step, replacement_expires_at,
              (SELECT count(*)::int FROM account_recovery_codes WHERE account_id=$1) AS remaining
       FROM account_mfa_totp WHERE account_id=$1`, [account.id])).rows[0];
    const before = await state();
    expect(before.replacement_expires_at).not.toBeNull();
    await submitProof(page, replacement, password, next, {
      afterCsrf: async () => {
        expect(await state()).toEqual(before);
        expect((await other.request.get(mfa, { maxRedirects: 0 })).status()).toBe(200);
      },
    });
    const revoked = await other.request.get(mfa, { maxRedirects: 0 });
    expect(revoked.status()).toBe(302);
    expect(revoked.headers().location).toBe(`/${lang}/login`);
    expect((await state()).generation).not.toBe(before.generation);
  } finally {
    await db.end();
    await peer.close();
  }
  const replacementCodes = await codes(page);
  expect(replacementCodes.length).toBe(8);

  await signInWithRecovery(replacementCodes[0]);
  await submitProof(page, manage, password, next, { action: 'rotate_codes' });
  const rotatedCodes = await codes(page);
  expect(rotatedCodes.length).toBe(8);
  expect(rotatedCodes.some(code => replacementCodes.includes(code))).toBe(false);
  await login(page, lang, account);
  await submit(page, 'form[action$="/challenge"]', { code: replacementCodes[1] });
  await expect(page).toHaveURL(/mfa\/challenge/);
  await submit(page, 'form[action$="/challenge"]', { code: rotatedCodes[0] });
  await expect(page).not.toHaveURL(/mfa\/challenge/);
  await page.goto(mfa);
  await expect(page.locator('[data-recovery-count]')).toHaveText('7');
  await submitProof(page, manage, password, next, { action: 'remove', status: 302 });
  await login(page, lang, account);
  await expect(page).not.toHaveURL(/mfa\/challenge/);
});

test('native device revocation requires recent authentication and family ownership', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const mfa = `/${lang}/mfa`;
  const action = `${mfa}/native-devices`;
  const account = await seedAccount(lang);
  const clientId = process.env.PORTAL_E2E_NATIVE_DEVICE_CLIENT;
  await seedEntity('oauth-native-client', { clientId });
  const db = pool();
  try {
    const owned = await seedRememberedFamily(db, account.id, clientId);
    const foreign = await seedRememberedFamily(db, (await seedAccount(lang)).id, clientId);
    const used = await requestNativeTicket(owned.credential, clientId);
    expect(used.status).toBe(200);
    expect(used.data.family_id).toBe(owned.id);
    expect(Boolean(used.data.ticket && used.data.device_credential)).toBe(true);
    expect(used.data.device_credential !== owned.credential).toBe(true);
    await login(page, lang, account);
    await page.goto(mfa);
    await expect(page.locator(`input[value="${owned.id}"]`)).toHaveCount(1);
    await expect(page.locator(`input[value="${foreign.id}"]`)).toHaveCount(0);
    const token = await csrf(page, action);
    expect((await post(page, action, token, { family: owned.id })).status()).toBe(403);
    await confirmRecentAuth(page, lang, account.password);
    await page.goto(mfa);
    const fresh = await csrf(page, action);
    await csrfRejected(page, action, { family: owned.id });
    expect((await post(page, action, fresh, { family: 'malformed' })).status()).toBe(422);
    expect((await post(page, action, fresh, { family: foreign.id })).status()).toBe(403);
    await page.locator(`form[action="${action}"] button`).click();
    await expect(page.locator(`input[value="${owned.id}"]`)).toHaveCount(0);
    const refused = await requestNativeTicket(used.data.device_credential, clientId);
    expect(refused.status).toBe(401);
    expect(refused.data.error === 'device_authorization_unavailable' && Object.keys(refused.data).length === 1).toBe(true);
    const tickets = await db.query('SELECT count(*)::int AS count FROM game_login_tickets WHERE device_family_id=$1', [owned.id]);
    expect(tickets.rows[0].count).toBe(1);
    const state = await db.query("SELECT id, revoked_at IS NOT NULL AS revoked FROM device_sessions WHERE id=ANY($1::uuid[])", [[owned.id, foreign.id]]);
    expect(Object.fromEntries(state.rows.map(row => [row.id, row.revoked]))).toEqual({ [owned.id]: true, [foreign.id]: false });
  } finally {
    await db.end();
  }
});

test.describe('native settings flag disabled', () => {
  test.use({ baseURL: process.env.PORTAL_E2E_NATIVE_PANEL_OFF_ORIGIN });

  test('owner cannot see or post native revocation with the flag off', async ({ page }, info) => {
    const lang = info.project.metadata.locale;
    const mfa = `/${lang}/mfa`;
    const action = `${mfa}/native-devices`;
    const account = await seedAccount(lang);
    const db = pool();
    try {
      const family = await seedRememberedFamily(db, account.id, process.env.PORTAL_E2E_NATIVE_DEVICE_CLIENT);
      await login(page, lang, account);
      await confirmRecentAuth(page, lang, account.password);
      expect((await page.goto(mfa)).status()).toBe(200);
      expect(new URL(page.url()).origin).toBe(process.env.PORTAL_E2E_NATIVE_PANEL_OFF_ORIGIN);
      const heading = lang === 'pl' ? 'Zapamiętane klienty gry' : 'Remembered game clients';
      await expect(page.getByRole('heading', { name: heading })).toHaveCount(0);
      await expect(page.locator(`form[action="${action}"]`)).toHaveCount(0);
      await expect(page.locator(`input[value="${family.id}"]`)).toHaveCount(0);
      const token = await csrf(page, `${mfa}/enroll`);
      expect((await post(page, action, token, { family: family.id })).status()).toBe(404);
      const result = await db.query('SELECT revoked_at FROM device_sessions WHERE id=$1', [family.id]);
      expect(result.rows[0].revoked_at).toBeNull();
    } finally {
      await db.end();
    }
  });
});
