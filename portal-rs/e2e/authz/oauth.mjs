import { mirrorHead } from './head.mjs';
import { createHash, randomBytes } from 'node:crypto';
import { expect, test } from '@playwright/test';
import { waitForLoginBudget } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { assertRefusalState, rateKey } from './database.mjs';

const path = '/oauth/authorize';
const tokenPath = '/oauth/token';
export const oauthRoutes = [['GET', '/oauth/authorize'], ['POST', '/oauth/authorize'], ['POST', '/oauth/token']];

export async function exerciseOAuth(request, accountId, csrf, routes, db, sessionHash) {
  const config = JSON.parse(process.env.PORTAL_OAUTH_CONFIG ?? '{}');
  const client = config.clients?.[0];
  if (!config.enabled || !client?.redirect_uris?.length) {
    throw new Error('The browser harness needs its local OAuth client configured');
  }
  const verifier = randomBytes(32).toString('base64url');
  const authorization = {
    response_type: 'code', client_id: client.client_id, redirect_uri: client.redirect_uris[0],
    scope: 'game:ticket', state: randomBytes(24).toString('base64url'), code_challenge_method: 'S256',
    code_challenge: createHash('sha256').update(verifier).digest('base64url'),
  };
  const target = `${path}?${new URLSearchParams(authorization)}`;
  const route = routes.find(r => r.listener === 'public' && r.method === 'GET' && r.path === path);
  const { guard, source } = route.registrations[0];
  expect(guard.kind).toBe('logged-in');
  await test.step(`GET ${path}`, async () => {
    await waitForLoginBudget(2);
    const response = await request.get(target, { maxRedirects: 0 });
    try {
      expect(response.status(), source).toBe(accountId ? 200 : 303);
      await mirrorHead(request, path, target, response);
      if (!accountId) expect(response.headers().location).toMatch(/^\/(pl|en)\/login$/);
    } finally {
      await response.dispose();
    }
  });
  for (const [endpoint, kind] of [[path, 'logged-in'], [tokenPath, 'oauth-grant']]) {
    const registration = routes.find(row => row.listener === 'public' && row.method === 'POST' && row.path === endpoint);
    expect(registration.registrations[0].guard.kind).toBe(kind);
  }
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  const policy = 'identity-login-source';
  const rate = { policy, key: rateKey(policy, '127.0.0.1') };
  const send = (endpoint, form) => request.post(endpoint, {
    form, headers: { origin, 'x-csrf-token': csrf }, maxRedirects: 0,
  });
  async function refuse(endpoint, form, error, eventAccount) {
    await waitForLoginBudget();
    const before = await db.snapshot(sessionHash, rate, true);
    const response = await send(endpoint, form);
    try {
      expect(response.status()).toBe(400);
      expect((await response.json()).error).toBe(error);
      assertRefusalState(before, await db.snapshot(sessionHash, rate, true), [],
        [['oauth_refused', eventAccount], ['oauth_http_refused', null]]);
    } finally {
      await response.dispose();
    }
  }
  const consent = { ...authorization, decision: 'approve', _csrf: csrf };
  await test.step('POST /oauth/authorize rejects invalid consent', () => refuse(path,
    accountId ? { ...consent, redirect_uri: new URL('/unregistered-oauth-callback', origin).href } : consent,
    accountId ? 'invalid_request' : 'invalid_grant', accountId ?? null));

  let code = randomBytes(32).toString('base64url');
  if (accountId) {
    await test.step('POST /oauth/authorize issues a bound code', async () => {
      await waitForLoginBudget();
      const response = await send(path, consent);
      try {
        expect(response.status()).toBe(302);
        const location = new URL(response.headers().location, origin);
        code = location.searchParams.get('code');
        expect(Boolean(code), 'consent issued a code').toBe(true);
        expect(location.searchParams.get('state') === authorization.state, 'state echoed').toBe(true);
        location.searchParams.delete('code');
        location.searchParams.delete('state');
        expect(location.href).toBe(authorization.redirect_uri);
      } finally {
        await response.dispose();
      }
    });
  }
  const exchange = { grant_type: 'authorization_code', client_id: authorization.client_id,
    redirect_uri: authorization.redirect_uri, code, code_verifier: verifier };
  await test.step('POST /oauth/token rejects an invalid grant', () => refuse(tokenPath,
    { ...exchange, code_verifier: randomBytes(32).toString('base64url') }, 'invalid_grant', null));
  if (accountId) {
    await test.step('POST /oauth/token consumes the same code with the right proof', async () => {
      await waitForLoginBudget();
      const response = await send(tokenPath, exchange);
      try {
        expect(response.status()).toBe(200);
        const result = await response.json();
        expect(result.scope).toBe(authorization.scope);
        expect(Boolean(result.access_token && result.refresh_token), 'exchange issued both tokens').toBe(true);
        const hash = createHash('sha256').update(code).digest();
        const [row] = await query(
          'SELECT account_id,used_at IS NOT NULL AS used FROM oauth_authorization_codes WHERE code_hash=$1', [hash]);
        expect(row).toEqual({ account_id: accountId, used: true });
      } finally {
        await response.dispose();
      }
    });
  }
}
