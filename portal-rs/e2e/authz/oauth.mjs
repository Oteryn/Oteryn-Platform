import { createHash, randomBytes } from 'node:crypto';
import { expect, test } from '@playwright/test';

export const oauthRoutes = [
  ['GET', '/oauth/authorize'],
  ['POST', '/oauth/authorize'],
  ['POST', '/oauth/token'],
];

export async function exerciseOAuth(request, token, accountId, routes, snapshot, verifyRefusal) {
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  const config = JSON.parse(process.env.PORTAL_OAUTH_CONFIG ?? '{}');
  const client = config.clients?.[0];
  if (!config.enabled || !client?.redirect_uris?.length) {
    throw new Error('The browser harness needs its local OAuth client configured');
  }
  const verifier = randomBytes(32).toString('base64url');
  const state = randomBytes(24).toString('base64url');
  const authorization = {
    response_type: 'code', client_id: client.client_id, redirect_uri: client.redirect_uris[0],
    scope: 'game:ticket', state, code_challenge_method: 'S256',
    code_challenge: createHash('sha256').update(verifier).digest('base64url'),
  };
  let code = randomBytes(32).toString('base64url');
  for (const [method, path] of oauthRoutes) {
    const exchange = path === '/oauth/token';
    const { guard, source } = routes.find(r => r.listener === 'public' && r.method === method && r.path === path)
      .registrations[0];
    expect(guard.kind).toBe(exchange ? 'oauth-grant' : 'logged-in');
    await test.step(`${method} ${path}`, async () => {
      const mutation = method === 'POST';
      const url = mutation ? path : `${path}?${new URLSearchParams(authorization)}`;
      const before = mutation && !accountId ? await snapshot() : null;
      const form = exchange ? {
        grant_type: 'authorization_code', client_id: authorization.client_id,
        redirect_uri: authorization.redirect_uri, code, code_verifier: verifier,
      } : { ...authorization, decision: 'approve', _csrf: token };
      const response = await request.fetch(url, {
        method, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
        ...(mutation && { form }),
      });
      try {
        if (method === 'GET') {
          expect(response.status(), source).toBe(accountId ? 200 : 303);
          if (!accountId) expect(response.headers().location).toMatch(/^\/(pl|en)\/login$/);
          return;
        }
        expect(response.status(), source).toBe(accountId ? exchange ? 200 : 302 : 400);
        if (!accountId) {
          expect((await response.json()).error).toBe('invalid_grant');
          await verifyRefusal(before, ['oauth_refused']);
        } else if (exchange) {
          const tokens = await response.json();
          expect(tokens.scope).toBe('game:ticket');
          expect(tokens.token_type).toBe('bearer');
          expect(Boolean(tokens.access_token && tokens.refresh_token), 'the valid code was exchanged').toBe(true);
        } else {
          const destination = new URL(response.headers().location);
          code = destination.searchParams.get('code');
          expect(Boolean(code), 'the owner received a one-time authorization code').toBe(true);
          expect(destination.searchParams.get('state') === state, 'the OAuth state was echoed').toBe(true);
        }
      } finally {
        await response.dispose();
      }
    });
  }
}
