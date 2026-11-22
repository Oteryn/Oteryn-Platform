import { createHash } from "node:crypto";
import { expect, test } from '@playwright/test';
import { connect, exchange, fixture, harness, jsonBytes, wire, withUnknown } from './transport.mjs';

const streamCases = [
  ['sanctions', '/internal/v1/support/sanctions/stream/read', 'sanction-export-v1', 'empty.json'],
  ['security-revocations', '/internal/v1/identity/security-revocations/stream/read', 'security-revocation-v1', 'empty.json'],
  ['loyalty-stream', '/internal/v1/products-entitlements/loyalty-snapshots/stream', 'loyalty-stream-v1', 'empty-response.json'],
];

for (const [peer, route, group, emptyFile] of streamCases) {
  test(`${peer} authenticates the certificate and enforces the bounded stream request`, async () => {
    const client = await connect('internal', peer);
    const wrongPeer = await connect('internal', 'bootstrap-intents');
    const body = fixture(group, 'request.json');
    try {
      const unauthorized = await exchange(wrongPeer, 'POST', route, body);
      await wire(unauthorized, 401, '');
      expect(unauthorized.headers()['cache-control']).toContain('no-store');
      for (const invalid of [withUnknown(body), jsonBytes({ ...JSON.parse(body), limit: '1' }),
        Buffer.concat([body, Buffer.alloc(257 - body.length, ' ')])]) {
        await wire(await exchange(client, 'POST', route, invalid), 400, '');
      }
      const response = await exchange(client, 'POST', route, body);
      await wire(response, 200, fixture(group, emptyFile));
      expect(response.headers()['cache-control']).toBe('no-store, private');
    } finally {
      await Promise.all([client.dispose(), wrongPeer.dispose()]);
    }
  });
}

test('the private listener rejects a connection without a client certificate', async () => {
  const client = await connect('internal');
  try {
    await expect(client.post(streamCases[0][1], {
      data: fixture('sanction-export-v1', 'request.json'),
      headers: { 'content-type': 'application/json' },
    })).rejects.toThrow(/certificate required|ECONNRESET|socket hang up/i);
  } finally {
    await client.dispose();
  }
});

const storeCases = [
  ['balance/read', 'balance-request.valid.json', 404, ''],
  ['history/read', 'history-request.valid.json', 404, ''],
  ['operations/pending', 'pending-request.valid.json', 200, '{"contract_version":1,"pending":[]}'],
  ['operations/reconcile', 'reconcile-request.valid.json', 200, '{"contract_version":1,"operations":[]}'],
  ['operations/read', 'read-request.valid.json', 404, ''],
  ['operations/receipt', 'receipt.deliver-item.committed.json', 404, ''],
];

async function storeWire(response, status, body, cacheControl = 'no-store') {
  expect(response.headers()['cache-control']).toBe(cacheControl);
  await wire(response, status, body);
}

async function rejectNonCanonicalStoreRequests(client, route, data, cacheControl) {
  const entries = Object.entries(JSON.parse(data));
  const reordered = jsonBytes(Object.fromEntries([entries[1], entries[0], ...entries.slice(2)]));
  const spaced = Buffer.concat([Buffer.from('{ '), data.subarray(1)]);
  for (const [variant, bytes] of [['unknown member', withUnknown(data)],
    ['extra whitespace', spaced], ['reordered members', reordered]]) {
    await test.step(variant, async () => {
      await storeWire(await exchange(client, 'POST', route, bytes), 400, '', cacheControl);
    });
  }
}

for (const [operation, file, status, body] of storeCases) {
  test(`Store ${operation} enforces identity and closed request framing`, async () => {
    const route = `/internal/v1/products-entitlements/store/${operation}`;
    const client = await connect('internal', 'store');
    const wrongPeer = await connect('internal', 'sanctions');
    const data = jsonBytes(JSON.parse(fixture('store-wire-v1', file)));
    try {
      await storeWire(await exchange(wrongPeer, 'POST', route, data), 401, '');
      await rejectNonCanonicalStoreRequests(client, route, data);
      await storeWire(await exchange(client, 'POST', route, data), status, body);
    } finally {
      await Promise.all([client.dispose(), wrongPeer.dispose()]);
    }
  });
}

test('Store purchase persists the fixture digest and replays its restricted-account decision', async () => {
  const route = '/internal/v1/products-entitlements/store/purchases';
  const group = 'store-wire-v1';
  const data = jsonBytes(JSON.parse(fixture(group, 'purchase-request.valid.json')));
  const client = await connect('internal', 'store');
  const wrongPeer = await connect('internal', 'sanctions');
  try {
    await storeWire(await exchange(wrongPeer, 'POST', route, data), 401, '');
    await rejectNonCanonicalStoreRequests(client, route, data);
    await storeWire(await exchange(client, 'POST', route, Buffer.alloc(1025, ' ')), 413, '');
    const response = await exchange(client, 'POST', route, data);
    expect(response.status()).toBe(200);
    const { decided_at: decidedAt } = await response.json();
    expect(decidedAt).toMatch(/^[1-9][0-9]*$/);
    expect(Math.abs(Date.now() / 1000 - Number(decidedAt))).toBeLessThan(60);
    const expected = JSON.parse(fixture(group, 'purchase-answer.purchase-disabled.json'));
    expect(createHash('sha256').update(data).digest('hex')).toBe(expected.request_digest);
    const answer = jsonBytes({ ...expected, result: 'ACCOUNT_RESTRICTED', decided_at: decidedAt });
    await storeWire(response, 200, answer);
    await storeWire(await exchange(client, 'POST', route, data), 200, answer);
    await storeWire(await exchange(client, 'POST', route, jsonBytes({ ...JSON.parse(data), quantity: 2 })), 409, '');
  } finally {
    await Promise.all([client.dispose(), wrongPeer.dispose()]);
  }
});

test('Premium snapshot authenticates its own certificate and rejects invalid bodies', async () => {
  const route = '/internal/v1/products-entitlements/premium-snapshots/read';
  const data = fixture('premium-snapshot-v1', 'request.valid.json');
  const client = await connect('internal', 'premium');
  const wrongPeer = await connect('internal', 'loyalty-snapshot');
  try {
    await wire(await exchange(wrongPeer, 'POST', route, data), 401, '');
    await wire(await exchange(client, 'POST', route, withUnknown(data)), 400, '');
    await wire(await exchange(client, 'POST', route, Buffer.alloc(257, ' ')), 400, '');
    await wire(await exchange(client, 'POST', route, data), 404, '');
    const account = harness().seed.coinAccount2.accountId;
    const request = { ...JSON.parse(data), account_id: account };
    const snapshot = await exchange(client, 'POST', route, jsonBytes(request));
    expect(snapshot.status()).toBe(200);
    const { authority_issued_at: issuedAt } = await snapshot.json();
    expect(issuedAt).toMatch(/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$/);
    const issued = Date.parse(issuedAt);
    expect(Math.abs(Date.now() - issued)).toBeLessThan(60_000);
    const iso = timestamp => new Date(timestamp).toISOString().replace('.000Z', 'Z');
    const expected = JSON.parse(fixture('premium-snapshot-v1', 'snapshot.none.json'));
    Object.assign(expected, {
      account_id: account, authority_issued_at: issuedAt,
      authority_valid_until: iso(issued + 3600_000), refresh_after: iso(issued + 2400_000),
    });
    await wire(snapshot, 200, jsonBytes(expected));
    expect(snapshot.headers()['cache-control']).toBe('no-store, private');

    const loyaltyRoute = '/internal/v1/products-entitlements/loyalty-snapshots/read';
    const loyaltyRequest = jsonBytes({ ...request, schema: 'oteryn.loyalty_snapshot_request.v1' });
    await wire(await exchange(client, 'POST', loyaltyRoute, loyaltyRequest), 401, '');
    await wire(await exchange(wrongPeer, 'POST', loyaltyRoute, withUnknown(loyaltyRequest)), 400, '');
    const loyalty = await exchange(wrongPeer, 'POST', loyaltyRoute, loyaltyRequest);
    expect(loyalty.status()).toBe(200);
    const { computed_at: computedAt } = await loyalty.json();
    expect(computedAt).toMatch(/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$/);
    expect(Math.abs(Date.now() - Date.parse(computedAt))).toBeLessThan(60_000);
    await wire(loyalty, 200, jsonBytes({
      schema: 'oteryn.loyalty_snapshot.v1', producer_revision: expected.producer_revision,
      nonce: request.nonce, account_id: account, loyalty_points: 0,
      loyalty_revision: expected.authority_revision + 1, computed_at: computedAt,
    }));

  } finally {
    await Promise.all([client.dispose(), wrongPeer.dispose()]);
  }
});

test('bootstrap reconciliation returns exactly the seeded v4 intent and digest', async () => {
  const issued = harness().seed.bootstrap;
  const route = '/internal/v1/game-auth/character-bootstrap-intents/read';
  const request = { contract_version: 4, operation_id: issued.operationId };
  const client = await connect('internal', 'bootstrap-intents');
  const wrongPeer = await connect('internal', 'sanctions');
  try {
    await wire(await exchange(wrongPeer, 'POST', route, jsonBytes(request)), 401, '');
    for (const version of [1, 2, 3, '4']) {
      await wire(await exchange(client, 'POST', route, jsonBytes({ ...request, contract_version: version })), 400, '');
    }
    await wire(await exchange(client, 'POST', route, withUnknown(jsonBytes(request))), 400, '');
    const expected = JSON.parse(fixture('character-bootstrap-intent-v4', 'intent.knight.json'));
    Object.assign(expected, {
      issuer_decision_id: issued.issuerDecisionId,
      source_revision: String(issued.sourceRevision),
      account_id: harness().seed.coinAccount1.accountId,
      issued_at_source: String(issued.issuedAtSource),
      expires_at_source: String(issued.expiresAtSource),
    });
    expect(Number(expected.expires_at_source) - Number(expected.issued_at_source)).toBe(300);
    const response = await exchange(client, 'POST', route, jsonBytes(request));
    await wire(response, 200, jsonBytes(expected));
    expect(createHash('sha256').update(await response.body()).digest('hex')).toBe(issued.intentSha256);
    await wire(await exchange(client, 'POST', route, jsonBytes(request)), 200, jsonBytes(expected));
  } finally {
    await Promise.all([client.dispose(), wrongPeer.dispose()]);
  }
});

test('restore notice preserves byte-exact replay and rejects a changed notice with the same key', async () => {
  const route = '/internal/v1/game-auth/game-restore/notice';
  const notice = JSON.parse(fixture('store-wire-v1', 'restore-notice.valid.json'));
  notice.source_authority = 'oteryn-game-e2e';
  const client = await connect('internal', 'restore-notice');
  const wrongPeer = await connect('internal', 'store');
  const data = jsonBytes(notice);
  const cacheControl = 'no-store, private';
  try {
    await storeWire(await exchange(wrongPeer, 'POST', route, data), 401, '', cacheControl);
    await rejectNonCanonicalStoreRequests(client, route, data, cacheControl);
    await storeWire(await exchange(client, 'POST', route, Buffer.alloc(513, ' ')), 413, '', cacheControl);
    const answer = jsonBytes({
      contract_version: 1, restore_event_id: notice.restore_event_id,
      status: 'ACKNOWLEDGED', reopened: 0, outstanding: 0,
    });
    await storeWire(await exchange(client, 'POST', route, data), 200, answer, cacheControl);
    await storeWire(await exchange(client, 'POST', route, data), 200, answer, cacheControl);
    await storeWire(await exchange(client, 'POST', route, jsonBytes({ ...notice, backup_label: 'changed' })), 409, '', cacheControl);
  } finally {
    await Promise.all([client.dispose(), wrongPeer.dispose()]);
  }
});

test('admin-control requires an authenticated agent identity in addition to a trusted CA', async () => {
  const client = await connect('internal', 'sanctions');
  try {
    for (const operation of ['hello', 'channel', 'results', 'heartbeat']) {
      await wire(await exchange(client, 'POST', `/internal/v1/admin-control/${operation}`, jsonBytes({})),
        403, '{"error":{"code":"ADM_FORBIDDEN","retryable":false}}');
    }
  } finally {
    await client.dispose();
  }
});

test('unconfigured native recovery locator refuses the fixture with an empty 503', async () => {
  const client = await connect('internal', 'runtime-status');
  try {
    await wire(await exchange(client, 'POST', '/internal/v1/game-auth/native-recovery-locators',
      fixture('native-gateway-protocol3-v1', 'locator-report.json')), 503, '');
  } finally {
    await client.dispose();
  }
});

const disabledRoutes = [
  ['/admin-analytics/page'],
  ['/admin-analytics/commit'],
  ['/admin-analytics/watermark'],
  ['/products-entitlements/profile-b/authority/read', 'profile-b-authority-v1', 'authority-request.json'],
  ['/products-entitlements/profile-b/changes/read', 'profile-b-authority-v1', 'changes-request.json'],
  ['/support/enforcement-commands/stream/read', 'game-enforcement-wire-v1', 'stream-request.json'],
  ['/support/enforcement-commands/commands/read', 'game-enforcement-wire-v1', 'read-request.json'],
  ['/support/enforcement-commands/reconcile/read', 'game-enforcement-wire-v1', 'reconcile-request.json'],
  ['/support/enforcement-commands/results', 'game-enforcement-wire-v1', 'result-applied.json'],
  ['/playerdata/journal/records', 'game-journal-items-v1', 'record-items-begin.valid.json'],
  ['/playerdata/journal/cursor', 'game-journal-items-v1', 'cursor-request.valid.json'],
  ['/playerdata/progress/record', 'game-journal-progress-v1', 'record-stamina.invalid.json'],
  ['/playerdata/progress/watermark', 'game-journal-progress-v1', 'watermark-baseline.valid.json'],
  ['/playerdata/journal/commands/pending'],
  ['/playerdata/journal/commands/read'],
  ['/playerdata/journal/commands/receipt'],
];

test('default-off producer routes fail closed with empty responses', async () => {
  const client = await connect('internal', 'public-projections');
  try {
    for (const [suffix, group, file] of disabledRoutes) {
      await test.step(suffix, async () => {
        await wire(await exchange(client, 'POST', '/internal/v1' + suffix,
          group ? fixture(group, file) : jsonBytes({})), 503, '');
      });
    }
  } finally {
    await client.dispose();
  }
});


test('native producer routes refuse unrelated mTLS peers with an empty 401', async () => {
  const wrongPeer = await connect('internal', 'sanctions');
  try {
    for (const operation of ['native-evidence', 'native-runtime-status', 'native-scope-assignments',
      'native-scope-revocations']) {
      await test.step(operation, async () => {
        await wire(await exchange(wrongPeer, 'POST', `/internal/v1/game-auth/${operation}`, jsonBytes({})), 401, '');
      });
    }
  } finally {
    await wrongPeer.dispose();
  }
});

test('Character commands reject an unrelated certificate and incomplete messages', async () => {
  const client = await connect('internal', 'character-commands');
  const wrongPeer = await connect('internal', 'sanctions');
  try {
    for (const operation of ['pending', 'read', 'receipt']) {
      const route = `/internal/v1/game-auth/character-commands/${operation}`;
      await test.step(operation, async () => {
        await wire(await exchange(wrongPeer, 'POST', route, jsonBytes({})), 401, '');
        await wire(await exchange(client, 'POST', route, jsonBytes({})), 400, '');
      });
    }
  } finally {
    await Promise.all([client.dispose(), wrongPeer.dispose()]);
  }
});
