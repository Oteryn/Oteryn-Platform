import { once } from 'node:events';
import { createServer } from 'node:http';
import { test, expect } from '@playwright/test';
import { cookie, expectSession, seedAccount, seedEntity } from '../security/auth-fixtures.mjs';
import { checkHeaders, checkIssued, deviceCookie, sessionCookie } from './policy.mjs';
import { challengeForm, loginForm, logoutForm, signIn, submitForm } from './session-actions.mjs';

async function checkRedirect(response, status, location) {
  expect(response.status(), 'redirect status').toBe(status);
  expect(response.headers().location, 'redirect location').toBe(location);
  return checkHeaders(response, location + ' HTTP ' + status);
}

async function formToken(page, selector) {
  const token = await page.locator(selector + ' input[name=_csrf]').inputValue();
  expect(/^[a-f0-9]{64}$/.test(token), 'synchronizer token in form').toBe(true);
  return token;
}

async function openLogin(page, locale) {
  const response = await page.goto('/' + locale + '/login');
  expect(response.status()).toBe(200);
  expect(response.headers()['content-type']).toMatch(/^text\/html\b/);
  checkIssued(await checkHeaders(response, 'anonymous login'), sessionCookie);
  return formToken(page, loginForm);
}

async function endSession(page, context, locale, authenticated) {
  const response = await submitForm(page, logoutForm, {});
  checkIssued(await checkRedirect(response, 302, '/' + locale + '/'), sessionCookie, '0');
  expect((await context.cookies()).some(entry => entry.name === sessionCookie), 'browser removed the session').toBe(false);
  await expectSession(authenticated, undefined);
  const protectedPage = await page.request.get('/account', {
    headers: { accept: 'text/html', 'accept-language': locale }, maxRedirects: 0,
  });
  await checkRedirect(protectedPage, 303, '/' + locale + '/login');
}

test('anonymous language redirect issues a host-only cookie and home stays no-store', async ({ page }, info) => {
  const locale = info.project.metadata.locale;
  const headers = { accept: 'text/html', 'accept-language': locale };
  await checkRedirect(await page.request.get('/', { headers, maxRedirects: 0 }), 303, '/' + locale + '/');
  const language = await page.request.get('/language/' + locale, { headers, maxRedirects: 0 });
  checkIssued(await checkRedirect(language, 303, '/' + locale + '/'), 'portal_language', '31536000');
  const home = await page.goto('/' + locale + '/');
  expect(home.status()).toBe(200);
  expect(home.headers()['content-type']).toMatch(/^text\/html\b/);
  await checkHeaders(home, 'anonymous home');

  const asset = await page.request.get('/static/css/tokens.css', { maxRedirects: 0 });
  expect(asset.status()).toBe(200);
  await checkHeaders(asset, 'public stylesheet', { cache: 'public, max-age=300, must-revalidate' });
});

test('login rotates session and CSRF and logout expires the session', async ({ page, context }, info) => {
  const locale = info.project.metadata.locale;
  const account = await seedAccount(locale);
  const anonymousCsrf = await openLogin(page, locale);
  const anonymous = await cookie(context);
  await expectSession(anonymous, null);

  const signedIn = await signIn(page, account);
  checkIssued(await checkRedirect(signedIn, 302, '/' + locale + '/'), sessionCookie);
  const authenticated = await cookie(context);
  expect(authenticated !== anonymous, 'session rotated').toBe(true);
  await expectSession(anonymous, undefined);
  await expectSession(authenticated, account.id);

  const home = await page.goto('/' + locale + '/');
  await checkHeaders(home, 'authenticated home');
  const nextCsrf = await formToken(page, logoutForm);
  expect(nextCsrf !== anonymousCsrf, 'CSRF token rotated').toBe(true);
  // CSRF uses a server-side synchronizer token delivered in forms.
  expect((await context.cookies()).map(entry => entry.name).sort()).toEqual([sessionCookie]);
  const stale = await submitForm(page, logoutForm, { _csrf: anonymousCsrf });
  expect(stale.status()).toBe(403);
  await checkHeaders(stale, 'pre-login CSRF token refused');
  await expectSession(authenticated, account.id);
  await endSession(page, context, locale, authenticated);
});

test('MFA rotates the challenge session and the opt-in remembered-device cookie', async ({ page, context }, info) => {
  const locale = info.project.metadata.locale;
  const home = '/' + locale + '/';
  const account = await seedAccount(locale);
  const seeded = await seedEntity('account-mfa', { accountId: account.id, password: account.password });
  const loginCsrf = await openLogin(page, locale);
  const anonymous = await cookie(context);

  const passwordOnly = await signIn(page, account);
  checkIssued(await checkRedirect(passwordOnly, 302, home + 'mfa/challenge'), sessionCookie);
  const pending = await cookie(context);
  expect(pending !== anonymous, 'password step rotated anonymous session').toBe(true);
  await expectSession(pending, null);
  const challenge = await page.goto(home + 'mfa/challenge');
  expect(challenge.status()).toBe(200);
  await checkHeaders(challenge, 'MFA challenge');
  expect(await formToken(page, challengeForm) !== loginCsrf, 'challenge rotated CSRF').toBe(true);

  const completion = await submitForm(page, challengeForm, { code: seeded.recoveryCodes[0], remember: 'yes' });
  const issued = await checkRedirect(completion, 302, home);
  checkIssued(issued, sessionCookie);
  checkIssued(issued, deviceCookie, '2592000');
  const authenticated = await cookie(context);
  expect(authenticated !== pending, 'MFA rotated the challenge session').toBe(true);
  await expectSession(pending, undefined);
  await expectSession(authenticated, account.id);
  const remembered = await cookie(context, deviceCookie);

  await page.goto(home);
  await endSession(page, context, locale, authenticated);
  // The explicit 30-day trust grant is separate from the signed-in session.
  expect(await cookie(context, deviceCookie) === remembered, 'logout preserves opted-in browser trust').toBe(true);
  await openLogin(page, locale);
  const trustedLogin = await signIn(page, account);
  const rotated = await checkRedirect(trustedLogin, 302, home);
  checkIssued(rotated, sessionCookie);
  checkIssued(rotated, deviceCookie, '2592000');
  expect(await cookie(context, deviceCookie) !== remembered, 'trusted login rotated the device cookie').toBe(true);
  const trustedSession = await cookie(context);
  await expectSession(trustedSession, account.id);
  await page.goto(home);
  await endSession(page, context, locale, trustedSession);
});

const refusals = {
  400: { pl: 'Nieprawidłowe żądanie', en: 'Invalid request' },
  406: { pl: 'Nieobsługiwany format odpowiedzi', en: 'Response format not available' },
  431: { pl: 'Nagłówki żądania są za duże', en: 'Request headers are too large' },
};

for (const { label, status, headers } of [
  { label: 'oversized field', status: 431, headers: { 'x-e2e-size': 'a'.repeat(8193) } },
  { label: 'oversized header section', status: 431, headers: { 'x-e2e-first': 'a'.repeat(6500), 'x-e2e-second': 'b'.repeat(6500) } },
  { label: 'oversized cookie', status: 431, headers: { cookie: 'e2e=' + 'a'.repeat(8192) } },
  { label: 'malformed Host', status: 400, headers: { host: 'localhost:invalid' } },
  { label: 'unacceptable media type', status: 406, headers: { accept: 'application/x-portal-e2e-unknown' } },
]) {
  test(label + ' returns a localized refusal with security headers', async ({ request }, info) => {
    const locale = info.project.metadata.locale;
    // Use the unprefixed root for header/Host handling and a known HTML page
    // for media negotiation. Each response is checked before following a redirect.
    const target = status === 406 ? '/' + locale + '/' : '/';
    const response = await request.get(target, {
      headers: { accept: 'text/html', 'accept-language': locale, ...headers },
      maxRedirects: 0,
    });
    expect(response.status()).toBe(status);
    await checkHeaders(response, label);
    expect(response.headers()['content-language']).toBe(locale);
    expect(response.headers()['content-type']).toMatch(/^text\/html\b/);
    const html = await response.text();
    expect(html.includes('<html lang="' + locale + '"'), 'error document language').toBe(true);
    expect(html.includes(refusals[status][locale]), 'localized error title').toBe(true);
  });
}

test('Chromium refuses to render a portal page in a cross-origin iframe', async ({ page, baseURL }, info) => {
  const url = new URL('/' + info.project.metadata.locale + '/bazaar', baseURL);
  const direct = await page.goto(url.href);
  expect(direct.status()).toBe(200);
  await checkHeaders(direct, 'direct Bazaar page');
  await expect(page.locator('main h1')).toBeVisible();
  const embedder = createServer((_request, response) => {
    response.writeHead(200, { 'content-type': 'text/html' });
    response.end('<!doctype html><title>Frame probe</title><iframe title="Portal" src="' + url.href + '"></iframe>');
  });
  embedder.listen(0, '127.0.0.1');
  await once(embedder, 'listening');
  try {
    const origin = 'http://127.0.0.1:' + embedder.address().port;
    expect(origin).not.toBe(url.origin);
    const blocked = page.waitForEvent('requestfailed', request =>
      request.isNavigationRequest() && request.url() === url.href);
    const policyViolation = page.waitForEvent('console', message =>
      message.type() === 'error' && /frame-ancestors|X-Frame-Options/i.test(message.text()));
    const [failure] = await Promise.all([blocked, policyViolation, page.goto(origin)]);
    await expect(page).toHaveTitle('Frame probe');
    expect(failure.failure()?.errorText).toBe('net::ERR_BLOCKED_BY_RESPONSE');
    await expect(page.frameLocator('iframe[title=Portal]').locator('main h1')).toHaveCount(0);
  } finally {
    await new Promise(resolve => embedder.close(resolve));
  }
});
