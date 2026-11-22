import { createHash } from 'node:crypto';
import { once } from 'node:events';
import { createServer } from 'node:http';
import { test, expect } from '@playwright/test';
import { mailLink } from '../helpers.mjs';
import { cookie, expectSession, seedAccount, seedEntity } from '../security/auth-fixtures.mjs';
import { atlas, checkHeaders, checkIssued, deviceCookie, matchesPortalCsp, sameOrigin, sessionCookie, shellPolicy, stylesForm, surface } from './policy.mjs';
import { withAtlasFixture } from './atlas-fixture.mjs';
import { challengeForm, getFromPortal, loginForm, logoutForm, signIn, submitForm } from './session-actions.mjs';

test.use({ trace: 'off', screenshot: 'off', video: 'off' });

async function checkRedirect(response, status, location, policy) {
  expect(response.status(), 'redirect status').toBe(status);
  expect(response.headers().location, 'redirect location').toBe(location);
  return checkHeaders(response, location + ' HTTP ' + status, policy);
}

const webPolicy = { csp: surface };
const mfaRedirect = { csp: sameOrigin };
const mfaPage = { csp: shellPolicy(sameOrigin) };

async function formToken(page, selector) {
  const token = await page.locator(selector + ' input[name=_csrf]').inputValue();
  expect(/^[a-f0-9]{64}$/.test(token), 'synchronizer token in form').toBe(true);
  return token;
}

async function openLogin(page, locale) {
  const response = await page.goto('/' + locale + '/login');
  expect(response.status()).toBe(200);
  expect(response.headers()['content-type']).toMatch(/^text\/html\b/);
  checkIssued(await checkHeaders(response, 'anonymous login', mfaPage), sessionCookie);
  return formToken(page, loginForm);
}

async function endSession(page, context, locale, authenticated) {
  const response = await submitForm(page, logoutForm, {});
  checkIssued(await checkRedirect(response, 302, '/' + locale + '/', webPolicy), sessionCookie, '0');
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
  await checkRedirect(await getFromPortal(page.request, '/', { headers, maxRedirects: 0 }), 303, '/' + locale + '/', webPolicy);
  const language = await getFromPortal(page.request, '/language/' + locale, { headers, maxRedirects: 0 });
  checkIssued(await checkRedirect(language, 303, '/' + locale + '/', webPolicy), 'portal_language', '31536000');
  const home = await page.goto('/' + locale + '/');
  expect(home.status()).toBe(200);
  expect(home.headers()['content-type']).toMatch(/^text\/html\b/);
  await checkHeaders(home, 'anonymous home', webPolicy);

  const asset = await getFromPortal(page.request, '/static/css/tokens.css', { maxRedirects: 0 });
  expect(asset.status()).toBe(200);
  await checkHeaders(asset, 'public stylesheet', { ...webPolicy, cache: 'public, max-age=300, must-revalidate' });
});

test('login rotates session and CSRF and logout expires the session', async ({ page, context }, info) => {
  const locale = info.project.metadata.locale;
  const account = await seedAccount(locale);
  const anonymousCsrf = await openLogin(page, locale);
  const anonymous = await cookie(context);
  await expectSession(anonymous, null);

  const signedIn = await signIn(page, account);
  checkIssued(await checkRedirect(signedIn, 302, '/' + locale + '/', mfaRedirect), sessionCookie);
  const authenticated = await cookie(context);
  expect(authenticated !== anonymous, 'session rotated').toBe(true);
  await expectSession(anonymous, undefined);
  await expectSession(authenticated, account.id);

  const home = await page.goto('/' + locale + '/');
  await checkHeaders(home, 'authenticated home', webPolicy);
  const nextCsrf = await formToken(page, logoutForm);
  expect(nextCsrf !== anonymousCsrf, 'CSRF token rotated').toBe(true);
  // CSRF uses a server-side synchronizer token delivered in forms.
  expect((await context.cookies()).map(entry => entry.name).sort()).toEqual([sessionCookie]);
  const stale = await submitForm(page, logoutForm, { _csrf: anonymousCsrf });
  expect(stale.status()).toBe(403);
  await checkHeaders(stale, 'pre-login CSRF token refused', webPolicy);
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
  checkIssued(await checkRedirect(passwordOnly, 302, home + 'mfa/challenge', mfaRedirect), sessionCookie);
  const pending = await cookie(context);
  expect(pending !== anonymous, 'password step rotated anonymous session').toBe(true);
  await expectSession(pending, null);
  const challenge = await page.goto(home + 'mfa/challenge');
  expect(challenge.status()).toBe(200);
  await checkHeaders(challenge, 'MFA challenge', mfaPage);
  expect(await formToken(page, challengeForm) !== loginCsrf, 'challenge rotated CSRF').toBe(true);

  const completion = await submitForm(page, challengeForm, { code: seeded.recoveryCodes[0], remember: 'yes' });
  const issued = await checkRedirect(completion, 302, home, mfaRedirect);
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
  const rotated = await checkRedirect(trustedLogin, 302, home, mfaRedirect);
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
  expect(matchesPortalCsp(sameOrigin, sameOrigin)).toBe(true);
  expect(matchesPortalCsp("frame-ancestors 'none'", sameOrigin)).toBe(false);
  expect(matchesPortalCsp(sameOrigin + "; FRAME-ANCESTORS 'none'", sameOrigin)).toBe(false);
  expect(matchesPortalCsp(sameOrigin.replace("object-src 'none'", "object-src *"), sameOrigin)).toBe(false);
  expect(matchesPortalCsp(sameOrigin.replace("; base-uri 'none'", ""), sameOrigin)).toBe(false);
  expect(matchesPortalCsp(sameOrigin.replace("default-src 'self'", "default-src *"), sameOrigin)).toBe(false);
  const atlasWithNonce = atlas.replace("'<nonce>'", "'nonce-" + 'a'.repeat(64) + "'");
  expect(matchesPortalCsp(atlasWithNonce, atlas)).toBe(true);
  expect(matchesPortalCsp(atlas, atlas)).toBe(false);
  expect(matchesPortalCsp(atlasWithNonce.replace(/; script-src '[^']+'/, ''), atlas)).toBe(false);
  expect(matchesPortalCsp(stylesForm, atlas)).toBe(false);
});

test('unacceptable reset GET preserves the one-time confirmation', async ({ page }, info) => {
  const locale = info.project.metadata.locale;
  const account = await seedAccount(locale);
  const target = '/' + locale + '/forgot-password';
  expect((await page.goto(target)).status()).toBe(200);
  const notice = page.locator('main [role=status]');
  await expect(notice).toHaveCount(0);
  await checkRedirect(await submitForm(page, 'main form', { email: account.email }), 302, target, webPolicy);
  const refused = await getFromPortal(page.request, target, {
    headers: { accept: 'application/x-portal-e2e-unknown' }, maxRedirects: 0,
  });
  expect(refused.status()).toBe(406);
  expect(refused.headers()['content-language']).toBe(locale);
  await checkHeaders(refused, 'reset media refusal', { ...webPolicy, referrer: 'no-referrer' });
  expect((await page.goto(target)).status()).toBe(200);
  await expect(notice).toHaveText(locale === 'pl'
    ? 'Jeśli konto z tym adresem istnieje, wysłaliśmy link do zresetowania hasła.'
    : 'If an account exists for that email, a password reset link has been sent.');
  expect((await page.reload()).status()).toBe(200);
  await expect(notice).toHaveCount(0);

  const resetURL = await mailLink(page, account.email, /https:\/\/localhost:18443\/(?:pl|en)\/reset-password\/[a-f0-9]+/);
  const resetLink = await page.request.get(resetURL, { maxRedirects: 0 })
    .catch(() => { throw new Error('Mailed password-reset link could not be opened'); });
  expect(resetLink.status(), 'mailed reset link').toBe(200);
  await checkHeaders(resetLink, 'mailed reset link', { ...webPolicy, referrer: 'no-referrer' });
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
  await checkHeaders(response, 'League stylesheet', { ...webPolicy, cache: 'public, max-age=300, must-revalidate' });
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
    303, '/' + locale + '/login', { csp: stylesForm, referrer: 'no-referrer' });

  params.set('state', 'security-headers-oauth-unacceptable');
  const refused = await getFromPortal(page.request, '/oauth/authorize?' + params, {
    headers: { ...html, accept: 'application/x-portal-e2e-unknown' }, maxRedirects: 0,
  });
  expect(refused.status()).toBe(406);
  expect(refused.headers()['content-language']).toBe(locale);
  await checkHeaders(refused, 'OAuth media refusal', { csp: shellPolicy(stylesForm), referrer: 'strict-origin' });

  expect((await page.goto('/' + locale + '/login')).status()).toBe(200);
  const signedIn = await signIn(page, account);
  expect(signedIn.status()).toBe(302);
  expect(signedIn.headers().location === accepted, 'login retained the first OAuth request').toBe(true);
  await checkHeaders(signedIn, 'OAuth login redirect', mfaRedirect);
  const consent = await page.goto(accepted);
  expect(consent.status()).toBe(200);
  await checkHeaders(consent, 'OAuth consent', {
    csp: shellPolicy(stylesForm.replace("form-action 'self'", "form-action 'self' " + new URL(baseURL).origin)),
    referrer: 'strict-origin',
  });
  await expect(page.locator('main form[action="/oauth/authorize"]')).toBeVisible();
});

test('unacceptable character creation GET preserves the validation and submitted form', async ({ page }, info) => {
  const locale = info.project.metadata.locale;
  const account = await seedAccount(locale);
  expect((await page.goto('/' + locale + '/login')).status()).toBe(200);
  await checkRedirect(await signIn(page, account), 302, '/' + locale + '/', mfaRedirect);
  const target = '/' + locale + '/account/characters/new';
  expect((await page.goto(target)).status()).toBe(200);
  const selector = 'main form[method=post]';
  const form = page.locator(selector);
  const world = await form.locator('select[name=world] option').evaluateAll(options => options.find(o => o.value)?.value);
  expect(Boolean(world), 'available local world').toBe(true);
  const submission = await form.locator('[name=submission]').inputValue();
  const fields = { name: 'Admin', world, sex: 'Female', vocation: 'Knight' };
  await checkRedirect(await submitForm(page, selector, fields), 302, target, webPolicy);

  const refused = await getFromPortal(page.request, target, { headers: { accept: 'application/json' }, maxRedirects: 0 });
  expect(refused.status()).toBe(406);
  expect(refused.headers()['content-language']).toBe(locale);
  await checkHeaders(refused, 'character form media refusal', webPolicy);
  const notice = page.locator('main [role=status]');
  expect((await page.goto(target)).status()).toBe(200);
  await expect(notice).toHaveText(locale === 'pl'
    ? 'To imię jest zastrzeżone. Wybierz inne.'
    : 'This character name is reserved. Choose another name.');
  await expect(form.locator('[name=name]')).toHaveValue(fields.name);
  await expect(form.locator('[name=world]')).toHaveValue(fields.world);
  await expect(form.locator('[name=submission]')).toHaveValue(submission);
  await expect(form.locator('[name=sex][value=Female]')).toBeChecked();
  await expect(form.locator('[name=vocation][value=Knight]')).toBeChecked();
  expect((await page.reload()).status()).toBe(200);
  await expect(notice).toHaveCount(0);
  await expect(form.locator('[name=name]')).toHaveValue('');
});

test('Atlas applies its nonce-bound script on the published viewer', async ({ page, baseURL }, info) => {
  const locale = info.project.metadata.locale;
  await withAtlasFixture(async version => {
    const tile = new URL('/atlas/tiles/' + version + '/7/0/0/0.webp', baseURL).href;
    const [document, response] = await Promise.all([
      page.goto('/' + locale + '/atlas?x=100&y=200&z=7&zoom=0'),
      page.waitForResponse(response => response.url() === tile),
    ]);
    expect(document.status()).toBe(200);
    await checkHeaders(document, 'Atlas viewer', { csp: shellPolicy(atlas) });
    const nonce = await page.locator('script[src="/static/atlas/viewer.js"]').evaluate(script => script.nonce);
    expect(/^[a-f0-9]{64}$/.test(nonce) && document.headers()['content-security-policy'].includes("'nonce-" + nonce + "'"),
      'Atlas script carries the response CSP nonce').toBe(true);
    await expect(page.locator('select#floor')).toHaveValue('7');
    expect(response.status()).toBe(200);
    expect(response.headers()['content-type']).toBe('image/webp');
    await checkHeaders(response, 'Atlas tile');
    await expect(page.locator('p#status')).toHaveText('100, 200, 7');
  });
});
