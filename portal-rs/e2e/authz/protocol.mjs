import { expect, test } from '@playwright/test';

const id = '019a541b-a000-7000-8000-000000000001';
const cases = [
  {
    method: 'POST', path: '/api/v1/game-auth/tickets', status: 401,
    data: { protocol_version: 1 }, source: 'crates/player-api/src/lib.rs:93',
  },
  {
    method: 'POST', path: '/api/v1/native-account/coin-purchases/orders', status: 401,
    data: { version: 1, request_id: id, package_id: id, quote_id: id, character_id: null },
    source: 'crates/payments/src/coin_http.rs:89', bearerChallenge: true,
  },
  {
    method: 'GET', path: '/api/v1/native-account/coin-purchases/requests/{request_id}', status: 401,
    source: 'crates/payments/src/coin_http.rs:89', bearerChallenge: true,
  },
  {
    method: 'POST', path: '/api/v1/payments/provider-webhook',
    data: { version: 1, event_id: 'authz-missing-signature', type: 'payment_succeeded' },
  },
];
export const protocolRoutes = cases.map(({ method, path }) => [method, path]);
export const authorityRoutes = [
  ['public', 'GET', '/{lang}/atlas/bestiary/{character}'],
  ['public', 'GET', '/{lang}/stats/rankings/{kind}'],
  ['native', 'POST', '/api/v1/game-auth/device-sessions/{*rest}'],
];

export async function exerciseUnavailableAuthorities(request, locale, anonymous, token, routes) {
  const native = new URL(process.env.PORTAL_E2E_BROWSER_NATIVE_ORIGIN || 'https://127.0.0.1:18444');
  expect(native.protocol === 'https:' && native.hostname === '127.0.0.1', 'loopback native TLS origin').toBe(true);
  for (const [listener, method, path] of authorityRoutes) {
    const { guard, refusal } = routes.find(r => r.listener === listener && r.method === method && r.path === path)
      .registrations[0];
    const expected = guard.kind === 'logged-in' && anonymous ? refusal.status : 503;
    const localPath = path.replace('{lang}', locale).replace('{character}', id)
      .replace('{kind}', 'yesterday').replace('{*rest}', 'unavailable-fixture');
    const url = listener === 'native' ? new URL(localPath, native).href : localPath;
    await test.step(`${method} ${path} on ${listener}`, async () => {
      const response = await request.fetch(url, { method, headers: { 'x-csrf-token': token }, maxRedirects: 0 });
      try {
        expect(response.status(), refusal?.source ?? path).toBe(expected);
        if (listener === 'native') {
          expect(await response.json()).toEqual({ error: 'device_authorization_unavailable' });
        }
      } finally {
        await response.dispose();
      }
    });
  }
}

export async function exerciseProtocol(request, token, routes, snapshot, verifyRefusal) {
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  for (const fixture of cases) {
    const { path, method, data } = fixture;
    const route = routes.find(r => r.listener === 'public' && r.method === method && r.path === path);
    const { guard, refusal } = route.registrations[0];
    expect(guard.kind, path).toBe(path.endsWith('/provider-webhook') ? 'provider-signature' : 'bearer');
    await test.step(`${method} ${path}`, async () => {
      const before = method === 'GET' ? null : await snapshot();
      const response = await request.fetch(path.replace('{request_id}', id), {
        method, data, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
      });
      try {
        expect(response.status(), refusal?.source ?? fixture.source).toBe(refusal?.status ?? fixture.status);
        if (fixture.bearerChallenge) expect(response.headers()['www-authenticate']).toBe('Bearer');
        if (before) await verifyRefusal(before);
      } finally {
        await response.dispose();
      }
    });
  }
}
