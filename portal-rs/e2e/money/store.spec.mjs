import { createHash, randomUUID } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import https from 'node:https';
import os from 'node:os';
import path from 'node:path';
import { expect, test } from '../fixtures.mjs';
import { pool } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { uuid7 } from '../game-sim-helpers.mjs';

import {
  balance, burst, creditEvidence, expectStatuses, loginSeeded, parallel, paymentEvent,
  portal, portalCommand, seedAccount, topUp, webhook,
} from './fixtures.mjs';

const contracts = path.resolve(portal, '../docs/contracts/fixtures');
const store = '/internal/v1/products-entitlements/store';
const issuer = 'OTERYN_PLATFORM_STORE_AUTHORITY';
const storeAuthority = 'oteryn-game/store-consumer';
const storeIdentity = 'store.money.e2e.oteryn.test';

function storeFixture(name) {
  return JSON.parse(readFileSync(path.join(contracts, 'store-wire-v1', name), 'utf8'));
}

function privateStoreClient() {
  expect(process.env.PRODUCTS_ENTITLEMENTS_STORE_MTLS_CLIENT_IDENTITY).toBe(storeIdentity);
  expect(process.env.PRODUCTS_ENTITLEMENTS_STORE_SOURCE_AUTHORITY).toBe(storeAuthority);
  const origin = new URL(`https://${process.env.OTERYN_INTERNAL_TLS_BIND}`);
  expect(origin.hostname).toBe('127.0.0.1');
  const ca = process.env.OTERYN_INTERNAL_TLS_CLIENT_CA_FILE;
  const directory = mkdtempSync(path.join(os.tmpdir(), 'portal-e2e-money-store-'));
  let agent;
  try {
    const key = path.join(directory, 'client.key');
    const csr = path.join(directory, 'client.csr');
    const cert = path.join(directory, 'client.pem');
    const extensions = path.join(directory, 'client.ext');
    writeFileSync(extensions, `subjectAltName=DNS:${storeIdentity}\nextendedKeyUsage=clientAuth\n`);
    execFileSync('openssl', ['req', '-new', '-newkey', 'ec', '-pkeyopt', 'ec_paramgen_curve:P-256',
      '-nodes', '-keyout', key, '-out', csr, '-subj', `/CN=${storeIdentity}`], { stdio: 'ignore' });
    execFileSync('openssl', ['x509', '-req', '-in', csr, '-CA', ca,
      '-CAkey', path.join(path.dirname(ca), 'ca-key.pem'), '-set_serial', `0x${randomUUID().replaceAll('-', '')}`,
      '-out', cert, '-days', '1', '-extfile', extensions], { stdio: 'ignore' });
    agent = new https.Agent({ ca: readFileSync(ca), key: readFileSync(key), cert: readFileSync(cert),
      servername: 'localhost', minVersion: 'TLSv1.3' });
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
  return {
    close: () => agent.destroy(),
    post(route, payload) {
      const body = JSON.stringify(payload);
      return new Promise((resolve, reject) => {
        const request = https.request(new URL(store + route, origin), {
          agent,
          method: 'POST',
          headers: { 'content-type': 'application/json', 'content-length': Buffer.byteLength(body) },
        }, response => {
          const chunks = [];
          response.on('data', chunk => chunks.push(chunk));
          response.on('error', reject);
          response.on('end', () => resolve({ status: response.statusCode, body: Buffer.concat(chunks) }));
        });
        request.on('error', reject);
        request.setTimeout(15_000, () => request.destroy(new Error('Store mTLS request timed out')));
        request.end(body);
      });
    },
  };
}

test('parallel Store purchases and delivery receipts commit one debit and grant after a timeout', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const buyer = seedAccount(lang);
  const operator = seedAccount(lang);
  await loginSeeded(page, lang, buyer);
  const payment = await topUp(page, lang);
  expect((await webhook(page, paymentEvent(payment))).status).toBe(204);
  await creditEvidence(payment.id, buyer.id);

  await query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'", [operator.id]);
  const revision = portalCommand(['catalog', 'import',
    path.join(contracts, 'store-price-list-v1/price-list.valid.json'), '--actor', operator.id]);
  expect(portalCommand(['catalog', 'activate', revision, '--actor', operator.id])).toBe(revision);

  const game = privateStoreClient();
  try {
    const purchase = {
      ...storeFixture('purchase-request.valid.json'),
      purchase_request_id: uuid7(),
      account_id: buyer.id,
      character_id: uuid7(),
      world_id: uuid7(),
      catalog_revision: revision,
      offer_key: 'test-supply',
      unit_price: '25',
    };
    const purchases = await burst(() => game.post('/purchases', purchase));
    expectStatuses(purchases, { 200: parallel });
    const answer = JSON.parse(purchases[0].body);
    expect(answer).toMatchObject({ result: 'ACCEPTED', total_price: '25', available_after: '1209' });
    expect(answer.units).toHaveLength(1);
    for (const response of purchases) expect(response.body.equals(purchases[0].body)).toBe(true);
    expect((await game.post('/purchases', { ...purchase, quantity: 2 })).status).toBe(409);
    const [{ delivery_operation_id: operation, entitlement_id: entitlement }] = answer.units;
    const read = await game.post('/operations/read', {
      contract_version: 1,
      operation: 'ReadStoreOperationIntentV1',
      source_authority: storeAuthority,
      issuer_authority: issuer,
      operation_kind: 'DELIVER',
      operation_id: operation,
    });
    expect(read.status).toBe(200);
    const intent = JSON.parse(read.body);
    expect(intent).toMatchObject({ entitlement_id: entitlement, product_id: 'oteryn.test_supply' });
    const receipt = {
      ...storeFixture('receipt.deliver-item.committed.json'),
      operation_id: operation,
      binding_digest: createHash('sha256').update(read.body).digest('hex'),
      account_id: buyer.id,
      entitlement_id: entitlement,
      product_id: intent.product_id,
      product_version: intent.product_version,
      delivered_quantity: 5,
      decided_at: String(Math.floor(Date.now() / 1000)),
    };
    const db = pool();
    const lock = await db.connect();
    try {
      await lock.query('BEGIN');
      await lock.query('SELECT operation_id FROM store_operations WHERE operation_id=$1 FOR UPDATE', [operation]);
      expect((await game.post('/operations/receipt', receipt)).status).toBe(503);
      expect(await query('SELECT state FROM store_operations WHERE operation_id=$1', [operation]))
        .toEqual([{ state: 'PENDING' }]);
      expect(await query('SELECT state FROM store_fulfilment_units WHERE entitlement_id=$1', [entitlement]))
        .toEqual([{ state: 'DELIVERY_PENDING' }]);
      expect(await query('SELECT receipt_id FROM store_receipts WHERE operation_id=$1', [operation])).toEqual([]);
    } finally {
      await lock.query('ROLLBACK');
      lock.release();
      await db.end();
    }
    const changedReceipt = { ...receipt, decided_at: String(Number(receipt.decided_at) - 1) };
    const deliveries = await burst(index => game.post('/operations/receipt', index % 2 === 0 ? receipt : changedReceipt));
    expectStatuses(deliveries, { 200: parallel / 2, 409: parallel / 2 });
    const receipts = await query('SELECT outcome, receipt_bytes FROM store_receipts WHERE operation_id=$1', [operation]);
    expect(receipts).toHaveLength(1);
    expect(receipts[0].outcome).toBe('COMMITTED');
    const winner = JSON.parse(receipts[0].receipt_bytes);
    for (const [index, response] of deliveries.entries()) {
      const sent = index % 2 === 0 ? receipt : changedReceipt;
      expect(response.status).toBe(sent.decided_at === winner.decided_at ? 200 : 409);
      if (response.status === 200) expect(JSON.parse(response.body)).toEqual({ contract_version: 1, result: 'accepted' });
    }
    expect(await query('SELECT state FROM store_operations WHERE operation_id=$1', [operation]))
      .toEqual([{ state: 'COMMITTED' }]);
    expect(await query('SELECT state FROM store_fulfilment_units WHERE entitlement_id=$1', [entitlement]))
      .toEqual([{ state: 'DELIVERED' }]);
    expect(await query(
      'SELECT order_id, total_price FROM store_orders WHERE account_id=$1', [buyer.id],
    )).toEqual([{ order_id: answer.order_id, total_price: '25' }]);
    await balance(page, lang, buyer.id, 1209);
  } finally {
    game.close();
  }
});
