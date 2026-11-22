import { mirrorHead } from './head.mjs';
import { randomBytes } from 'node:crypto';
import { expect, test } from '@playwright/test';

const path = '/oauth/authorize';
export const oauthRoutes = [['GET', path]];

export async function exerciseOAuth(request, accountId, routes) {
  const config = JSON.parse(process.env.PORTAL_OAUTH_CONFIG ?? '{}');
  const client = config.clients?.[0];
  if (!config.enabled || !client?.redirect_uris?.length) {
    throw new Error('The browser harness needs its local OAuth client configured');
  }
  const authorization = new URLSearchParams({
    response_type: 'code', client_id: client.client_id, redirect_uri: client.redirect_uris[0],
    scope: 'game:ticket', state: randomBytes(24).toString('base64url'), code_challenge_method: 'S256',
    code_challenge: randomBytes(32).toString('base64url'),
  });
  const route = routes.find(r => r.listener === 'public' && r.method === 'GET' && r.path === path);
  const { guard, source } = route.registrations[0];
  expect(guard.kind).toBe('logged-in');
  await test.step(`GET ${path}`, async () => {
    const response = await request.get(`${path}?${authorization}`, { maxRedirects: 0 });
    try {
      expect(response.status(), source).toBe(accountId ? 200 : 303);
      await mirrorHead(request, path, `${path}?${authorization}`, response);
      if (!accountId) expect(response.headers().location).toMatch(/^\/(pl|en)\/login$/);
    } finally {
      await response.dispose();
    }
  });
}
