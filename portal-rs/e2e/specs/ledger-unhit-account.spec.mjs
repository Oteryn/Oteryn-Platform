import { escapeRegex } from '../regex.mjs';
import {createHash, createHmac, randomBytes, randomUUID} from 'node:crypto';
import {test, expect} from '../fixtures.mjs';
import {login, logout, mailLink, register, seedPasswordRange, submit, waitForLoginBudget} from '../helpers.mjs';
import {enrollMfa, query} from '../staff-rbac-helpers.mjs';

test('MFA aliases accept a real challenge and revoke its remembered device', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const account = await register(page, lang);
  await login(page, lang, account);
  await enrollMfa(page, lang, account);
  await login(page, lang, account);
  await expect(page).toHaveURL(/\/mfa\/challenge$/);
  await submit(page, 'main form[action$="/mfa/email"]');
  await expect(page).toHaveURL(/\/mfa\/challenge$/);

  const deliveredCode = (await mailLink(page, account.email, /(?:^|\r?\n)[a-f0-9]{64}(?=\r?\n|$)/)).trim();

  const origin = new URL(page.url()).origin;
  const csrf = await page.locator('main form[action$="/mfa/challenge"] [name=_csrf]').inputValue();
  const challenged = await page.request.post('/mfa/challenge', {
    form: {_csrf: csrf, code: deliveredCode, remember: 'yes'},
    headers: {origin}, maxRedirects: 0,
  });
  expect(challenged.status()).toBe(302);
  expect((await page.request.get(`/${lang}/account/privacy`, {maxRedirects: 0})).status()).toBe(200);
  expect(await query(`
    SELECT a.action FROM audit_log a JOIN accounts u ON u.id=a.actor
    WHERE u.email=$1 AND a.action='mfa.login.email'
  `, [account.email])).toEqual([{action: 'mfa.login.email'}]);
  expect((await page.goto(`/${lang}/mfa`)).status()).toBe(200);
  const device = 'main form[action$="/mfa/devices"]';
  await expect(page.locator(device)).toHaveCount(1);
  await submit(page, device);
  await expect(page.locator(device)).toHaveCount(0);
});

for (const prefix of ['', 'localized']) {
  test(`password sign-in through the ${prefix || 'unprefixed'} MFA entry point`, async ({page}, info) => {
    const lang = info.project.metadata.locale;
    const account = await register(page, lang);
    const origin = new URL(page.url()).origin;
    for (const target of prefix ? [`/${lang}/mfa/login`] : ['/login', '/mfa/login']) {
      expect((await page.goto(`/${lang}/login`)).status()).toBe(200);
      const csrf = await page.locator('main [name=_csrf]').inputValue();
      await waitForLoginBudget();
      const response = await page.request.post(target, {
        form: {_csrf: csrf, email: account.email, password: account.password},
        headers: {origin}, maxRedirects: 0,
      });
      expect(response.status()).toBe(302);
      expect((await page.request.get(`/${lang}/account/privacy`, {maxRedirects: 0})).status()).toBe(200);
      await page.context().clearCookies();
    }
  });
}

test('player denies OAuth consent without granting an authorization code', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  await login(page, lang, await register(page, lang));
  const state = randomUUID();
  const redirect = new URL('/__oauth/callback', page.url()).href;
  const challenge = createHash('sha256').update(randomBytes(32)).digest('base64url');
  const params = new URLSearchParams({
    response_type: 'code', client_id: 'e2e-local', redirect_uri: redirect,
    scope: 'game:ticket', state, code_challenge: challenge, code_challenge_method: 'S256',
  });
  expect((await page.goto(`/oauth/authorize?${params}`)).status()).toBe(200);
  const [response] = await Promise.all([
    page.waitForResponse(r => r.request().method() === 'POST' && new URL(r.url()).pathname === '/oauth/authorize'),
    page.locator('main button[name=decision][value=deny]').click(),
  ]);
  expect(response.status()).toBe(302);
  const destination = new URL(response.headers().location);
  expect(destination.origin + destination.pathname).toBe(redirect);
  expect(destination.searchParams.get('state')).toBe(state);
  expect(destination.searchParams.get('error')).toBe('access_denied');
  expect(destination.searchParams.has('code')).toBe(false);
});

test('player creates and deletes a private atlas marker with a session CSRF token', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  await login(page, lang, await register(page, lang));
  const origin = new URL(page.url()).origin;
  const path = '/atlas/private-markers';
  const before = await page.request.get(path);
  expect(before.status()).toBe(200);
  const {csrf_token} = await before.json();
  const data = {x: 100, y: 200, floor: 7, text: 'Ledger marker'};
  expect((await page.request.post(path, {data, headers: {origin}})).status()).toBe(403);
  const headers = {origin, 'x-csrf-token': csrf_token};
  const created = await page.request.post(path, {data, headers});
  expect(created.status()).toBe(201);
  const {id} = await created.json();
  const read = await page.request.get(path);
  expect(read.status()).toBe(200);
  expect(read.headers()['cache-control']).toBe('private, no-store');
  expect((await read.json()).markers).toEqual([{id, x: 100, y: 200, floor: 7, label: data.text}]);
  expect((await page.request.delete(`${path}/${id}`, {headers})).status()).toBe(204);
  expect((await (await page.request.get(path)).json()).markers).toEqual([]);
});

test('stats watch API rejects browser authentication for write operations', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  await login(page, lang, await register(page, lang));
  for (const method of ['POST', 'DELETE']) {
    const response = await page.request.fetch('/api/v1/stats/watches', {
      method, data: {target_id: randomUUID(), kind: 'character'},
    });
    expect(response.status()).toBe(401);
    expect(response.headers()['cache-control']).toBe('private, no-store');
  }
});

test('player updates privacy with PUT and revokes other sessions through both DELETE routes', async ({page, browser}, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const account = await register(page, lang);
  await login(page, lang, account);
  const origin = new URL(page.url()).origin;
  const privacy = `/${lang}/account/privacy`;
  expect((await page.request.get(privacy, {maxRedirects: 0})).status()).toBe(200);
  expect((await page.goto(privacy)).status()).toBe(200);
  const token = await page.locator('main form [name=_csrf]').inputValue();
  const changed = await page.request.put(privacy, {
    form: {_csrf: token, locale: lang, public_status_visible: '1'},
    headers: {origin}, maxRedirects: 0,
  });
  expect(changed.status()).toBe(303);
  expect((await page.reload()).status()).toBe(200);
  await expect(page.locator('[name=public_status_visible]')).toBeChecked();

  const peer = await browser.newContext({baseURL: origin, locale: lang});
  try {
    const other = await peer.newPage();
    for (const target of ['/account/sessions', `/${lang}/account/sessions`]) {
      await login(other, lang, account);
      expect((await other.request.get(privacy, {maxRedirects: 0})).status()).toBe(200);
      expect((await page.goto(`/${lang}/account/sessions`)).status()).toBe(200);
      const csrf = await page.locator('main form[action$="/others"] [name=_csrf]').inputValue();
      const revoked = await page.request.delete(target, {
        headers: {origin, 'x-csrf-token': csrf}, maxRedirects: 0,
      });
      expect(revoked.status()).toBe(303);
      expect((await other.request.get(privacy, {maxRedirects: 0})).status()).toBe(303);
      expect((await page.request.get(privacy)).status()).toBe(200);
    }
    await login(other, lang, account);
    expect((await other.request.get(privacy, {maxRedirects: 0})).status()).toBe(200);
    expect((await page.goto(`/${lang}/account/sessions`)).status()).toBe(200);
    const selected = page.locator('main li:not(:has(p)) form[action*="/account/sessions/"]');
    await expect(selected).toHaveCount(1);
    const action = (await selected.getAttribute('action')).replace(`/${lang}/`, '/');
    const csrf = await selected.locator('[name=_csrf]').inputValue();
    const revoked = await page.request.post(action, {
      form: {_csrf: csrf}, headers: {origin}, maxRedirects: 0,
    });
    expect(revoked.status()).toBe(302);
    const denied = await other.request.get(privacy, {maxRedirects: 0});
    expect(denied.status()).toBe(303);
    expect(denied.headers().location).toBe(`/${lang}/login`);
    expect((await page.request.get(privacy, {maxRedirects: 0})).status()).toBe(200);
  } finally {
    await peer.close();
  }
});

test('mail reset token works through the unprefixed reset endpoint', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const account = await register(page, lang);
  await logout(page, lang);
  expect((await page.goto(`/${lang}/forgot-password`)).status()).toBe(200);
  await submit(page, 'main form', {email: account.email});
  const resetUrl = new URL(`/${lang}/reset-password/`, page.url());
  const resetPath = resetUrl.href.replace(/[\^$.*+?()[\]{}|\\]/g, '\\$&');
  const link = await mailLink(page, account.email, new RegExp(resetPath + '[a-f0-9]+'));
  expect((await page.goto(link)).status()).toBe(200);
  const form = await page.locator('main form').evaluate(f => Object.fromEntries(new FormData(f)));
  const next = await seedPasswordRange();
  const response = await page.request.post('/reset-password', {
    form: {...form, email: account.email, password: next, password_confirmation: next},
    headers: {origin: new URL(page.url()).origin}, maxRedirects: 0,
  });
  expect(response.status()).toBe(302);
  expect(response.headers().location).toMatch(/^\/(pl|en)\/login$/);
  await login(page, lang, {...account, password: next});
  expect((await page.request.get(`/${lang}/account/privacy`, {maxRedirects: 0})).status()).toBe(200);
});

test('anonymous verification resend has the same generic response through its form and alias', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const path = `/${lang}/verify-email/resend`;
  const email = `ledger-${randomUUID()}@local.test`;
  await expect.poll(async () => {
    const [row] = await query("SELECT COALESCE(max(count),0)::int AS used FROM web_rate_limits WHERE limiter_name='identity-registration' AND window_start=date_trunc('minute',clock_timestamp())");
    return row.used;
  }, {timeout: 65_000}).toBeLessThanOrEqual(3);
  expect((await page.goto(path)).status()).toBe(200);
  await submit(page, 'main form', {email});
  await expect(page).toHaveURL(new RegExp(`/${escapeRegex(lang)}/register/check-inbox$`));
  expect((await page.goto(path)).status()).toBe(200);
  const csrf = await page.locator('main form [name=_csrf]').inputValue();
  const response = await page.request.post('/verify-email/resend', {
    form: {_csrf: csrf, email},
    headers: {origin: new URL(page.url()).origin}, maxRedirects: 0,
  });
  expect(response.status()).toBe(303);
  expect(response.headers().location).toMatch(/^\/(pl|en)\/register\/check-inbox$/);
  expect(await query('SELECT id FROM accounts WHERE email=$1', [email])).toEqual([]);
});

for (const localized of [false, true]) {
  test(`owner generates, confirms, and revokes a recovery key through the ${localized ? 'localized DELETE' : 'unprefixed'} route aliases`, async ({page}, info) => {
    test.setTimeout(300_000);
    const lang = info.project.metadata.locale;
    const account = await register(page, lang);
    await login(page, lang, account);
    const manage = '/account/recovery-key';
    const readState = () => query(`
      SELECT r.confirmed_at IS NOT NULL AS confirmed, r.revoked_at IS NOT NULL AS revoked
        FROM account_recovery_keys r JOIN accounts a ON a.id = r.account_id
        WHERE a.email=$1
    `, [account.email]);
    const prefix = localized ? `/${lang}` : '';
    expect((await page.goto(manage)).status()).toBe(200);
    const csrf = await page.locator('main form [name=_csrf]').inputValue();
    const headers = {origin: new URL(page.url()).origin, 'accept-language': lang};
    const form = {_csrf: csrf, current_password: account.password};
    const generated = await page.request.post(manage, {form, headers});
    expect(generated.status()).toBe(200);
    const key = await page.evaluate(html => new DOMParser().parseFromString(html, 'text/html')
      .querySelector('.op-code')?.textContent.trim(), await generated.text());
    expect(Boolean(key)).toBe(true);
    const confirmed = await page.request.post(`${manage}/confirm`, {
      form: {...form, recovery_key: key}, headers,
    });
    expect(confirmed.status()).toBe(200);
    expect(await readState()).toEqual([{confirmed: true, revoked: false}]);

    const revoked = await page.request.delete(`${prefix}${manage}`, {
      form, headers, maxRedirects: 0,
    });
    expect(revoked.status()).toBe(303);
    expect(revoked.headers().location).toBe(`/${lang}${manage}`);
    expect(await readState()).toEqual([{confirmed: true, revoked: true}]);

    if (!localized) {
      await logout(page, lang);
      expect((await page.goto(`/${lang}/recovery-key`)).status()).toBe(200);
      const anonymousCsrf = await page.locator('main form [name=_csrf]').inputValue();
      const next = await seedPasswordRange();
      const denied = await page.request.post('/recovery-key', {
        form: {_csrf: anonymousCsrf, email: account.email, recovery_key: key, password: next, password_confirmation: next},
        headers, maxRedirects: 0,
      });
      expect(denied.status()).toBe(302);
      expect(denied.headers().location).toBe(`/${lang}/recovery-key`);
      await login(page, lang, account);
      expect((await page.request.get(`/${lang}/account/privacy`, {maxRedirects: 0})).status()).toBe(200);
    }
  });
}

test('player changes the password through both PUT routes after sudo', async ({page}, info) => {
  test.setTimeout(240_000);
  const lang = info.project.metadata.locale;
  const account = await register(page, lang);
  await login(page, lang, account);
  for (const [path, fixture] of [
    ['/password/change', 'replacement'], [`/${lang}/password/change`, 'alternate'],
  ]) {
    expect((await page.goto(`/${lang}/account/sudo`)).status()).toBe(200);
    await submit(page, 'main form', {current_password: account.password});
    expect((await page.goto(`/${lang}/password/change`)).status()).toBe(200);
    const csrf = await page.locator('main form [name=_csrf]').inputValue();
    const next = await seedPasswordRange(fixture);
    const response = await page.request.put(path, {
      form: {_csrf: csrf, current_password: account.password, password: next, password_confirmation: next},
      headers: {origin: new URL(page.url()).origin}, maxRedirects: 0,
    });
    expect(response.status()).toBe(303);
    expect(response.headers().location).toBe('/login');
    const denied = await page.request.get(`/${lang}/account/privacy`, {maxRedirects: 0});
    expect(denied.status()).toBe(303);
    expect(denied.headers().location).toBe(`/${lang}/login`);
    account.password = next;
    await login(page, lang, account);
    expect((await page.request.get(`/${lang}/account/privacy`, {maxRedirects: 0})).status()).toBe(200);
  }
});

test('signed unsubscribe form disables optional mail for its account', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const account = await register(page, lang);
  await login(page, lang, account);
  const settings = `/${lang}/account/notifications`;
  expect((await page.goto(settings)).status()).toBe(200);
  await submit(page, 'main form', {news_email: true, support_email: true});
  const [{id}] = await query('SELECT id FROM accounts WHERE email=$1', [account.email]);
  const expires = Math.floor(Date.now() / 1000) + 3600;
  const payload = `unsubscribe:${id}:news:${expires}`;
  const signature = createHmac('sha256', process.env.OTERYN_NOTIFY_UNSUBSCRIBE_KEY).update(payload).digest('hex');
  const target = `/${lang}/unsubscribe`;
  await logout(page, lang);
  expect((await page.goto(`${target}?token=${encodeURIComponent(`${payload}:${signature}`)}`)).status()).toBe(200);
  const [response] = await Promise.all([
    page.waitForResponse(r => r.request().method() === 'POST' && new URL(r.url()).pathname === target),
    page.locator('main form button[type=submit]').click(),
  ]);
  expect(response.status()).toBe(200);
  await login(page, lang, account);
  expect((await page.goto(settings)).status()).toBe(200);
  await expect(page.locator('[name=news_email]')).not.toBeChecked();
  await expect(page.locator('[name=support_email]')).toBeChecked();
});

test('unprefixed registration enforces proof of work before creating the account', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const email = `ledger-${randomUUID()}@local.test`;
  const secret = await seedPasswordRange('registration');
  expect((await page.goto(`/${lang}/register`)).status()).toBe(200);
  const fields = await page.locator('main form').evaluate(form => {
    const data = Object.fromEntries(new FormData(form));
    for (const name of ['terms', 'privacy']) data[name] = form.elements.namedItem(name).value;
    return data;
  });
  const response = await page.request.post('/register', {
    form: {...fields, email, password: secret, password_confirmation: secret, work_token: '', work_nonce: ''},
    headers: {origin: new URL(page.url()).origin}, maxRedirects: 0,
  });
  expect(response.status()).toBe(422);
  expect(await query('SELECT id FROM accounts WHERE email=$1', [email])).toEqual([]);
});

// F-web-33906076394532, R-web-98846564089840: rename and deletion forms
// require fresh character-service lifecycle evidence. The harness publishes
// LCFA ownership but cannot supply the service-specific Game projection.
