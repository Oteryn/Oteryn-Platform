import { createHash } from 'node:crypto';
import { expect, test } from '@playwright/test';
import { pool } from '../helpers.mjs';
import { connect, exchange, fixture, jsonBytes, wire, withUnknown } from './transport.mjs';

const coin = name => fixture('coin-wire-v1', name);
const sender = '019a65f0-0000-7000-8000-000000000001';
const prefix = '/internal/v1/coins/';

test('coin wire manifest vectors retain their byte counts and digests', () => {
  const manifest = JSON.parse(coin('manifest.json'));
  for (const [file, vector] of Object.entries(manifest.valid)) {
    const bytes = coin(file);
    expect(bytes.length, file).toBe(vector.canonical_bytes);
    expect(createHash('sha256').update(bytes).digest('hex'), file).toBe(vector.sha256);
  }
});

async function seedOpeningBalance() {
  // Account creation uses the seed CLI. The CLI has no wallet funding command.
  // Seed only the pre-existing credit; every tested effect goes through HTTP.
  const db = pool();
  const client = await db.connect();
  try {
    await client.query('BEGIN');
    await client.query('INSERT INTO wallet_accounts(account_id) VALUES($1)', [sender]);
    const { rows: [{ id }] } = await client.query(
      "INSERT INTO wallet_operations(id,kind,account_id,amount) VALUES(uuidv7(),'adjustment',$1,105) RETURNING id",
      [sender],
    );
    await client.query(
      'INSERT INTO wallet_entries(id,operation_id,account_id,available_delta,reserved_delta) VALUES(uuidv7(),$1,$2,105,0)',
      [id, sender],
    );
    await client.query('COMMIT');
  } catch (error) {
    await client.query('ROLLBACK');
    throw error;
  } finally {
    client.release();
    await db.end();
  }
}

async function receiptTemplate(response, file, operationId) {
  expect(response.status()).toBe(200);
  expect((await response.body()).length).toBeLessThanOrEqual(2048);
  const actual = await response.json();
  expect(actual.receipt.occurred_at).toMatch(/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d{1,9})?Z$/);
  expect(Math.abs(Date.now() - Date.parse(actual.receipt.occurred_at))).toBeLessThan(60_000);
  const expected = JSON.parse(coin(file));
  expected.receipt.operation_id = operationId;
  expected.receipt.occurred_at = actual.receipt.occurred_at;
  await wire(response, 200, jsonBytes(expected));
  expect(response.headers()['cache-control']).toBe('no-store');
  return expected.receipt;
}

test('coin routes enforce peer scope, exact mutation replay and global idempotency keys', async () => {
  await seedOpeningBalance();
  const client = await connect('internal', 'coins');
  const wrongPeer = await connect('internal', 'sanctions');
  const route = prefix + 'transfers';
  const data = coin('transfers-request.json');
  const transfer = JSON.parse(data);
  try {
    const manifest = JSON.parse(coin('manifest.json'));
    for (const { method, path, request } of manifest.routes) {
      await wire(await exchange(wrongPeer, method, path,
        method === 'POST' ? coin(request) : undefined), 401, '');
    }
    for (const invalid of ['invalid-extra.json', 'invalid-negative.json', 'invalid-overflow.json',
      'invalid-same-account.json', 'invalid-zero.json', 'invalid-bad-key.json'].map(coin)) {
      await wire(await exchange(client, 'POST', route, invalid), 422, coin('invalid.json'));
    }
    await wire(await exchange(client, 'POST', route, Buffer.alloc(1025, ' ')), 422, coin('invalid.json'));

    const original = await receiptTemplate(await exchange(client, 'POST', route, data),
      'transfers-ok.json', transfer.operation_id);
    await wire(await exchange(client, 'POST', route, data), 200, jsonBytes({ result: 'replay', receipt: original }));
    await wire(await exchange(client, 'POST', route, jsonBytes({ ...transfer, amount: '11' })),
      409, coin('idempotency_conflict.json'));
    await wire(await exchange(client, 'GET', prefix + 'operations/' + transfer.operation_id),
      200, jsonBytes({ result: 'ok', receipt: original }));
    await wire(await exchange(client, 'GET', prefix + `accounts/${sender}/balance`), 200, coin('balance-ok.json'));

    const history = await exchange(client, 'GET', prefix + `accounts/${sender}/history?limit=1`);
    expect(history.status()).toBe(200);
    const { next_cursor: cursor } = await history.json();
    expect(cursor).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
    const expectedHistory = JSON.parse(coin('history-ok.json'));
    expectedHistory.next_cursor = cursor;
    expectedHistory.entries[0].entry_id = cursor;
    expectedHistory.entries[0].occurred_at = original.occurred_at;
    await wire(history, 200, jsonBytes(expectedHistory));
    await wire(await exchange(client, 'GET', prefix + `accounts/${sender}/history?unexpected=1`),
      422, coin('invalid.json'));

    for (const [operation, number] of [['offers', 4], ['settlements', 5], ['cancellations', 6]]) {
      const input = JSON.parse(coin(`${operation}-request.json`));
      const operationId = `019a65f0-0000-7000-8000-00000000000${number}`;
      const marketRoute = prefix + 'market/' + operation;
      input.operation_id = operationId;
      await wire(await exchange(client, 'POST', marketRoute, withUnknown(jsonBytes(input))), 422, coin('invalid.json'));
      const expected = JSON.parse(coin(`${operation}-ok.json`));
      expected.receipt.transferable_after = operation === 'cancellations' ? '85' : '65';
      const response = await exchange(client, 'POST', marketRoute, jsonBytes(input));
      expect(response.status()).toBe(200);
      const actual = await response.json();
      expect(Math.abs(Date.now() - Date.parse(actual.receipt.occurred_at))).toBeLessThan(60_000);
      expected.receipt.occurred_at = actual.receipt.occurred_at;
      expected.receipt.operation_id = operationId;
      await wire(response, 200, jsonBytes(expected));
      await wire(await exchange(client, 'POST', marketRoute, jsonBytes(input)),
        200, jsonBytes({ result: 'replay', receipt: expected.receipt }));
    }
  } finally {
    await Promise.all([client.dispose(), wrongPeer.dispose()]);
  }
});
