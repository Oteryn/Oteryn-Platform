import { login, logout, submit, mailLink, password, seedPasswordRange, waitForLoginBudget } from '../helpers.mjs';
import { enrollMfa, unique } from '../staff-rbac-helpers.mjs';
import { after, before, digest, expect, formPost, one, redirected, sweep, test, text } from './fixtures.mjs';

async function cookieDigest(context, name) {
  const cookie = (await context.cookies()).find(cookie => cookie.name === name);
  expect(Boolean(cookie), 'portal issued the expected cookie').toBe(true);
  return digest(cookie.value);
}

const sessionDigest = context => cookieDigest(context, '__Host-oteryn_session');
const loginTitle = lang => text(lang, 'Account security', 'Bezpieczeństwo konta');

for (const [kind, column] of [['idle', 'idle_expires_at'], ['absolute', 'absolute_expires_at']]) {
  test(`web session expires at its ${kind} deadline`, async ({ page, context, lang, db, seed }) => {
    const account = await seed();
    await login(page, lang, account);
    const hash = await sessionDigest(context);
    const path = `/${lang}/account/sessions`;
    const args = [hash, account.id];
    const move = seconds => one(db, `UPDATE web_sessions SET ${column}=clock_timestamp()+make_interval(secs=>$3)
      WHERE id_hash=$1 AND account_id=$2 RETURNING session_id`, [...args, seconds]);
    expect(await one(db, `SELECT
        extract(epoch FROM idle_expires_at-last_seen_at)::int AS idle,
        extract(epoch FROM absolute_expires_at-created_at)::int AS absolute
        FROM web_sessions WHERE id_hash=$1 AND account_id=$2`, args))
      .toEqual({ idle: 1800, absolute: 86400 });
    // A second browser login for this account must survive both the refused
    // bulk revoke and the worker's cleanup of the expired session.
    const sibling = await context.browser().newContext({
      baseURL: new URL(page.url()).origin, ignoreHTTPSErrors: true, locale: lang,
    });
    let siblingHash;
    try {
      const other = await sibling.newPage();
      await login(other, lang, account);
      siblingHash = await sessionDigest(sibling);
    } finally {
      await sibling.close();
    }
    await move(before);
    expect((await page.goto(path)).status()).toBe(200);
    await expect(page.locator('main h1')).toHaveText(text(lang, 'Web sessions', 'Sesje web'));
    const csrf = await page.locator('form[action$="/all"] [name=_csrf]').inputValue();

    await move(after);
    redirected(await page.request.get(path, { maxRedirects: 0 }), 303, `/${lang}/login`);
    const mutation = await page.request.post(`${path}/all`, {
      form: { _csrf: csrf }, headers: { origin: new URL(page.url()).origin }, maxRedirects: 0,
    });
    expect(mutation.status()).toBe(403);
    await page.goto(path);
    await expect(page.locator('main h1')).toHaveText(loginTitle(lang));
    await expect(page.locator('main form button')).toHaveText(text(lang, 'Sign in', 'Zaloguj się'));
    const expired = async () => (await db.query('SELECT session_id FROM web_sessions WHERE id_hash=$1', [hash])).rows;
    await sweep(db, 'cleanup.sessions', expired, []);
    expect(await one(db, `SELECT account_id FROM web_sessions
      WHERE id_hash=$1 AND idle_expires_at>clock_timestamp() AND absolute_expires_at>clock_timestamp()`,
      [siblingHash])).toEqual({ account_id: account.id });
  });
}

test('sudo expiration prevents export and account closure', async ({ page, context, lang, db, seed }) => {
  const account = await seed();
  await login(page, lang, account);
  await page.goto(`/${lang}/account/sudo`);
  await submit(page, 'main form', { current_password: account.password });
  const hash = await sessionDigest(context);
  const move = seconds => one(db, `UPDATE web_sessions SET sudo_until=clock_timestamp()+make_interval(secs=>$3)
    WHERE id_hash=$1 AND account_id=$2 RETURNING session_id`, [hash, account.id, seconds]);
  const state = () => one(db, `SELECT a.native_security_generation,
    (SELECT count(*)::int FROM account_terminations t WHERE t.account_id=a.id) AS terminations
    FROM accounts a WHERE a.id=$1`, [account.id]);
  const original = await state();
  const exportURL = `/${lang}/account/privacy/export`;
  const issued = await one(db, 'SELECT extract(epoch FROM sudo_until-updated_at)::float8 AS seconds FROM web_sessions WHERE id_hash=$1', [hash]);
  expect(issued.seconds).toBeGreaterThan(898);
  expect(issued.seconds).toBeLessThanOrEqual(900);
  expect((await page.goto(`/${lang}/account/privacy/delete`)).status()).toBe(200);
  await move(before);
  const exported = await page.request.get(exportURL);
  expect(exported.status()).toBe(200);
  expect((await exported.text()).includes(account.email)).toBe(true);

  await move(after);
  const refusal = await page.request.get(exportURL);
  expect(refusal.status()).toBe(403);
  expect((await refusal.text()).includes(account.email)).toBe(false);
  const closure = await formPost(page, 'main form', {
    current_password: account.password, confirmation: 'TERMINATE',
  });
  expect(closure.status()).toBe(403);
  expect((await closure.text()).includes(text(lang, 'Access denied', 'Brak dostępu'))).toBe(true);
  expect(await state()).toEqual(original);
});

test('the staff MFA session grant expires while the browser session stays live', async ({ page, context, lang, db, seed }) => {
  const account = await seed();
  expect((await db.query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'", [account.id])).rowCount).toBe(1);
  await login(page, lang, account);
  await enrollMfa(page, lang, account);
  const firstRecoveryCode = page.locator('textarea[readonly]').first();
  await expect(firstRecoveryCode).toBeVisible();
  const code = await firstRecoveryCode.inputValue();
  await login(page, lang, account);
  await expect(page).toHaveURL(new RegExp(`/${lang}/mfa/challenge$`));
  await submit(page, 'form[action$="/mfa/challenge"]', { code });
  const hash = await sessionDigest(context);
  const grant = await one(db, `SELECT t.token_hash,extract(epoch FROM t.expires_at-t.created_at)::int AS lifetime
    FROM web_sessions s JOIN account_mfa_tokens t ON t.token_hash=sha256(convert_to(s.data->>'mfa_grant','UTF8'))
    WHERE s.id_hash=$1 AND t.account_id=$2 AND t.purpose='grant'`, [hash, account.id]);
  expect(grant.lifetime).toBe(86400);
  const move = seconds => one(db, `UPDATE account_mfa_tokens SET expires_at=clock_timestamp()+make_interval(secs=>$2)
    WHERE token_hash=$1 RETURNING purpose`, [grant.token_hash, seconds]);

  await move(before);
  expect((await page.goto('/admin/roles')).status()).toBe(200);
  const role = unique();
  await move(after);
  const refused = await formPost(page, 'form:has(input[name=key])', { key: role, permissions: 'admin.access' },
    { 'accept-language': lang });
  expect(refused.status()).toBe(403);
  expect(await one(db, 'SELECT count(*)::int AS count FROM roles WHERE key=$1', [role])).toEqual({ count: 0 });
  expect((await page.reload()).status()).toBe(403);
  await expect(page.locator('main h1')).toHaveText(text(lang,
    'Multi-factor authentication required', 'Wymagane uwierzytelnianie wieloskładnikowe'));
  expect((await page.goto(`/${lang}/account/sessions`)).status()).toBe(200);
  const expired = async () => (await db.query('SELECT purpose FROM account_mfa_tokens WHERE token_hash=$1', [grant.token_hash])).rows;
  await sweep(db, 'cleanup.mfa', expired, []);
});

test('remembered browser device stops satisfying MFA after its deadline', async ({ page, context, lang, db, seed }) => {
  const account = await seed();
  await login(page, lang, account);
  await enrollMfa(page, lang, account);
  const firstRecoveryCode = page.locator('textarea[readonly]').first();
  await expect(firstRecoveryCode).toBeVisible();
  const code = await firstRecoveryCode.inputValue();
  await login(page, lang, account);
  await expect(page).toHaveURL(new RegExp(`/${lang}/mfa/challenge$`));
  await submit(page, 'form[action$="/mfa/challenge"]', { code, remember: true });

  const device = async () => cookieDigest(context, '__Host-oteryn_mfa_device');
  const read = hash => one(db, `SELECT extract(epoch FROM expires_at-created_at)::int AS lifetime
    FROM account_mfa_tokens WHERE token_hash=$1 AND account_id=$2 AND purpose='trusted'`, [hash, account.id]);
  const move = (hash, seconds) => one(db, `UPDATE account_mfa_tokens SET expires_at=clock_timestamp()+make_interval(secs=>$3)
    WHERE token_hash=$1 AND account_id=$2 AND purpose='trusted' RETURNING purpose`, [hash, account.id, seconds]);
  const first = await device();
  expect(await read(first)).toEqual({ lifetime: 30 * 86400 });

  const start = async () => {
    await logout(page, lang);
    expect((await page.goto(`/${lang}/login`)).status()).toBe(200);
    await waitForLoginBudget();
  };
  const credentials = { email: account.email, password: account.password };
  await start();
  await move(first, before);
  redirected(await formPost(page, 'main form', credentials), 302, `/${lang}/`);
  expect((await page.goto(`/${lang}/account/sessions`)).status()).toBe(200);
  const rotated = await device();
  expect(rotated.equals(first), 'trusted proof rotates after use').toBe(false);

  await start();
  await move(rotated, after);
  redirected(await formPost(page, 'main form', credentials), 302, `/${lang}/mfa/challenge`);
  const challenge = await page.goto(`/${lang}/mfa/challenge`);
  expect(challenge.status()).toBe(200);
  await expect(page.locator('form[action$="/mfa/challenge"] button').first()).toHaveText(
    text(lang, 'Verify code', 'Potwierdź kod'));
  redirected(await page.request.get(`/${lang}/account/sessions`, { maxRedirects: 0 }), 303, `/${lang}/login`);
  expect((await device()).equals(rotated), 'expired device received no replacement').toBe(true);
  const expired = async () => (await db.query('SELECT purpose FROM account_mfa_tokens WHERE token_hash=$1', [rotated])).rows;
  await sweep(db, 'cleanup.mfa', expired, []);
});

for (const expired of [false, true]) {
  test(`verification link ${expired ? 'refuses a stale confirmation after expiry' : 'accepts a live confirmation before expiry'}`, async ({ page, lang, db }) => {
    const email = `expiry-${crypto.randomUUID()}@local.test`;
    const pass = password();
    await seedPasswordRange(pass);
    expect((await page.goto(`/${lang}/register`)).status()).toBe(200);
    await page.locator('[name=work_token]').waitFor({ state: 'attached' });
    await submit(page, 'main form', { email, password: pass, password_confirmation: pass, terms: true, privacy: true });
    await expect(page).toHaveURL(new RegExp(`/${lang}/register/check-inbox$`), { timeout: 90_000 });
    const link = await mailLink(page, email, /https:\/\/localhost:18443\/(?:pl|en)\/verify-email\?token=[a-f0-9]+/);
    const tokenHash = digest(new URL(link).searchParams.get('token'));
    const token = await one(db, `SELECT account_id,extract(epoch FROM expires_at-created_at)::int AS lifetime
      FROM account_email_tokens WHERE token_hash=$1 AND purpose='verify_email'`, [tokenHash]);
    expect(token.lifetime).toBe(86400);
    const state = () => one(db, 'SELECT status,email_verified_at FROM accounts WHERE id=$1', [token.account_id]);
    const move = seconds => one(db, `UPDATE account_email_tokens SET expires_at=clock_timestamp()+make_interval(secs=>$2)
      WHERE token_hash=$1 RETURNING purpose`, [tokenHash, seconds]);
    await expect.poll(async () => (await one(db, `SELECT coalesce(max(count),0)::int AS count
      FROM web_rate_limits WHERE limiter_name='identity-verification-source'
      AND window_start=date_trunc('minute',clock_timestamp())`, [])).count, { timeout: 65_000 }).toBeLessThanOrEqual(7);

    await move(before);
    expect((await page.goto(link)).status()).toBe(200);
    const form = 'main form[action*="verify-email?"]';
    await expect(page.locator(form).getByRole('button')).toHaveText(
      text(lang, 'Verify e-mail address', 'Potwierdź adres e-mail'));
    expect(await state()).toEqual({ status: 'pending_verification', email_verified_at: null });

    if (!expired) {
      const accepted = await formPost(page, form);
      expect(accepted.status()).toBe(200);
      expect((await accepted.text()).includes(text(lang,
        'Your e-mail has been verified.', 'Twój adres e-mail został zweryfikowany.'))).toBe(true);
      expect(await state()).toEqual({ status: 'active', email_verified_at: expect.any(Date) });
      expect((await one(db, 'SELECT used_at FROM account_email_tokens WHERE token_hash=$1', [tokenHash])).used_at)
        .toBeInstanceOf(Date);
      return;
    }

    await move(after);
    const refused = await formPost(page, form);
    expect(refused.status()).toBe(422);
    const refusedRead = await page.reload();
    expect(refusedRead.status()).toBe(422);
    await expect(page.locator('main')).toContainText(text(lang,
      'This link is invalid or has expired. Request a new verification e-mail.',
      'Ten link jest nieprawidłowy lub wygasł. Poproś o nową wiadomość weryfikacyjną.'));
    expect(await state()).toEqual({ status: 'pending_verification', email_verified_at: null });
    expect(await one(db, 'SELECT used_at FROM account_email_tokens WHERE token_hash=$1', [tokenHash])).toEqual({ used_at: null });

    // Verification retention also takes the resend cooldown into account.
    await one(db, `UPDATE account_email_tokens SET created_at=expires_at-interval '24 hours',
      updated_at=clock_timestamp()-interval '61 seconds' WHERE token_hash=$1 RETURNING purpose`, [tokenHash]);
    const retained = async () => (await db.query('SELECT purpose FROM account_email_tokens WHERE token_hash=$1', [tokenHash])).rows;
    await sweep(db, 'cleanup.tokens', retained, []);
  });
}

for (const expired of [false, true]) {
  test(`password reset ${expired ? 'refuses' : 'consumes'} a link at its deadline`, async ({ page, lang, db, seed }) => {
    const account = await seed();
    const next = password();
    await seedPasswordRange(next);
    await page.goto(`/${lang}/forgot-password`);
    await submit(page, 'main form', { email: account.email });
    const link = await mailLink(page, account.email, /https:\/\/localhost:18443\/(?:pl|en)\/reset-password\/[a-f0-9]+/);
    expect((await page.goto(link)).status()).toBe(200);
    const state = () => one(db, `SELECT a.password_hash,a.native_security_generation,r.used_at,
        extract(epoch FROM r.expires_at-r.created_at)::int AS lifetime
      FROM accounts a JOIN account_password_resets r ON r.account_id=a.id WHERE a.id=$1`, [account.id]);
    const issued = await state();
    expect(issued.lifetime).toBe(3600);
    expect(issued.used_at).toBeNull();
    await one(db, `UPDATE account_password_resets SET expires_at=clock_timestamp()+make_interval(secs=>$2)
      WHERE account_id=$1 RETURNING account_id`, [account.id, expired ? after : before]);
    await submit(page, 'main form', { email: account.email, password: next, password_confirmation: next });
    const result = await state();
    if (expired) {
      await expect(page.locator('main [role=alert]')).toHaveText(text(lang,
        'This password reset link is invalid or expired, or the passwords do not match. Request a new link if needed.',
        'Link jest nieprawidłowy lub wygasł albo hasła nie są identyczne. W razie potrzeby poproś o nowy link.'));
      expect(result.used_at).toBeNull();
      expect(result.password_hash === issued.password_hash, 'password digest unchanged').toBe(true);
      expect(result.native_security_generation).toBe(issued.native_security_generation);
    } else {
      await expect(page).toHaveURL(new RegExp(`/${lang}/login$`));
      expect(result.used_at).toBeInstanceOf(Date);
      expect(result.password_hash === issued.password_hash, 'password digest changed').toBe(false);
      await login(page, lang, { ...account, password: next });
      expect((await page.goto(`/${lang}/account/sessions`)).status()).toBe(200);
    }
  });
}
