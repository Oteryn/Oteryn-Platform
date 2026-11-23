import { randomBytes, createHash, randomUUID } from 'node:crypto';
import { rateKey } from '../authz/database.mjs';
import { test, expect } from '../fixtures.mjs';
import { login, pool, register, waitForLoginBudget } from '../helpers.mjs';

const titles = {
  400: ['Nieprawidłowe żądanie', 'Invalid request'],
  401: ['Wymagane logowanie', 'Sign-in required'],
  403: ['Brak dostępu', 'Access denied'],
  404: ['Nie znaleziono strony', 'Page not found'],
  405: ['Niedozwolona metoda', 'Method not allowed'],
  413: ['Zbyt duże żądanie', 'Request too large'],
  429: ['Zbyt wiele żądań', 'Too many requests'],
  503: ['Usługa chwilowo niedostępna', 'Temporarily unavailable'],
};

async function assertError(page, lang, status, response) {
  expect(response.status()).toBe(status);
  const headers = response.headers();
  expect(headers['content-language']).toBe(lang);
  expect(headers['content-type']).toMatch(/^text\/html\b/);
  const cache = headers['cache-control'].split(',').map(value => value.trim());
  expect(cache.sort()).toEqual(status === 403 || status === 429 ? ['no-store', 'private'] : ['no-store']);
  const body = await response.text();
  await page.setContent(body);
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  for (const landmark of ['header#portal-header', 'main#main', 'footer']) {
    await expect(page.locator(landmark).first()).toBeVisible();
  }
  await expect(page.locator('header #portal-navigation')).toHaveCount(1);
  await expect(page.locator('main h1')).toHaveText(titles[status][lang === 'pl' ? 0 : 1]);
  expect(/stack backtrace|panicked at|sqlstate|database error|\/rustc\/|traceback/i.test(body),
    'error page excludes internal diagnostics').toBe(false);
}

test('public browser errors use the localized shell and no-store', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const headers = { accept: 'text/html', 'accept-language': lang, connection: 'close' };
  const options = { headers, maxRedirects: 0 };
  await page.goto(`/${lang}/`);
  const missing = `/${lang}/missing-${randomUUID()}`;
  for (const [status, path] of [
    [400, '/language/unsupported'],
    [401, `/${lang}/atlas/bestiary/019a0000-0000-7000-8000-000000000001`],
    [404, missing],
  ]) {
    await assertError(page, lang, status, await page.request.get(path, options));
  }
  const weighted = await page.request.get(missing, {
    ...options, headers: { ...headers, accept: 'text/html;q=1, application/json;q=0.1' },
  });
  await assertError(page, lang, 404, weighted);
  const excluded = await page.request.get(missing, {
    ...options, headers: { ...headers, accept: 'text/html;q=0' },
  });
  expect(excluded.status()).toBe(404);
  expect(excluded.headers()['content-type']).toBeUndefined();
  expect(await excluded.text()).toBe('');
  const wildcard = await page.request.get(missing, {
    ...options, headers: { ...headers, accept: 'text/html;q=0, */*;q=0.8' },
  });
  expect(wildcard.status()).toBe(404);
  expect(wildcard.headers()['content-type']).toMatch(/^application\/json\b/);
  expect(await wildcard.json()).toEqual({ status: 404, error: titles[404][lang === 'pl' ? 0 : 1] });
  const charsetRefusal = await page.request.get(`/api/missing-${randomUUID()}`, {
    ...options, headers: { ...headers, accept: 'text/html; charset=iso-8859-1, application/json;q=0.5' },
  });
  expect(charsetRefusal.status()).toBe(404);
  expect(charsetRefusal.headers()['content-type']).toMatch(/^application\/json\b/);
  expect(await charsetRefusal.json()).toEqual({ status: 404, error: titles[404][lang === 'pl' ? 0 : 1] });
  const forbidden = await page.request.post(`/${lang}/login`, { ...options, form: {} });
  await assertError(page, lang, 403, forbidden);
  for (const accept of ['application/json, text/html;q=0', 'application/json, text/html']) {
    const jsonRefusal = await page.request.post(`/${lang}/login`, {
      ...options, headers: { ...headers, accept }, form: {},
    });
    expect(jsonRefusal.status()).toBe(403);
    expect(jsonRefusal.headers()['content-type']).toMatch(/^application\/json\b/);
    expect(await jsonRefusal.json()).toEqual({ status: 403, error: titles[403][lang === 'pl' ? 0 : 1] });
  }
  const unsupported = await page.request.post(`/${lang}/`, options);
  expect(unsupported.headers().allow.split(/,\s*/)).toEqual(expect.arrayContaining(['GET', 'HEAD']));
  await assertError(page, lang, 405, unsupported);
  // The portal can reject the known Content-Length before reading the upload.
  // Send it directly to the disposable listener to observe that early response.
  const listener = new URL(`http://${process.env.OTERYN_BIND || '127.0.0.1:18080'}`);
  expect(listener.hostname).toBe('127.0.0.1');
  const oversized = await page.request.post(new URL(`/${lang}/`, listener).href, {
    ...options, data: Buffer.alloc(1024 * 1024 + 1),
  });
  await assertError(page, lang, 413, oversized);
});

test('login throttling and a cold account dependency use localized error pages', async ({ page }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const account = await register(page, lang);
  await login(page, lang, account);
  const unavailable = await page.goto(`/${lang}/account/dashboard`);
  await assertError(page, lang, 503, unavailable);

  await page.context().clearCookies();
  await page.goto(`/${lang}/login`);
  const token = await page.locator('[name=_csrf]').inputValue();
  const form = { _csrf: token, email: `limit-${randomUUID()}@local.test`, password: 'Wrong local password!' };
  const source = '127.0.0.2';
  const headers = {
    accept: 'text/html', origin: new URL(page.url()).origin, connection: 'close',
    'x-portal-e2e-login-peer': source,
  };
  const identityKey = Buffer.from(rateKey('identity-login', form.email, source), 'hex');
  const sourceKey = Buffer.from(rateKey('identity-login-source', source), 'hex');
  const db = pool();
  try {
    const bucket = async () => {
      const { rows: [row] } = await db.query(`
        WITH clock AS (SELECT statement_timestamp() AS now)
        SELECT extract(epoch FROM date_trunc('minute', now))::text AS window,
               extract(second FROM now)::float8 AS second,
               ARRAY(SELECT count::int FROM web_rate_limits
                     WHERE limiter_name='identity-login' AND key_hash=$1
                       AND window_start=date_trunc('minute', now)) AS identities,
               ARRAY(SELECT count::int FROM web_rate_limits
                     WHERE limiter_name='identity-login-source' AND key_hash=$2
                       AND window_start=date_trunc('minute', now)) AS sources
        FROM clock
      `, [identityKey, sourceKey]);
      return row;
    };
    let start;
    await expect.poll(async () => {
      const row = await bucket();
      start = row.window;
      return row.second < 5 && row.identities.length === 0 && row.sources.length === 0;
    }, { timeout: 65_000, intervals: [500] }).toBe(true);
    for (let attempt = 1; attempt <= 6; attempt++) {
      const response = await page.request.post(`/${lang}/login`, { form, headers, maxRedirects: 0 });
      const { window, identities, sources } = await bucket();
      expect(window).toBe(start);
      expect(identities).toEqual([attempt]);
      expect(sources).toEqual([attempt]);
      if (attempt <= 5) {
        expect(response.status()).toBe(302);
      } else {
        expect(Number(response.headers()['retry-after'])).toBeGreaterThan(0);
        await assertError(page, lang, 429, response);
      }
    }
  } finally {
    await db.end();
  }
});

test('OAuth token errors preserve their JSON envelope across Accept values', async ({ request }) => {
  await waitForLoginBudget(5);
  for (const accept of ['application/json', 'text/html', '*/*', 'text/html, application/json', undefined]) {
    const response = await request.post('/oauth/token', {
      form: { grant_type: 'unsupported', client_id: 'e2e-local' },
      headers: accept ? { accept } : {},
      maxRedirects: 0,
    });
    expect(response.status()).toBe(400);
    expect(response.headers()['content-type']).toMatch(/^application\/json\b/);
    expect(response.headers()['cache-control']).toContain('no-store');
    expect(await response.json()).toEqual({
      error: 'unsupported_grant_type',
      error_description: 'The authorization grant type is not supported by the authorization server.',
    });
  }
});

test('OAuth consent retains its protocol error for a browser form submission', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  await login(page, lang, await register(page, lang));
  const origin = new URL(page.url()).origin;
  const query = {
    response_type: 'code', client_id: 'e2e-local', redirect_uri: `${origin}/__oauth/callback`,
    scope: 'game:ticket', state: randomBytes(24).toString('hex'),
    code_challenge: createHash('sha256').update(randomBytes(32).toString('base64url')).digest('base64url'),
    code_challenge_method: 'S256',
  };
  await waitForLoginBudget(2);
  const consent = await page.goto(`/oauth/authorize?${new URLSearchParams(query)}`);
  expect(consent.status()).toBe(200);
  const fields = await page.locator('main form').evaluate(form => Object.fromEntries(new FormData(form)));
  const refusal = await page.request.post('/oauth/authorize', {
    headers: { accept: 'text/html', origin },
    form: { ...fields, decision: 'invalid' }, maxRedirects: 0,
  });
  expect(refusal.status()).toBe(400);
  expect(refusal.headers()['content-type']).toMatch(/^application\/json\b/);
  expect(refusal.headers()['cache-control']).toBe('no-store');
  expect(await refusal.json()).toEqual({
    error: 'invalid_request',
    error_description: 'The request is missing a required parameter or is malformed.',
  });
});
