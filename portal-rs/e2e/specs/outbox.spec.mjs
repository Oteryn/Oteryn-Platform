// Run with the default local launcher or start.sh --outbox.
// Game pulls Store operations, revocations and character intents through mTLS.
// Payment funding uses the worker registry's wallet.top_up job.
import crypto from 'node:crypto';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { test as base, expect } from '../fixtures.mjs';
import { login, password, pool, seedPasswordRange, submit } from '../helpers.mjs';
import { paidTopup } from '../staff-commerce-helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { gameJson, gamePeer, nil, portalCommand, seededAccount, seededPlayer, sourceTime } from '../outbox-helpers.mjs';
import { uuid7 } from '../game-sim-helpers.mjs';

// Keep the default suite's catalogue and character feeds on its own database.
const outboxEnv = process.env.PORTAL_E2E_OUTBOX_ENV
  ? JSON.parse(await fs.readFile(process.env.PORTAL_E2E_OUTBOX_ENV, 'utf8'))
  : null;
const test = base.extend({
  outboxEnvironment: [async ({}, use) => {
    const previous = Object.fromEntries(Object.keys(outboxEnv ?? {}).map(key => [key, process.env[key]]));
    Object.assign(process.env, outboxEnv);
    try {
      await use();
    } finally {
      for (const [key, value] of Object.entries(previous)) {
        if (value === undefined) delete process.env[key];
        else process.env[key] = value;
      }
    }
  }, { auto: true }],
});
if (outboxEnv) test.use({ baseURL: outboxEnv.PORTAL_E2E_ORIGIN });

async function changePassword(page, lang, account) {
  await page.goto(`/${lang}/account/sudo`);
  await submit(page, 'main form', { current_password: account.password });
  await page.goto(`/${lang}/password/change`);
  const next = password();
  await seedPasswordRange(next);
  await submit(page, 'main form', {
    current_password: account.password,
    password: next,
    password_confirmation: next,
  });
  await expect(page).toHaveURL(/\/login$/);
  return next;
}

async function fundThroughWorker(page, lang, accountId, verifyRetry = false) {
  const db = pool();
  const job = () => query(`SELECT id, state, attempts, last_error,
      extract(epoch FROM run_after - updated_at)::float8 AS delay
    FROM outbox WHERE kind='wallet.top_up' AND payload::jsonb->>'account_id'=$1`, [accountId]);
  try {
    let funding;
    let retry;
    if (verifyRetry) {
      const lock = await db.connect();
      try {
        await lock.query('BEGIN');
        // Permit the provider's reads and webhook commit, but block the worker's
        // wallet write until its real transaction deadline expires.
        await lock.query('LOCK TABLE wallet_operations IN SHARE MODE');
        funding = paidTopup(page, lang, db);
        funding.catch(() => {});
        await expect.poll(async () => {
          const rows = await job();
          [retry] = rows;
          return rows;
        }, { intervals: [50], timeout: 25_000 }).toEqual([{
          id: expect.any(String),
          state: 'pending', attempts: 1, last_error: 'top-up wallet refused', delay: expect.any(Number),
        }]);
        expect(retry.delay).toBeGreaterThanOrEqual(1);
        expect(retry.delay).toBeLessThanOrEqual(2);
      } finally {
        await lock.query('ROLLBACK');
        lock.release();
      }
    } else {
      funding = paidTopup(page, lang, db);
    }
    await funding;
    await expect.poll(job).toEqual([{
      id: retry?.id ?? expect.any(String),
      state: 'done', attempts: verifyRetry ? 2 : 1, last_error: null, delay: expect.any(Number),
    }]);
  } finally {
    await db.end();
  }
}

test('credential change publishes one replayable native security revocation', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const read = gamePeer('security-revocations');
  const route = '/internal/v1/identity/security-revocations/stream/read';
  const request = { schema: 'oteryn.security_revocation_stream_request.v1', after: null, limit: 500 };
  const earlier = await seededPlayer(page, lang);
  await changePassword(page, lang, earlier);
  const account = await seededPlayer(page, lang);
  const before = await gameJson(read, route, request);
  expect(before.items.at(-1)).toMatchObject({ account_id: earlier.id, cause: 1 });
  expect(before.next_cursor).toMatch(/^v1:[1-9]\d*$/);
  expect(before.items.filter(item => item.account_id === account.id)).toEqual([]);

  const secondSession = await browser.newContext({ baseURL: new URL(page.url()).origin, ignoreHTTPSErrors: true });
  try {
    const other = await secondSession.newPage();
    await other.goto(new URL(`/${lang}/login`, page.url()).href);
    await login(other, lang, account);
    expect((await other.goto(`/${lang}/password/change`)).status()).toBe(200);

    const next = await changePassword(page, lang, account);
    await other.goto(`/${lang}/password/change`);
    await expect(other).toHaveURL(/\/login$/);

    const fromCursor = { ...request, after: before.next_cursor };
    const revocations = await gameJson(read, route, fromCursor);
    expect(revocations.items).toEqual([{
      revocation_id: expect.stringMatching(/^[\da-f-]{36}$/),
      account_id: account.id,
      native_security_generation: 2,
      cause: 1,
      effective_at_ms: expect.any(Number),
    }]);
    expect(revocations.next_cursor).toMatch(/^v1:[1-9]\d*$/);

    const poison = await read(route, { ...request, after: 'v1:0' });
    expect(poison.status).toBe(400);
    expect(poison.raw).toBe('');
    expect(await gameJson(read, route, fromCursor)).toEqual(revocations);
    expect(await gameJson(read, route, { ...request, after: revocations.next_cursor })).toEqual({
      schema: 'oteryn.security_revocation_stream_page.v1', items: [], next_cursor: null,
    });
    const [published] = revocations.items;
    expect(await query(`SELECT revocation_id,native_security_generation,cause,effective_at_ms,position
      FROM identity_security_revocations WHERE account_id=$1`, [account.id])).toEqual([{
      revocation_id: published.revocation_id,
      native_security_generation: '2',
      cause: 1,
      effective_at_ms: String(published.effective_at_ms),
      position: revocations.next_cursor.slice(3),
    }]);
    await login(page, lang, { ...account, password: next });
    await expect(page).toHaveURL(new RegExp(`/${lang}/$`));
  } finally {
    await secondSession.close();
  }
});

test('character services retain pending commands, reconcile receipts and settle a rename once', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const account = await seededPlayer(page, lang);
  const character = uuid7();
  const world = uuid7();
  const service = gamePeer('character-services');
  const command = gamePeer('character-commands');
  const lcfa = gamePeer('lcfa');
  const portfolioPath = '/internal/v1/game-auth/native-account-characters';
  const servicePath = '/internal/v2/game-auth/character-services';
  const commandPath = '/internal/v2/game-auth/character-commands';
  const authority = process.env.PORTAL_CHARACTER_COMMAND_SOURCE_AUTHORITY;
  const projection = process.env.PORTAL_CHARACTER_SERVICES_PROJECTION_AUTHORITY;
  const serviceHead = operation => ({ contract_version: 2, operation, source_authority: projection });
  const commandHead = operation => ({ contract_version: 2, operation, source_authority: authority });
  const portfolioHead = operation => ({
    contract_version: 1, operation, source_authority: process.env.OTERYN_LCFA_SOURCE_AUTHORITY,
  });
  const name = 'Aldric';
  const url = `/${lang}/account/characters/${character}/services`;
  const messages = lang === 'pl' ? {
    pending: 'Oczekuje na potwierdzenie gry',
    committed: 'Potwierdzone przez grę',
    rejected: 'Odrzucone przez grę',
  } : {
    pending: 'Pending game confirmation',
    committed: 'Confirmed by the game',
    rejected: 'Refused by the game',
  };

  async function publishState(revision, characterRevision, deletion = null, currentName = name) {
    const at = sourceTime();
    await gameJson(service, `${servicePath}/state`, {
      ...serviceHead('PublishCharacterServiceStateV2'),
      account_id: account.id, character_id: character, projection_epoch: '1',
      projection_revision: revision, source_observed_at: at, character_revision: characterRevision,
      world_id: world, name: currentName, sex: null, lifecycle: deletion ? 'DELETION_PENDING' : 'ACTIVE',
      deletion_operation_id: deletion?.id ?? null,
      deletion_scheduled_at: deletion?.start ?? null,
      deletion_effective_at: deletion?.end ?? null,
    });
    const watermark = { projection_epoch: '1', complete_through: at, observed_at: at };
    await gameJson(lcfa, `${portfolioPath}/watermark`, { ...portfolioHead('PublishProjectionWatermarkV1'), ...watermark });
    await gameJson(service, `${servicePath}/watermark`, { ...serviceHead('PublishCharacterServiceWatermarkV2'), ...watermark });
  }

  async function status(id, expected) {
    expect((await page.goto(url)).status()).toBe(200);
    await expect(page.locator('[aria-labelledby=history] li').filter({ hasText: id })).toHaveText(`${id}: ${expected}`);
  }

  async function request(action, commandName, fields = {}) {
    expect((await page.goto(url)).status()).toBe(200);
    const selector = `form:has(input[name=action][value=${action}])`;
    const requestId = await page.locator(`${selector} input[name=request_id]`).inputValue();
    const accepted = page.waitForResponse(r => r.request().method() === 'POST'
      && r.request().isNavigationRequest() && new URL(r.url()).pathname === url);
    await submit(page, selector, { ...fields, confirm: true });
    expect((await accepted).status()).toBe(202);
    const [{ operation_id: id }] = await query(
      'SELECT operation_id FROM character_service_orders WHERE request_id=$1', [requestId],
    );
    await expect(page.locator('body')).toHaveText(lang === 'pl'
      ? 'Zgłoszenie oczekuje na potwierdzenie przez grę. Ponowienie tego samego zgłoszenia nie spowoduje kolejnej opłaty.'
      : 'Your request is pending confirmation by the game. No further charge will be taken for a retry of this request.');

    const pending = await gameJson(command, `${commandPath}/pending`, {
      ...commandHead('ListPendingCharacterCommandsV2'),
      after_source_revision: '0', after_operation_id: nil, max_entries: 32,
    });
    expect(pending.pending.map(entry => [entry.operation_id, entry.command])).toContainEqual([id, commandName]);
    const read = {
      ...commandHead('ReadCharacterCommandIntentV2'),
      issuer_authority: 'OTERYN_PLATFORM_CHARACTER_AUTHORITY', command: commandName, operation_id: id,
    };
    const intent = await command(`${commandPath}/read`, read);
    expect(intent.status).toBe(200);
    expect(JSON.parse(intent.raw)).toMatchObject({ operation_id: id, account_id: account.id, character_id: character, command: commandName });
    await status(id, messages.pending);
    return { id, read, intent };
  }

  await fundThroughWorker(page, lang, account.id);
  const observed = sourceTime();
  await gameJson(lcfa, portfolioPath, {
    ...portfolioHead('PublishAccountCharactersV1'),
    account_id: account.id, projection_epoch: '1', projection_revision: '1', source_observed_at: observed,
    characters: [{ character_id: character, world_id: world, name, availability: 'AVAILABLE' }],
  });
  await page.goto(`/${lang}/account/sudo`);
  await submit(page, 'main form', { current_password: account.password });
  await publishState('1', '8');
  const renamed = 'Aldric Vale';
  const rename = await request('rename', 'RenameCharacter', { requested_name: renamed });
  const financialState = async () => query(
    'SELECT financial_state,amount FROM character_service_orders WHERE operation_id=$1', [rename.id],
  );
  expect(await financialState()).toEqual([{ financial_state: 'RESERVED', amount: '70' }]);
  const renameReceipt = {
    ...commandHead('PublishCharacterCommandReceiptV2'),
    command: 'RenameCharacter', operation_id: rename.id, outcome: 'COMMITTED', result_code: 'CHAR_CMD_OK',
    character_id: character, account_id: account.id, character_revision: '9', projection_epoch: '1',
    projection_revision: '2', previous_name: name, name: renamed, recovery_generation: '1',
    decided_at: sourceTime(),
  };
  const renameAck = await gameJson(command, `${commandPath}/receipt`, renameReceipt);
  expect(renameAck).toEqual({ contract_version: 2, result: 'accepted' });
  expect(await gameJson(command, `${commandPath}/receipt`, renameReceipt)).toEqual(renameAck);
  expect(await financialState()).toEqual([{ financial_state: 'SETTLED', amount: '70' }]);
  const [{ reservation_id: reservation, settlement_id: settlement }] = await query(
    'SELECT reservation_id,settlement_id FROM character_service_orders WHERE operation_id=$1', [rename.id],
  );
  expect(await query('SELECT id,status,amount FROM wallet_reservations WHERE account_id=$1', [account.id]))
    .toEqual([{ id: reservation, status: 'captured', amount: '70' }]);
  expect(await query(`SELECT id,kind,reference_id,amount FROM wallet_operations
    WHERE account_id=$1 ORDER BY created_at,id`, [account.id])).toEqual([
    { id: expect.any(String), kind: 'top_up', reference_id: account.id, amount: '1234' },
    { id: reservation, kind: 'reserve', reference_id: null, amount: '70' },
    { id: settlement, kind: 'capture', reference_id: reservation, amount: '70' },
  ]);
  expect(await query(`SELECT operation_id,available_delta,reserved_delta FROM wallet_entries
    WHERE account_id=$1 ORDER BY created_at,id`, [account.id])).toEqual([
    { operation_id: expect.any(String), available_delta: '1234', reserved_delta: '0' },
    { operation_id: reservation, available_delta: '-70', reserved_delta: '70' },
    { operation_id: settlement, available_delta: '0', reserved_delta: '-70' },
  ]);
  await publishState('2', '9', null, renamed);
  await status(rename.id, messages.committed);
  await expect(page.getByRole('heading', { name: renamed, exact: true })).toBeVisible();
  const schedule = await request('schedule', 'ScheduleCharacterDeletion');

  // An invalid decision must leave the real pending intent and browser status intact.
  const decided = sourceTime();
  const deletion = { id: schedule.id, start: decided, end: String(Number(decided) + 3600) };
  const receipt = {
    ...commandHead('PublishCharacterCommandReceiptV2'),
    command: 'ScheduleCharacterDeletion', operation_id: schedule.id, outcome: 'COMMITTED',
    result_code: 'CHAR_CMD_OK', character_id: character, account_id: account.id, character_revision: '10',
    projection_epoch: '1', projection_revision: '3', lifecycle: 'DELETION_PENDING',
    deletion_operation_id: schedule.id, deletion_scheduled_at: deletion.start, deletion_effective_at: deletion.end,
    recovery_generation: '1', decided_at: decided,
  };
  const poison = await command(`${commandPath}/receipt`, { ...receipt, unexpected: true });
  expect(poison.status).toBe(400);
  await status(schedule.id, messages.pending);
  expect(await query('SELECT state,receipt_json FROM character_commands WHERE id=$1', [schedule.id]))
    .toEqual([{ state: 'PENDING', receipt_json: null }]);
  expect((await command(`${commandPath}/read`, schedule.read)).raw).toBe(schedule.intent.raw);

  const ack = await gameJson(command, `${commandPath}/receipt`, receipt);
  expect(ack).toEqual({ contract_version: 2, result: 'accepted' });
  await publishState('3', '10', deletion, renamed);
  await status(schedule.id, messages.committed);
  expect(await gameJson(command, `${commandPath}/receipt`, receipt)).toEqual(ack);
  expect(await query('SELECT state,receipt_json FROM character_commands WHERE id=$1', [schedule.id]))
    .toEqual([{ state: 'TERMINAL', receipt_json: JSON.stringify(receipt) }]);

  const cancel = await request('cancel', 'CancelCharacterDeletion');
  await gameJson(command, `${commandPath}/receipt`, {
    ...commandHead('PublishCharacterCommandReceiptV2'),
    command: 'CancelCharacterDeletion', operation_id: cancel.id, outcome: 'REJECTED',
    result_code: 'CHAR_CMD_INTENT_EXPIRED', recovery_generation: '1', decided_at: sourceTime(),
  });
  await status(cancel.id, messages.rejected);
  await status(schedule.id, messages.committed);

  const retry = await request('cancel', 'CancelCharacterDeletion');
  expect(retry.id).not.toBe(cancel.id);
  const cancelReceipt = {
    ...commandHead('PublishCharacterCommandReceiptV2'),
    command: 'CancelCharacterDeletion', operation_id: retry.id, outcome: 'COMMITTED', result_code: 'CHAR_CMD_OK',
    character_id: character, account_id: account.id, character_revision: '11', projection_epoch: '1',
    projection_revision: '4', lifecycle: 'ACTIVE', deletion_operation_id: schedule.id,
    recovery_generation: '1', decided_at: sourceTime(),
  };
  const cancelAck = await gameJson(command, `${commandPath}/receipt`, cancelReceipt);
  expect(cancelAck).toEqual({ contract_version: 2, result: 'accepted' });
  expect(await gameJson(command, `${commandPath}/receipt`, cancelReceipt)).toEqual(cancelAck);
  await publishState('4', '11', null, renamed);
  await status(retry.id, messages.committed);
  await expect(page.locator('form:has(input[name=action][value=cancel])')).toHaveCount(0);
});

test('Store delivery receipts reconcile an item and premium entitlement without duplicate effects', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const account = await seededPlayer(page, lang);
  await fundThroughWorker(page, lang, account.id, true);
  const operator = await seededAccount(lang);
  // Provision the first catalog operator as in admin.spec.mjs. Catalog and
  // delivery state are created through the operator CLI and Game exchange.
  await query(`INSERT INTO account_roles(account_id,role_id)
    SELECT $1,id FROM roles WHERE key='platform_admin' AND built_in`, [operator.id]);
  const fixtureFile = new URL('../../../docs/contracts/fixtures/store-price-list-v1/price-list.valid.json', import.meta.url);
  const catalog = JSON.parse(await fs.readFile(fixtureFile, 'utf8'));
  const item = catalog.offers[0];
  const premium = {
    ...item,
    offer_key: 'test-premium',
    units: [{ ...item.units[0], product_id: 'oteryn.test_premium', unit_kind: 'PREMIUM_TIME' }],
  };
  catalog.offers.push(premium);
  const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'portal-e2e-outbox-'));
  let catalogRevision;
  try {
    const file = path.join(directory, 'catalog.json');
    await fs.writeFile(file, JSON.stringify(catalog));
    catalogRevision = await portalCommand(['catalog', 'import', file, '--actor', operator.id]);
    expect(catalogRevision).toMatch(/^[a-f0-9]{64}$/);
    expect(await portalCommand(['catalog', 'activate', catalogRevision, '--actor', operator.id])).toBe(catalogRevision);
  } finally {
    await fs.rm(directory, { recursive: true, force: true });
  }

  const game = gamePeer('store');
  const premiumGame = gamePeer('premium');
  const root = '/internal/v1/products-entitlements/store';
  const operations = `${root}/operations`;
  const head = operation => ({
    contract_version: 1, operation, source_authority: process.env.PRODUCTS_ENTITLEMENTS_STORE_SOURCE_AUTHORITY,
  });
  const issuer = 'OTERYN_PLATFORM_STORE_AUTHORITY';
  const cursor = { after_source_revision: '0', after_operation_id: nil, max_entries: 32 };
  const pending = () => gameJson(game, `${operations}/pending`, { ...head('ListPendingStoreOperationsV1'), ...cursor });
  const reconcile = async id => {
    const result = await gameJson(game, `${operations}/reconcile`, { ...head('ListStoreOperationStatesV1'), ...cursor });
    return result.operations.find(entry => entry.operation_id === id);
  };
  const digest = raw => crypto.createHash('sha256').update(raw).digest('hex');
  const character = uuid7();
  const world = uuid7();

  async function buy(offer, balance) {
    const request = {
      ...head('PurchaseStoreOfferV1'),
      purchase_request_id: uuid7(), account_id: account.id, character_id: character, world_id: world,
      catalog_revision: catalogRevision, offer_key: offer.offer_key, currency: 'COINS',
      unit_price: offer.unit_price, quantity: 1,
    };
    const result = await gameJson(game, `${root}/purchases`, request);
    expect(result).toMatchObject({
      purchase_request_id: request.purchase_request_id, result: 'ACCEPTED',
      total_price: '25', available_after: balance,
      units: [{ unit_kind: offer.units[0].unit_kind }],
    });
    expect(await gameJson(game, `${root}/purchases`, request)).toEqual(result);
    const [unit] = result.units;
    return unit;
  }

  async function complete(kind, id, entitlement, facts, { interrupt = false } = {}) {
    expect((await pending()).pending).toContainEqual({
      operation_kind: kind, operation_id: id, source_revision: expect.stringMatching(/^\d+$/),
    });
    const request = {
      ...head('ReadStoreOperationIntentV1'), issuer_authority: issuer, operation_kind: kind, operation_id: id,
    };
    const read = await game(`${operations}/read`, request);
    expect(read.status).toBe(200);
    const intent = JSON.parse(read.raw);
    expect(intent).toMatchObject({ operation_id: id, entitlement_id: entitlement, account_id: account.id });
    const receipt = {
      ...head('PublishStoreDeliveryReceiptV1'), issuer_authority: issuer, operation_kind: kind,
      operation_id: id, binding_digest: digest(read.raw), outcome: 'COMMITTED', result_code: 'STORE_OK',
      account_id: account.id, entitlement_id: entitlement, product_id: intent.product_id,
      product_version: intent.product_version, ...facts(intent), recovery_generation: '1', decided_at: sourceTime(),
    };
    if (interrupt) {
      // Delay receipt processing at the real Store issuer fence. The fixture
      // only holds a lock and never writes a Store operation or receipt.
      const db = pool();
      const lock = await db.connect();
      try {
        await lock.query('BEGIN');
        await lock.query('SELECT pg_advisory_xact_lock(29,1)');
        const transient = await game(`${operations}/receipt`, receipt);
        expect(transient.status).toBe(503);
        expect(transient.raw).toBe('');
        const waiting = async () => {
          const { rows } = await lock.query(`SELECT count(*)::int AS waiting FROM pg_locks waiter
            JOIN pg_locks owner USING (locktype, database, classid, objid, objsubid)
            WHERE owner.pid=pg_backend_pid() AND owner.locktype='advisory'
              AND owner.granted AND NOT waiter.granted`);
          return rows[0].waiting;
        };
        const timedOut = expect(game(`${operations}/receipt`, receipt, { timeout: 1_500 }))
          .rejects.toMatchObject({ code: 'ABORT_ERR' });
        await expect.poll(waiting, { intervals: [25], timeout: 1_000 }).toBe(1);
        await timedOut;
        // Wait for the server's own deadline before releasing the lock so the
        // abandoned exchange cannot race the explicit redelivery.
        await expect.poll(waiting).toBe(0);
      } finally {
        await lock.query('ROLLBACK');
        lock.release();
        await db.end();
      }
    }
    const poisoned = await game(`${operations}/receipt`, { ...receipt, binding_digest: '0'.repeat(64) });
    expect(poisoned.status).toBe(409);
    expect(poisoned.raw).toBe('');
    expect(await reconcile(id)).toMatchObject({ state: 'PENDING', receipt_digest: null, recovery_generation: null });
    expect(await game(`${operations}/read`, request)).toEqual(expect.objectContaining({ status: 200, raw: read.raw }));

    const ack = await gameJson(game, `${operations}/receipt`, receipt);
    expect(ack).toEqual({ contract_version: 1, result: 'accepted' });
    expect(await gameJson(game, `${operations}/receipt`, receipt)).toEqual(ack);
    expect(await reconcile(id)).toMatchObject({
      state: 'COMMITTED', receipt_digest: digest(JSON.stringify(receipt)), recovery_generation: '1',
    });
    expect((await pending()).pending.map(entry => entry.operation_id)).not.toContain(id);
    expect(await query('SELECT count(*)::int AS receipts FROM store_receipts WHERE operation_id=$1', [id]))
      .toEqual([{ receipts: 1 }]);
  }

  const itemUnit = await buy(item, '1209');
  await complete('DELIVER', itemUnit.delivery_operation_id, itemUnit.entitlement_id, intent => ({
    delivered_quantity: intent.quantity,
  }), { interrupt: true });
  const claims = await query("SELECT operation_id FROM store_operations WHERE entitlement_id=$1 AND operation_kind='CLAIM'", [
    itemUnit.entitlement_id,
  ]);
  expect(claims).toHaveLength(1);
  await complete('CLAIM', claims[0].operation_id, itemUnit.entitlement_id, intent => ({
    character_id: character, world_id: world, minted_quantity: intent.quantity,
    minted_units_digest: digest(`${intent.operation_id}:${intent.quantity}`),
  }));
  expect(await query('SELECT state FROM store_fulfilment_units WHERE entitlement_id=$1', [itemUnit.entitlement_id]))
    .toEqual([{ state: 'CLAIMED' }]);

  const premiumPage = `/${lang}/panel/premium`;
  const premiumStatus = page.locator('main dl dd').first();
  expect((await page.goto(premiumPage)).status()).toBe(200);
  await expect(premiumStatus).toHaveText(lang === 'pl' ? 'Brak premium' : 'No premium');
  const premiumUnit = await buy(premium, '1184');
  expect((await page.reload()).status()).toBe(200);
  await expect(premiumStatus).toHaveText(lang === 'pl' ? 'Aktywne' : 'Active');
  await complete('DELIVER', premiumUnit.delivery_operation_id, premiumUnit.entitlement_id, intent => ({
    delivered_quantity: intent.quantity,
  }));
  const [grant] = await query('SELECT effective_until,delta_seconds FROM premium_ledger WHERE account_id=$1', [account.id]);
  expect(await query('SELECT count(*)::int AS grants FROM premium_ledger WHERE account_id=$1', [account.id]))
    .toEqual([{ grants: 1 }]);
  expect(grant.delta_seconds).toBe('432000');
  const nonce = crypto.randomBytes(16).toString('hex');
  const snapshot = await gameJson(premiumGame, '/internal/v1/products-entitlements/premium-snapshots/read', {
    schema: 'oteryn.premium_snapshot_request.v1', account_id: account.id, nonce,
  });
  expect(snapshot).toMatchObject({
    account_id: account.id, nonce, entitlement_state: 'ACTIVE',
  });

  expect(Date.parse(snapshot.effective_until)).toBe(Math.floor(grant.effective_until.getTime() / 1000) * 1000);
  expect((await page.reload()).status()).toBe(200);
  await expect(premiumStatus).toHaveText(lang === 'pl' ? 'Aktywne' : 'Active');
  await expect(page.locator('html')).toHaveAttribute('lang', lang);
  await expect(page.locator('main')).toContainText(grant.effective_until.toISOString().slice(0, 10));
  expect((await page.goto(`/${lang}/shop/history`)).status()).toBe(200);
  await expect(page.locator('main dl dd').first()).toHaveText('1184');
});
