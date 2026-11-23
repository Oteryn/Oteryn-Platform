import { test, expect } from '@playwright/test';
import { uuid7 } from '../game-sim-helpers.mjs';
import { bearer, empty, fixture, gateway, json, nativeTls, providerEvent, publishDownloads, requestSource, sha256, waitForPortal } from '../gameapi-helpers.mjs';

const versioned = 'public, max-age=31536000, immutable, no-transform';
const current = 'no-store, no-transform';
const gatewayFixtures = 'gateway/tests/fixtures/game_client_native_login';
const nativeRequest = JSON.parse(fixture(`${gatewayFixtures}/request.json`));
const errors = JSON.parse(fixture(`${gatewayFixtures}/errors.json`));

async function gatewayError(response, code, echo = nativeRequest.attempt_ref) {
  const vector = errors.find(entry => {
    const body = JSON.parse(entry.body);
    return body.error.code === code && body.attempt_ref === echo;
  });
  expect(vector, code).toBeDefined();
  expect(response.status()).toBe(vector.status);
  expect(await response.text()).toBe(vector.body + '\n');
  expect(response.headers()['content-type']).toBe('application/json');
  expect(response.headers()['cache-control']).toBe('no-store, no-cache, must-revalidate, private');
}

test.beforeAll(async ({ request }) => {
  await waitForPortal(request);
});

test('gateway health, readiness, version, login validation, issuer refusal and protocol 3 gate', async ({ request }, info) => {
  const headers = requestSource(info, 10);
  const post = (path, data, overrides = {}) => request.post(gateway + path, {
    data, headers: { ...headers, ...overrides },
  });
  await json(await request.get(gateway + '/health'), 200, { status: 'ok' });
  await json(await request.get(gateway + '/ready'), 200, { status: 'ready' });
  await json(await request.get(gateway + '/version'), 200, { service: 'oteryn-game-gateway', version: 'e2e-gameapi' });
  await json(await post('/v1/login?unexpected=1', nativeRequest), 400, { error: 'invalid_request' });
  await json(await post('/v1/login', { protocol_version: 1 }), 400, { error: 'invalid_request' });
  await json(await post('/v1/login', ' '.repeat(16_385)), 400, { error: 'invalid_request' });
  await gatewayError(await post('/v1/login', nativeRequest, { 'content-type': 'text/plain' }), 'NATIVE_LOGIN_REQUEST_MALFORMED');
  await gatewayError(await post('/v1/login', { ...nativeRequest, unknown: true }), 'NATIVE_LOGIN_REQUEST_MALFORMED');
  await gatewayError(await post('/v1/login', { ...nativeRequest, protocol_version: 4 }), 'NATIVE_LOGIN_UNSUPPORTED_VERSION');
  await gatewayError(await post('/v1/login', { ...nativeRequest, offer: {
    ...nativeRequest.offer, transports: [{ protocol_major: 9, transport_profile: 1, alpn: 'oteryn-game/9' }],
  } }), 'NATIVE_LOGIN_OFFER_UNSUPPORTED');
  await gatewayError(await post('/v1/login', nativeRequest), 'NATIVE_LOGIN_AUTHENTICATION_REQUIRED');
  await gatewayError(await post('/v1/recover', { protocol_version: 3, attempt_ref: nativeRequest.attempt_ref }), 'NATIVE_LOGIN_UNSUPPORTED_VERSION');
  await gatewayError(await post('/v1/recover/revoke', { protocol_version: 3 }), 'NATIVE_LOGIN_UNSUPPORTED_VERSION', null);
});

test('gateway applies the 2 KiB body boundary and 30-request source budget', async ({ request }, info) => {
  const headers = { ...requestSource(info, 11), 'content-type': 'application/json' };
  const path = gateway + '/v1/login';
  const data = JSON.stringify(nativeRequest).padEnd(2049);
  await Promise.all(Array.from({ length: 30 }, async () => {
    await gatewayError(await request.post(path, { headers, data }), 'NATIVE_LOGIN_REQUEST_MALFORMED', null);
  }));
  const limited = await request.post(path, { headers, data: JSON.stringify(nativeRequest) });
  await gatewayError(limited, 'NATIVE_LOGIN_RATE_LIMITED');
  expect(Number(limited.headers()['retry-after'])).toBeGreaterThanOrEqual(1);
  expect(Number(limited.headers()['retry-after'])).toBeLessThanOrEqual(30);
});

function publicRefusal(locale, status) {
  const messages = {
    en: { 404: 'Page not found', 503: 'Temporarily unavailable' },
    pl: { 404: 'Nie znaleziono strony', 503: 'Usługa chwilowo niedostępna' },
  };
  return { status, error: messages[locale][status] };
}

test('client directory and channel probes are absent on public listeners', async ({ request }, info) => {
  const locale = info.project.metadata.locale;
  const headers = { accept: 'application/json', 'accept-language': locale };
  const missing = publicRefusal(locale, 404);
  // The contracts are present in docs/contracts; the Platform has no handlers.
  for (const path of ['/v1/client/directory', '/v1/client/channel-probes']) {
    const anonymous = await request.get(path, { headers });
    await json(anonymous, 404, missing);
    expect(anonymous.headers()['cache-control']).toBe('no-store');
    expect(anonymous.headers()['content-language']).toBe(locale);
    await json(await request.get(path, { headers: { ...headers, ...bearer('unknown') } }), 404, missing);
    await json(await request.get(nativeTls + path, { headers }), 404, missing);
  }
  await empty(await request.get(gateway + '/v1/probe'), 404);
});

const internalRoutes = [
  'game-auth/native-admissions', 'game-auth/native-recoveries', 'game-auth/native-recoveries/revoke',
  'game-auth/character-bootstrap-intents/read', 'game-auth/game-restore/notice',
  'game-auth/native-account-characters', 'game-auth/native-account-characters/watermark',
  'game-auth/character-commands/pending', 'game-auth/character-commands/read', 'game-auth/character-commands/receipt',
  'game-auth/account-closure', 'game-auth/account-closure/watermark',
  'bazaar/commands/pending', 'bazaar/commands/read', 'bazaar/commands/reconcile',
  'bazaar/transferability', 'bazaar/status', 'bazaar/watermark', 'game-events',
  'playerdata/journal/records', 'playerdata/journal/cursor', 'playerdata/progress/record', 'playerdata/progress/watermark',
  'coins/transfers', 'coins/market/offers', 'coins/market/settlements', 'coins/market/cancellations',
  'products-entitlements/premium-snapshots/read', 'products-entitlements/loyalty-snapshots/read',
  'products-entitlements/loyalty-snapshots/stream', 'products-entitlements/profile-b/authority/read',
  'products-entitlements/profile-b/changes/read', 'products-entitlements/store/operations/pending',
  'products-entitlements/store/operations/read', 'products-entitlements/store/operations/receipt',
  'products-entitlements/store/operations/reconcile', 'products-entitlements/store/balance/read',
  'products-entitlements/store/history/read', 'products-entitlements/store/purchases',
  'support/sanctions/stream/read', 'support/enforcement-commands/stream/read',
  'support/enforcement-commands/commands/read', 'support/enforcement-commands/reconcile/read',
  'support/enforcement-commands/results', 'identity/security-revocations/stream/read',
  'game-public/world-event-outcome', 'game-public/guild-season', 'game-public/league-watermark',
];

test('private Game operations refuse the public HTTP and native TLS listeners', async ({ request }, info) => {
  const locale = info.project.metadata.locale;
  const headers = { accept: 'application/json', 'accept-language': locale, ...bearer('public-client') };
  const missing = publicRefusal(locale, 404);
  const v2Internal = [
    'game-auth/character-commands/pending', 'game-auth/character-commands/read', 'game-auth/character-commands/receipt',
    'game-auth/character-services/state', 'game-auth/character-services/watermark',
    'game-public/character-skills', 'game-public/character-achievements', 'game-public/character-equipment',
    'game-public/character-outfit', 'game-public/character-online-session', 'game-public/world-daily-kills',
  ];
  const hidden = [
    ...internalRoutes.map(route => ['/internal/v1/' + route, 'POST']),
    ...v2Internal.map(route => ['/internal/v2/' + route, 'POST']),
    ...[`operations/${uuid7()}`, `accounts/${uuid7()}/balance`, `accounts/${uuid7()}/history`]
      .map(route => ['/internal/v1/coins/' + route, 'GET']),
  ];
  for (let offset = 0; offset < hidden.length; offset += 4) {
    await Promise.all(hidden.slice(offset, offset + 4).flatMap(([path, method]) =>
      ['', nativeTls].map(async base => {
        const options = method === 'POST' ? { method, headers, data: {} } : { method, headers };
        await json(await request.fetch(base + path, options), 404, missing);
      }),
    ));
  }
  const unconfigured = [
    'native-recovery-locators', 'native-evidence', 'native-runtime-status',
    'native-scope-assignments', 'native-scope-revocations',
  ];
  for (const route of unconfigured) {
    for (const base of ['', nativeTls]) {
      const response = await request.post(base + '/internal/v1/game-auth/' + route, { headers, data: {} });
      if (route === 'native-recovery-locators') {
        await empty(response, 503, 'no-store, private');
      } else {
        await json(response, 503, publicRefusal(locale, 503));
        expect(response.headers()['cache-control']).toBe('no-store, private');
      }
    }
  }
  for (const route of ['highscores','character-profile','character-deaths','guild','house','world-online','watermark']) {
    await empty(await request.post('/internal/v1/game-public/' + route, { data: {} }), 401, 'no-store, private');
  }
  const v2Cache = {
    'character-deaths': 'no-store, private',
    guild: 'no-store, private',
    house: 'private, no-store',
    watermark: 'no-store, private',
    'world-daily': 'private, no-store, max-age=0',
  };
  for (const [route, cache] of Object.entries(v2Cache)) {
    await empty(await request.post('/internal/v2/game-public/' + route, { data: {} }), 401, cache);
  }
  for (const route of ['native-admissions', 'native-recoveries', 'native-recoveries/revoke']) {
    const privatePath = 'http://127.0.0.1:18091/internal/v1/game-auth/' + route;
    const response = await request.post(privatePath, { headers: bearer('public-client'), data: {} });
    await json(response, 401, { error: 'unauthorized_service' });
    expect(response.headers()['cache-control']).toBe('no-store, private');
  }
});

test('account summary, feed and watches preserve their authority refusals', async ({ request }) => {
  const privateCache = 'no-store, private';
  for (const path of ['/api/v1/account/summary', '/api/v1/account/summary?extra=1', '/api/v1/feed', '/api/v1/feed?limit=0']) {
    for (const headers of [{}, bearer('unverified')]) {
      const response = await request.get(path, { headers });
      await empty(response, 503, privateCache);
      expect(response.headers().vary).toBe('Authorization');
      expect(response.headers().pragma).toBe('no-cache');
    }
  }
  const watches = '/api/v1/stats/watches';
  for (const method of ['GET', 'POST', 'DELETE']) {
    await empty(await request.fetch(watches, { method }), 401, 'private, no-store');
    await empty(await request.fetch(watches, { method, headers: bearer('unverified') }), 401, 'private, no-store');
  }
  for (const method of ['POST', 'DELETE']) {
    expect((await request.fetch(watches, { method, data: ' '.repeat(1025) })).status()).toBe(413);
  }
});

test('world registry JSON and conditional reads preserve fixture identity and ETag', async ({ request }) => {
  const path = '/api/v1/worlds';
  const response = await request.get(path);
  expect(response.status()).toBe(200);
  expect(response.headers()['content-type']).toContain('application/json');
  expect(response.headers()['cache-control']).toBe('no-store');
  const body = await response.json();
  expect(body.schema).toBe(1);
  expect(Number.isSafeInteger(body.generated_at)).toBe(true);
  expect(body.worlds).toContainEqual({
    id: '0192a000-0000-7000-8000-000000000012', name: 'Local Bazaar World', status: 'unknown', players_online: null,
  });
  const etag = response.headers().etag;
  expect(etag).toMatch(/^W\/"[a-f0-9]{64}"$/);
  for (const validator of [etag, etag.slice(2), '*']) {
    const unchanged = await request.get(path, { headers: { 'if-none-match': validator } });
    await empty(unchanged, 304, 'no-store');
    expect(unchanged.headers().etag).toBe(etag);
  }
});

test('signed stable and beta updates preserve exact bytes, cache classes and read-only methods', async ({ request }) => {
  await publishDownloads();
  for (const channel of ['stable', 'beta']) {
    const base = `/updates/${channel}`;
    const bundle = `downloads/tests/fixtures/origin-v1/${channel}`;
    const policy = fixture(`${bundle}/policy-v1.json`);
    const hash = sha256(policy).digest('hex');
    const targets = [
      ['/policy-v1.json', policy, current],
      [`/targets/${hash}.channels/${channel}/policy-v1.json`, policy, versioned],
      ['/metadata/timestamp.json', fixture(`${bundle}/metadata/timestamp.json`), current],
    ];
    for (const role of ['root', 'snapshot', 'targets', channel]) {
      const bytes = fixture(`${bundle}/metadata/1.${role}.json`);
      targets.push([`/metadata/${role}.json`, bytes, current], [`/metadata/1.${role}.json`, bytes, versioned]);
    }
    for (const [path, bytes, cache] of targets) {
      const [response, head, write] = await Promise.all([
        request.get(base + path),
        request.head(base + path),
        request.post(base + path, { data: '{}' }),
      ]);
      expect(response.status(), base + path).toBe(200);
      expect((await response.body()).equals(bytes), base + path).toBe(true);
      expect(response.headers()['cache-control']).toBe(cache);
      expect(response.headers()['content-type']).toBe('application/json');
      expect(response.headers()['x-content-type-options']).toBe('nosniff');
      await empty(head, 200, cache);
      await empty(write, 405);
    }
    await empty(await request.get(base + '/metadata/99.root.json'), 404, 'no-store');
    await empty(await request.get(base + '/metadata/root.txt'), 400, 'no-store');
    await empty(await request.get(base + '/targets/missing/policy-v1.json'), 404, 'no-store');
  }
  for (const path of ['/policy-v1.json', '/metadata/root.json', '/targets/policy-v1.json']) {
    await empty(await request.get('/updates/preview' + path), 400, 'no-store');
  }
});

test('provider webhook verifies signatures, enforces size, and deduplicates exact events', async ({ request }) => {
  const path = '/api/v1/payments/provider-webhook';
  const event = { version: 1, event_id: uuid7(), type: 'probe' };
  const data = JSON.stringify(event);
  const signed = (body, offset = 0) => providerEvent(request, body, offset);
  await empty(await request.post(path, { data }), 401, 'no-store');
  await empty(await request.post(path, { data, headers: {
    'oteryn-webhook-timestamp': String(Math.floor(Date.now() / 1000)),
    'oteryn-webhook-signature': 'v1=' + '0'.repeat(64),
  } }), 401, 'no-store');
  await empty(await signed(data, -301), 401, 'no-store');
  await empty(await signed('x'.repeat(16_385)), 413);
  await empty(await signed('{}'), 400, 'no-store');
  const refund = {
    version: 1, event_id: uuid7(), type: 'refund', order_reference: uuid7(),
    amount: { minor_units: 499, currency: 'PLN' },
  };
  const unknown = JSON.stringify(refund);
  await json(await signed(unknown), 422, {
    version: 1, event_id: refund.event_id, code: 'PROVIDER_WEBHOOK_UNKNOWN_ORDER',
  });
  await json(await signed(unknown), 200, {
    version: 1, event_id: refund.event_id, outcome: 'unknown_order', replay: true,
  });
  const result = { version: 1, event_id: event.event_id, outcome: 'ignored' };
  await json(await signed(data), 200, { ...result, replay: false });
  await json(await signed(data), 200, { ...result, replay: true });
  await empty(await signed(JSON.stringify({ ...event, type: 'changed' })), 409, 'no-store');
});
