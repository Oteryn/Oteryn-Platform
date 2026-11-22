import { createHash } from 'node:crypto';
import { once } from 'node:events';
import { createServer } from 'node:http';
import { test, expect } from '@playwright/test';
import { cookie, expectSession, seedAccount, seedEntity } from '../security/auth-fixtures.mjs';
import { checkHeaders, checkIssued, deviceCookie, matchesPortalCsp, sessionCookie } from './policy.mjs';
import { challengeForm, getFromPortal, loginForm, logoutForm, signIn, submitForm } from './session-actions.mjs';

test.use({ trace: 'off', screenshot: 'off', video: 'off' });

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
  const protectedPage = await getFromPortal(page.request, '/account', {
    headers: { accept: 'text/html', 'accept-language': locale }, maxRedirects: 0,
  });
  await checkRedirect(protectedPage, 303, '/' + locale + '/login');
}

test('anonymous language redirect issues a host-only cookie and home stays no-store', async ({ page }, info) => {
  const locale = info.project.metadata.locale;
  const headers = { accept: 'text/html', 'accept-language': locale };
  await checkRedirect(await getFromPortal(page.request, '/', { headers, maxRedirects: 0 }), 303, '/' + locale + '/');
  const language = await getFromPortal(page.request, '/language/' + locale, { headers, maxRedirects: 0 });
  checkIssued(await checkRedirect(language, 303, '/' + locale + '/'), 'portal_language', '31536000');
  const home = await page.goto('/' + locale + '/');
  expect(home.status()).toBe(200);
  expect(home.headers()['content-type']).toMatch(/^text\/html\b/);
  await checkHeaders(home, 'anonymous home');

  const asset = await getFromPortal(page.request, '/static/css/tokens.css', { maxRedirects: 0 });
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
    const response = await getFromPortal(request, target, {
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

test('CSP validator requires the full policy with unique directives', () => {
  const strict = "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'";
  expect(matchesPortalCsp(strict)).toBe(true);
  expect(matchesPortalCsp("frame-ancestors 'none'")).toBe(false);
  expect(matchesPortalCsp(strict + "; FRAME-ANCESTORS 'none'")).toBe(false);
  expect(matchesPortalCsp(strict.replace("object-src 'none'", "object-src *"))).toBe(false);
  expect(matchesPortalCsp(strict.replace("; base-uri 'none'", ""))).toBe(false);
  expect(matchesPortalCsp(strict.replace("default-src 'self'", "default-src *"))).toBe(false);
  const atlas = "default-src 'none'; script-src 'nonce-" + 'a'.repeat(64)
    + "'; style-src 'self'; img-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'";
  expect(matchesPortalCsp(atlas)).toBe(true);
  expect(matchesPortalCsp(atlas.replace(/'nonce-[a-f0-9]+'/, "'<nonce>'"))).toBe(false);
});

test('unacceptable reset GET preserves the one-time confirmation', async ({ page }, info) => {
  const locale = info.project.metadata.locale;
  const account = await seedAccount(locale);
  const target = '/' + locale + '/forgot-password';
  expect((await page.goto(target)).status()).toBe(200);
  const notice = page.locator('main [role=status]');
  await expect(notice).toHaveCount(0);
  await checkRedirect(await submitForm(page, 'main form', { email: account.email }), 302, target);
  const refused = await getFromPortal(page.request, target, {
    headers: { accept: 'application/x-portal-e2e-unknown' }, maxRedirects: 0,
  });
  expect(refused.status()).toBe(406);
  expect(refused.headers()['content-language']).toBe(locale);
  await checkHeaders(refused, 'reset media refusal');
  expect((await page.goto(target)).status()).toBe(200);
  await expect(notice).toHaveText(locale === 'pl'
    ? 'Jeśli konto z tym adresem istnieje, wysłaliśmy link do zresetowania hasła.'
    : 'If an account exists for that email, a password reset link has been sent.');
  expect((await page.reload()).status()).toBe(200);
  await expect(notice).toHaveCount(0);
});

test('League applies its same-origin stylesheet under CSP', async ({ page }, info) => {
  const asset = new URL('/static/css/league.css', info.project.use.baseURL).href;
  const stylesheet = page.waitForResponse(response => response.url() === asset
    && response.request().resourceType() === 'stylesheet');
  const [response, document] = await Promise.all([
    stylesheet, page.goto('/' + info.project.metadata.locale + '/league'),
  ]);
  expect(document.status()).toBe(200);
  await checkHeaders(document, 'League document');
  expect(response.status()).toBe(200);
  expect(response.headers()['content-type']).toMatch(/^text\/css\b/);
  await checkHeaders(response, 'League stylesheet', { cache: 'public, max-age=300, must-revalidate' });
  await expect(page.locator('main.league-page section').first()).toHaveCSS('display', 'grid');
  await expect(page.locator('main.league-page')).toHaveCSS('max-width', '1440px');
});

test('unacceptable OAuth GET preserves the pending login destination', async ({ page, baseURL }, info) => {
  const locale = info.project.metadata.locale;
  const account = await seedAccount(locale);
  const params = new URLSearchParams({
    response_type: 'code', client_id: 'e2e-local',
    redirect_uri: new URL('/__oauth/callback', baseURL).href,
    scope: 'game:ticket', state: 'security-headers-oauth-first',
    code_challenge: createHash('sha256').update('security headers PKCE fixture').digest('base64url'),
    code_challenge_method: 'S256',
  });
  const accepted = '/oauth/authorize?' + params;
  const html = { accept: 'text/html', 'accept-language': locale };
  await checkRedirect(await getFromPortal(page.request, accepted, { headers: html, maxRedirects: 0 }),
    303, '/' + locale + '/login');

  params.set('state', 'security-headers-oauth-unacceptable');
  const refused = await getFromPortal(page.request, '/oauth/authorize?' + params, {
    headers: { ...html, accept: 'application/x-portal-e2e-unknown' }, maxRedirects: 0,
  });
  expect(refused.status()).toBe(406);
  expect(refused.headers()['content-language']).toBe(locale);
  await checkHeaders(refused, 'OAuth media refusal');

  expect((await page.goto('/' + locale + '/login')).status()).toBe(200);
  const signedIn = await signIn(page, account);
  expect(signedIn.status()).toBe(302);
  expect(signedIn.headers().location === accepted, 'login retained the first OAuth request').toBe(true);
  await checkHeaders(signedIn, 'OAuth login redirect');
  const consent = await page.goto(accepted);
  expect(consent.status()).toBe(200);
  await checkHeaders(consent, 'OAuth consent', { consentOrigin: new URL(baseURL).origin });
  await expect(page.locator('main form[action="/oauth/authorize"]')).toBeVisible();
});
