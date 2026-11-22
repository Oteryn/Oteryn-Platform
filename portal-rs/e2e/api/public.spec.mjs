import { createHmac } from 'node:crypto';
import { expect, test } from '@playwright/test';
import { connect, exchange, fixture, jsonBytes, wire, withUnknown } from './transport.mjs';

test('public native ticket and character endpoints require OAuth access', async () => {
  const client = await connect('public');
  try {
    for (const authorization of [undefined, 'Bearer invalid-e2e-access-token']) {
      await wire(await exchange(client, 'POST', '/api/v1/game-auth/tickets',
        fixture('native-oauth-ticket-v1', 'ticket-request.json'), authorization),
      401, fixture('native-oauth-ticket-v1', 'ticket-unauthenticated.json'));
      await wire(await exchange(client, 'GET', '/api/v1/game-auth/native-characters',
        undefined, authorization), 401, fixture('native-characters-v1', 'characters-unauthenticated.json'));
    }
  } finally {
    await client.dispose();
  }
});

for (const [suffix, file, scheme] of [
  ['', 'enroll.json', 'Bearer'],
  ['/native-characters', 'rotate.json', 'OterynDevice'],
  ['/tickets', 'rotate.json', 'OterynDevice'],
  ['/revoke', 'rotate.json', 'OterynDevice'],
]) {
  test(`device ${suffix || 'enrollment'} refuses unverified HTTP transport`, async () => {
    const client = await connect('public');
    try {
      const response = await exchange(client, 'POST', `/api/v1/game-auth/device-sessions${suffix}`,
        fixture('remembered-device-v1', file), `${scheme} invalid-e2e-credential`);
      await wire(response, 503, fixture('remembered-device-v1', 'denied.json'));
      expect(response.headers()['pragma']).toBe('no-cache');
      expect(response.headers()['cache-control']).toContain('no-store');
    } finally {
      await client.dispose();
    }
  });
}

test('signed provider events bind idempotency to the exact delivered bytes', async () => {
  const client = await connect('public');
  const route = '/api/v1/payments/provider-webhook';
  const event = { version: 1, event_id: 'contract-ignored-1', type: 'e2e.ignored' };
  const timestamp = String(Math.floor(Date.now() / 1000));
  const data = jsonBytes(event);
  async function signed(body, signedBody = body) {
    const signature = createHmac('sha256', process.env.PORTAL_API_WEBHOOK_SECRET)
      .update(timestamp + '.').update(signedBody).digest('hex');
    try {
      return await client.post(route, { data: body, headers: {
        'content-type': 'application/json',
        'oteryn-webhook-timestamp': timestamp,
        'oteryn-webhook-signature': 'v1=' + signature,
      } });
    } catch {
      throw new Error('Provider webhook request failed at the transport layer');
    }
  }
  try {
    await wire(await exchange(client, 'POST', route, data), 401, '');
    await wire(await signed(jsonBytes({ ...event, type: 'e2e.changed' }), data), 401, '');
    await wire(await signed(Buffer.alloc(16_385, ' ')), 413, '');
    await wire(await signed(withUnknown(jsonBytes({
      version: 1, event_id: 'contract-refund-1', type: 'refund',
      order_reference: 'test:unknown-order', amount: { minor_units: 499, currency: 'EUR' },
    }))), 400, '');
    const answer = { event_id: event.event_id, outcome: 'ignored', replay: false, version: 1 };
    await wire(await signed(data), 200, jsonBytes(answer));
    await wire(await signed(data), 200, jsonBytes({ ...answer, replay: true }));
    await wire(await signed(Buffer.concat([data, Buffer.from(' ')])), 409, '');
  } finally {
    await client.dispose();
  }
});

test('verified device HTTPS checks credentials, closed bodies and the byte ceiling', async () => {
  const client = await connect('native');
  const group = 'remembered-device-v1';
  const route = '/api/v1/game-auth/device-sessions';
  const { device_credential: syntheticCredential } = JSON.parse(fixture(group, 'enroll-response.json'));
  try {
    const body = fixture(group, 'enroll.json');
    const bearer = 'Bearer invalid-e2e-access-token';
    await wire(await exchange(client, 'POST', route, body, bearer), 401, fixture(group, 'denied.json'));
    await wire(await exchange(client, 'POST', route, withUnknown(body), bearer), 400, fixture(group, 'invalid.json'));
    await wire(await exchange(client, 'POST', route, Buffer.alloc(1025, ' '), bearer), 413, fixture(group, 'invalid.json'));
    for (const operation of ['native-characters', 'tickets', 'revoke']) {
      await wire(await exchange(client, 'POST', route + '/' + operation,
        fixture(group, 'rotate.json'), 'OterynDevice ' + syntheticCredential), 401, fixture(group, 'denied.json'));
    }
  } finally {
    await client.dispose();
  }
});
