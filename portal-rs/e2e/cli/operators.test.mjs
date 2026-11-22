import assert from 'node:assert/strict';
import { createHash, generateKeyPairSync } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { after, before, test } from 'node:test';
import {
  account, audit, grant, id, json, password as newPassword, portal, query, refused, run, scratch, seed,
  seedRefused, start, stop, success,
} from './support.mjs';

before(start);
after(stop);

const repository = path.resolve(portal, '..');
const bootstrapEnv = {
  GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_ENABLED: 'true',
  GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_TTL_SECONDS: '300',
};
const jsonFile = (name, contents) => {
  const file = path.join(scratch, name);
  fs.writeFileSync(file, contents, { mode: 0o600 });
  return file;
};
const intentFields = {
  requestedName: 'Aldric Vale', requestedSex: 'Female', requestedVocation: 'Druid',
  profileRevision: 'profile:1', rulesetRevision: 'ruleset:1',
  contentRevision: 'content:1', starterTemplateRevision: 'starter:1',
};

async function whileSuspended(actor, verify) {
  await query("UPDATE accounts SET status='suspended' WHERE id=$1", [actor]);
  try {
    verify();
  } finally {
    await query("UPDATE accounts SET status='active' WHERE id=$1", [actor]);
  }
}

test('migrate records the applied versions and preserves them on rerun', async () => {
  const migrations = () => query('SELECT version, description, installed_on, success, checksum FROM _sqlx_migrations ORDER BY version');
  const before = await migrations();
  const expectedVersions = fs.readdirSync(path.join(portal, 'migrations'))
    .filter(file => /^\d+_.*\.sql$/.test(file) && !file.endsWith('.down.sql'))
    .map(file => file.split('_')[0]).sort();
  assert.ok(expectedVersions.length > 0);
  assert.deepEqual(before.map(row => row.version), expectedVersions);
  assert.ok(before.every(row => row.success));
  for (let attempt = 0; attempt < 2; attempt++) {
    const messages = success(['migrate']).trimEnd().split('\n').map(line => json(line).fields.message);
    assert.deepEqual(messages, [
      'relation "_sqlx_migrations" already exists, skipping',
      'migrations applied',
    ]);
  }
  const missing = run(['migrate'], 1, { env: { OTERYN_DATABASE_URL: undefined } });
  assert.equal(missing.stderr, '');
  assert.equal(json(missing.stdout).fields.error, 'missing environment variable OTERYN_DATABASE_URL');
  const wrong = run(['migrate', '--dry-run'], 2);
  assert.equal(wrong.stdout, '');
  assert.equal(wrong.stderr, 'usage: oteryn-portal migrate\n');
  const unconfigured = run(['migrate', '--dry-run'], 2, { emptyEnv: true });
  assert.equal(unconfigured.stdout, '');
  assert.equal(unconfigured.stderr, 'usage: oteryn-portal migrate\n');
  assert.deepEqual(await migrations(), before);
});

test('seed refuses non-testing environments, malformed JSON, extra flags and unknown entities', () => {
  const refusal = (...args) => refused(...args, 'stdout');
  refusal(['seed', 'world', '{}'], 1, 'unavailable',
    'seed runs only with OTERYN_ENV=testing', { env: { OTERYN_ENV: 'production' } });
  refusal(['seed', 'world', '{broken'], 2, 'bad_input', 'arguments are not JSON', {});
  refusal(['seed', 'world', '{}', '--extra'], 2, 'bad_input', 'expected one JSON object argument', {});
  seedRefused('unknown', {}, 2, 'bad_input', 'unknown entity "unknown"');
  seedRefused('native-evidence', {}, 3, 'not_applicable',
    'evidence is observed live; the first observation bootstraps the retained witness');
});

test('seed account and MFA persist the account while enrollment secrets are issued once', async () => {
  const player = await account();
  const { accountId, email, password } = player;
  const state = () => query('SELECT id,email,status,locale,email_verified_at FROM accounts WHERE id=$1', [accountId]);
  const rows = await state();
  assert.equal(rows.length, 1);
  assert.deepEqual({ ...rows[0], email_verified_at: Boolean(rows[0].email_verified_at) },
    { id: accountId, email, status: 'active', locale: 'en', email_verified_at: true });
  assert.equal(seed('account', player).accountId, accountId);
  assert.deepEqual(await state(), rows);
  seedRefused('account', { ...player, email: 'outside@example.com' }, 2, 'bad_input',
    'email must end with @example.invalid');

  const mfa = { accountId, password };
  seedRefused('account-mfa', { ...mfa, accountId: await id() }, 2, 'bad_input',
    'accountId is not an active verified account with this password');
  const mfaState = () => query(`SELECT
    (SELECT count(*)::int FROM account_mfa_totp WHERE account_id=$1) AS enrollments,
    (SELECT count(*)::int FROM account_recovery_codes WHERE account_id=$1) AS codes`, [accountId]);
  const mfaAudit = () => query(`SELECT action,(before='{}' AND after='{}') AS empty_details
    FROM audit_log WHERE actor=$1 AND target='mfa' ORDER BY id`, [accountId]);
  const initialAudit = await mfaAudit();
  assert.deepEqual(await mfaState(), [{ enrollments: 0, codes: 0 }]);
  seedRefused('account-mfa', { ...mfa, password: newPassword() }, 2, 'bad_input',
    'accountId is not an active verified account with this password');
  assert.deepEqual(await mfaState(), [{ enrollments: 0, codes: 0 }]);
  assert.deepEqual(await state(), rows);
  // Identity intentionally audits credential refusals without recording enrollment material.
  assert.deepEqual(await mfaAudit(), [...initialAudit, { action: 'mfa.refused', empty_details: true }]);

  const enrolled = seed('account-mfa', mfa);
  assert.ok(enrolled.accountId === accountId, 'MFA enrolls the requested account');
  assert.ok(enrolled.algorithm === 'SHA1', 'MFA uses SHA1');
  assert.ok(enrolled.digits === 6, 'MFA uses six digits');
  assert.ok(enrolled.period === 30, 'MFA uses a 30-second period');
  assert.ok(/^[A-Z2-7]+=*$/.test(enrolled.totpSecret), 'TOTP secret is base32');
  assert.ok(Array.isArray(enrolled.recoveryCodes) && enrolled.recoveryCodes.length > 0,
    'MFA returns recovery codes once');
  const enrollment = () => query(`SELECT confirmed,generation,
    (SELECT count(*)::int FROM account_recovery_codes WHERE account_id=$1) AS codes
    FROM account_mfa_totp WHERE account_id=$1`, [accountId]);
  const [confirmed] = await enrollment();
  assert.equal(confirmed.confirmed, true);
  assert.equal(confirmed.codes, enrolled.recoveryCodes.length);
  seedRefused('account-mfa', mfa, 2, 'conflict',
    'MFA is already enrolled; its secret is shown only at enrolment');
  assert.deepEqual(await enrollment(), [confirmed]);
});

test('seed world, channel, route and OAuth client retain their persisted identity on replay', async () => {
  const worldId = await id();
  const channelId = await id();
  const world = { worldId, name: 'CLI World', region: 'local' };
  const scope = { worldId, channelId };
  seedRefused('channel', scope, 2, 'bad_input', 'worldId is not a seeded world');
  seedRefused('world', { ...world, name: '' }, 2, 'bad_input', 'name must be 1..=96 characters, trimmed');
  const seededWorld = seed('world', world);
  assert.equal(seededWorld.worldId, worldId);
  assert.deepEqual(seed('world', world), seededWorld);
  assert.deepEqual(await query('SELECT name FROM world_status WHERE id=$1', [worldId]), [{ name: world.name }]);

  const channel = seed('channel', scope);
  assert.equal(channel.channelId, channelId);
  assert.equal(channel.routeRevision, null);
  assert.equal(channel.nativeLoginEnabled, false);
  assert.deepEqual(seed('channel', scope), channel);

  const args = { ...scope, host: '127.0.0.1', port: 7172, tlsServerName: 'game.example.invalid', version: 1 };
  seedRefused('route', { ...args, worldId: await id() }, 2, 'bad_input', 'worldId is not a seeded world');
  const route = seed('route', args);
  assert.equal(route.version, 1);
  assert.equal(route.nativeLoginEnabled, true);
  assert.match(route.routeRevision, /^rt\.1\.[a-f0-9]{32}$/);
  assert.deepEqual(seed('route', args), route);
  assert.deepEqual(await query('SELECT native_route_revision AS revision FROM game_channels WHERE world_id=$1 AND channel_id=$2',
    [worldId, channelId]), [{ revision: route.routeRevision }]);

  const clientId = await id();
  const client = seed('oauth-native-client', { clientId });
  assert.deepEqual(seed('oauth-native-client', { clientId }), client);
  assert.deepEqual(await query('SELECT enabled,first_party,public_client,native_ticket FROM oauth_native_clients WHERE client_id=$1',
    [clientId]), [{ enabled: true, first_party: true, public_client: true, native_ticket: true }]);
  seedRefused('oauth-native-client', { clientId: 'bad-id' }, 2, 'bad_input', 'clientId is not a UUID');
});

test('seed signing trust and encrypted custody publish stable keys and refuse invalid inputs', async () => {
  const directory = path.join(scratch, 'custody');
  const args = { directory, keyId: 'cli-custody' };
  seedRefused('signing-custody', args, 1, 'unavailable',
    'signing-custody runs only with OTERYN_ENV=testing or preprod', { env: { OTERYN_ENV: 'production' } });
  seedRefused('signing-custody', { ...args, directory: 'relative' }, 2, 'bad_input',
    'directory must be an absolute path');
  const custody = seed('signing-custody', args);
  assert.equal(custody.state, 'active');
  assert.equal(custody.keyId, args.keyId);
  assert.equal(custody.wrappingKeyFile, path.join(directory, 'admission-wrapping.key'));
  assert.equal(custody.custodyFile, path.join(directory, 'admission-custody.bin'));
  assert.equal(fs.statSync(custody.wrappingKeyFile).mode & 0o777, 0o600);
  assert.equal(fs.statSync(custody.wrappingKeyFile).size, 32);
  assert.equal(fs.statSync(custody.custodyFile).mode & 0o777, 0o600);
  assert.deepEqual(seed('signing-custody', args), custody);
  seedRefused('signing-custody', { ...args, keyId: 'different' }, 2, 'conflict',
    'custody already holds another key id');

  const key = generateKeyPairSync('ed25519').publicKey.export({ format: 'jwk' }).x;
  const trustArgs = { purpose: custody.purpose, keyId: 'cli-trust', publicKeyHex: Buffer.from(key, 'base64url').toString('hex') };
  seedRefused('signing-trust', { ...trustArgs, publicKeyHex: 'bad' }, 2, 'bad_input',
    'publicKeyHex must be 64 hex digits');
  const trust = seed('signing-trust', trustArgs);
  assert.equal(trust.publicKey, key);
  assert.equal(trust.keyId, trustArgs.keyId);
  const saved = () => query(`SELECT p.id AS profile_id,p.profile_version,p.issuer_revision,p.updated_at AS profile_updated_at,
    k.key_id,k.public_key,k.key_revision,k.trusted,k.trusted_at,k.revoked_at,k.updated_at
    FROM native_game_signing_trust_profiles p
    JOIN native_game_signing_trust_key_versions k ON k.profile_id=p.id
    WHERE p.issuer=$1 AND p.profile=$2 AND p.key_purpose=$3 AND k.key_id=$4
    ORDER BY p.profile_version,k.key_revision`, [trust.issuer, trust.profile, trustArgs.purpose, trustArgs.keyId]);
  const persisted = await saved();
  assert.equal(persisted.length, 1);
  assert.equal(persisted[0].public_key, key);
  assert.equal(persisted[0].trusted, true);
  assert.equal(persisted[0].revoked_at, null);
  assert.equal(persisted[0].issuer_revision, String(trust.issuerRevision));
  assert.deepEqual(seed('signing-trust', trustArgs), trust);
  assert.deepEqual(await saved(), persisted);
});

test('seed bootstrap intent is stable and has no operator audit row', async () => {
  const { accountId } = await account();
  const operationId = await id();
  const args = { accountId, operationId, targetWorldId: await id(), ...intentFields };
  seedRefused('bootstrap-intent', { ...args, accountId: await id() }, 2, 'bad_input',
    'identityId is not an active account');
  const issued = seed('bootstrap-intent', args);
  assert.equal(issued.operationId, operationId);
  assert.match(issued.intentSha256, /^[a-f0-9]{64}$/);
  const saved = () => query('SELECT * FROM character_bootstrap_intents WHERE operation_id=$1', [operationId]);
  const persisted = await saved();
  assert.equal(persisted.length, 1);
  const [record] = persisted;
  assert.deepEqual({
    accountId: record.account_id, operationId: record.operation_id, targetWorldId: record.target_world_id,
    requestedName: record.requested_name, requestedSex: record.requested_sex,
    requestedVocation: record.requested_vocation, profileRevision: record.profile_revision,
    rulesetRevision: record.ruleset_revision, contentRevision: record.content_revision,
    starterTemplateRevision: record.starter_template_revision,
  }, args);
  assert.equal(record.issuer_decision_id, issued.issuerDecisionId);
  assert.equal(record.source_revision, issued.sourceRevision);
  assert.equal(record.issued_at_source, issued.issuedAtSource);
  assert.equal(record.expires_at_source, issued.expiresAtSource);
  assert.equal(record.intent_sha256, issued.intentSha256);
  assert.equal(createHash('sha256').update(record.intent_json).digest('hex'), issued.intentSha256);
  assert.deepEqual(seed('bootstrap-intent', args), issued);
  assert.deepEqual(await saved(), persisted);
  assert.deepEqual(await audit(operationId), []);
});

test('bootstrap-intent issue binds the decision to Postgres and audits it once', async () => {
  const actor = (await account()).accountId;
  const identityId = (await account()).accountId;
  const operationId = await id();
  const unknown = await id();
  await grant(actor, ['game.player.manage']);
  const args = ['bootstrap-intent', 'issue', '--actor', actor];
  const request = { identityId, operationId, targetWorldId: await id(), ...intentFields };
  const input = JSON.stringify(request);
  const options = { env: bootstrapEnv, input };
  refused(args, 4, 'unavailable', 'bootstrap-intent issuance is disabled',
    { input, env: { ...bootstrapEnv, GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_ENABLED: 'false' } });
  refused(args, 4, 'unavailable', 'GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_TTL_SECONDS must be 1..=300',
    { input, env: { ...bootstrapEnv, GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_TTL_SECONDS: '301' } });
  refused(args.slice(0, 2), 2, 'bad_input', '--actor <uuidv7> is required', options);
  refused([...args, '--wrong', 'value'], 2, 'bad_input',
    'usage: oteryn-portal bootstrap-intent issue --actor <uuidv7> [--input <file>]', options);
  for (const denied of [identityId, unknown]) {
    refused([...args.slice(0, 3), denied], 2, 'bad_input',
      'actor is not an active staff account with game.player.manage', options);
  }
  refused(args, 2, 'bad_input', 'identityId is not an active account',
    { env: bootstrapEnv, input: JSON.stringify({ ...request, identityId: unknown }) });
  refused([...args, '--input', jsonFile('broken-intent.json', '{broken')], 2, 'bad_input',
    'input must be one flat JSON object of at most 16 KiB', { env: bootstrapEnv });
  const saved = () => query(`SELECT account_id,target_world_id,intent_json,intent_sha256,
    source_revision,issued_at_source,expires_at_source FROM character_bootstrap_intents WHERE operation_id=$1`,
  [operationId]);
  assert.deepEqual(await saved(), []);
  assert.deepEqual(await audit(operationId), []);

  await whileSuspended(actor, () => refused(args, 2, 'bad_input',
    'actor is not an active staff account with game.player.manage', options));
  assert.deepEqual(await saved(), []);
  assert.deepEqual(await audit(operationId), []);

  const file = jsonFile('valid-intent.json', input);
  const stdout = success([...args, '--input', file], { env: bootstrapEnv });
  const [row] = await saved();
  const intent = json(stdout);
  assert.equal(intent.contract_version, 4);
  assert.equal(intent.operation_id, operationId);
  assert.equal(intent.requested_name, request.requestedName);
  assert.equal(intent.requested_sex, request.requestedSex);
  assert.equal(intent.requested_vocation, request.requestedVocation);
  assert.equal(row.account_id, identityId);
  assert.equal(row.target_world_id, request.targetWorldId);
  assert.equal(Number(row.expires_at_source) - Number(row.issued_at_source), 300);
  assert.equal(stdout, `${row.intent_json}\n`);
  assert.equal(row.intent_sha256, createHash('sha256').update(row.intent_json).digest('hex'));
  assert.equal(success(args, options), stdout);
  refused(args, 3, 'conflict', 'operationId was issued with a different binding',
    { env: bootstrapEnv, input: JSON.stringify({ ...request, requestedName: 'Aldric Stone' }) });
  assert.deepEqual(await saved(), [row]);
  assert.deepEqual(await audit(operationId), [{
    actor, action: 'character.bootstrap_intent.issued', request_id: operationId, before: {},
    after: { identityId, operationId, sourceRevision: row.source_revision, ttlSeconds: 300, intentSha256: row.intent_sha256 },
  }]);
});

test('catalog import and activate require their own permissions and audit each transition once', async () => {
  const actor = (await account()).accountId;
  const outsider = (await account()).accountId;
  const unknown = await id();
  await grant(actor, ['store.catalog.import']);
  const fixture = path.join(repository, 'docs/contracts/fixtures/store-price-list-v1/price-list.valid.json');
  const importArgs = ['catalog', 'import', fixture];
  const activateArgs = revision => ['catalog', 'activate', revision];
  for (const args of [importArgs, activateArgs('a'.repeat(64))]) {
    refused(args, 2, 'Usage');
    refused([...args, '--wrong', actor], 2, 'Usage');
    for (const denied of [outsider, unknown]) refused([...args, '--actor', denied], 2, 'Denied');
  }
  refused(['catalog', 'import', jsonFile('broken-catalog.json', '{broken'), '--actor', actor], 2, 'InvalidDocument');
  refused(['catalog', 'import', path.join(scratch, 'missing.json'), '--actor', actor], 2, 'ReadFile');
  assert.deepEqual(await query('SELECT catalog_revision FROM store_price_list_imports'), []);
  await whileSuspended(actor, () => refused([...importArgs, '--actor', actor], 2, 'Denied'));
  assert.deepEqual(await query('SELECT catalog_revision FROM store_price_list_imports'), []);
  assert.deepEqual(await query('SELECT action FROM audit_log WHERE actor=$1', [actor]), []);
  const stdout = success([...importArgs, '--actor', actor]);
  assert.match(stdout, /^[a-f0-9]{64}\n$/);
  const revision = stdout.trim();
  assert.equal(success([...importArgs, '--actor', actor]), stdout);
  refused([...activateArgs(revision), '--actor', actor], 2, 'Denied');
  await grant(actor, ['store.catalog.activate']);
  refused([...activateArgs('a'.repeat(64)), '--actor', actor], 2, 'NotImported');
  refused([...activateArgs('broken'), '--actor', actor], 2, 'InvalidRevision');
  assert.deepEqual(await query('SELECT catalog_revision FROM store_active_catalog'), [{ catalog_revision: null }]);

  const activate = [...activateArgs(revision), '--actor', actor];
  const preActivationAudit = await audit(revision);
  await whileSuspended(actor, () => refused(activate, 2, 'Denied'));
  assert.deepEqual(await query('SELECT catalog_revision FROM store_active_catalog'), [{ catalog_revision: null }]);
  assert.deepEqual(await query('SELECT catalog_revision FROM store_catalog_activations'), []);
  assert.deepEqual(await audit(revision), preActivationAudit);
  assert.equal(success(activate), stdout);
  assert.equal(success(activate), stdout);
  assert.deepEqual(await query('SELECT catalog_revision,activated_by FROM store_active_catalog'),
    [{ catalog_revision: revision, activated_by: actor }]);
  assert.deepEqual(await query('SELECT catalog_revision,imported_by FROM store_price_list_imports'),
    [{ catalog_revision: revision, imported_by: actor }]);
  assert.deepEqual(await query('SELECT catalog_revision,actor_id FROM store_catalog_activations'),
    [{ catalog_revision: revision, actor_id: actor }]);
  const entries = await audit(revision);
  assert.deepEqual(entries.map(({ action, actor, after }) => ({ action, actor, after })), [
    { action: 'store.catalog.activated', actor, after: { catalog_revision: revision, previous_revision: null } },
    { action: 'store.catalog.imported', actor, after: { catalog_revision: revision, offers: 1 } },
  ]);
  assert.ok(entries.every(entry => entry.request_id));
});

test('game-route publish refuses unauthorized or conflicting writes and audits route rotation', async () => {
  const actor = (await account()).accountId;
  const outsider = (await account()).accountId;
  await grant(actor, ['game.world.admin']);
  const worldId = await id();
  const channelId = await id();
  const request = {
    worldId, channelId, name: 'Published CLI World', region: 'local', host: '127.0.0.1',
    port: 7172, tlsServerName: 'game.example.invalid', version: 1,
  };
  const prefix = ['game-route', 'publish'];
  const args = (operator, input) => [...prefix, '--actor', operator, input];
  const input = JSON.stringify(request);
  const refusal = (words, code, error, reason) => refused(words, code, error, reason, {}, 'stdout');
  const usage = "usage: oteryn-portal game-route publish --actor <uuid> '<json object>'";
  refusal(prefix, 2, 'bad_input', usage);
  refusal([...prefix, '--wrong', actor, input], 2, 'bad_input', usage);
  refusal(args(actor, '{broken'), 2, 'bad_input', 'arguments are not JSON');
  for (const denied of [outsider, await id()]) {
    refusal(args(denied, input), 4, 'denied', 'the actor is not an active account holding game.world.admin');
  }
  await whileSuspended(actor, () => refusal(args(actor, input), 4, 'denied',
    'the actor is not an active account holding game.world.admin'));
  assert.deepEqual(await query('SELECT id FROM world_status WHERE id=$1', [worldId]), []);
  assert.deepEqual(await query('SELECT channel_id FROM game_channels WHERE channel_id=$1', [channelId]), []);
  const target = `game_channels:${worldId}/${channelId}`;
  assert.deepEqual(await audit(target), []);

  const initial = success(args(actor, input));
  assert.equal(success(args(actor, input)), initial);
  const initialRoute = json(initial);
  assert.equal(initialRoute.worldId, worldId);
  assert.equal(initialRoute.channelId, channelId);
  assert.match(initialRoute.routeRevision, /^rt\.1\.[a-f0-9]{32}$/);
  const versionTwo = { ...request, port: 7173, version: 2 };
  refusal(args(actor, JSON.stringify({ ...versionTwo, version: 1 })), 2, 'conflict',
    'a route record with this or a higher version exists with other values');
  const rotated = json(success(args(actor, JSON.stringify(versionTwo))));
  assert.equal(rotated.port, 7173);
  assert.match(rotated.routeRevision, /^rt\.2\.[a-f0-9]{32}$/);
  assert.deepEqual(await query(`SELECT native_route_revision AS revision, native_route_port AS port
    FROM game_channels WHERE world_id=$1 AND channel_id=$2`, [worldId, channelId]),
  [{ revision: rotated.routeRevision, port: versionTwo.port }]);
  const entries = await audit(target);
  assert.equal(entries.length, 2);
  assert.ok(entries.every(entry => entry.action === 'game.route.published' && entry.actor === actor && entry.request_id));
  const rotation = entries.find(entry => entry.after.version === 2);
  assert.equal(rotation.before.routeRevision, initialRoute.routeRevision);
  assert.equal(rotation.after.routeRevision, rotated.routeRevision);
});

test('commerce-refunds-reconcile previews, revives and audits one dead outbox job', async () => {
  const actor = (await account()).accountId;
  const outsider = (await account()).accountId;
  await grant(actor, ['admin.access', 'payments.reconcile']);
  const operation = await id();
  const payment = await id();
  const job = await id();
  // Create persisted dead-delivery state. The harness worker leaves refund jobs
  // queued with the commerce/refunds switches off, and no provider is contacted.
  await query(`WITH intent AS (
    INSERT INTO payment_intents (id,account_id,provider,provider_reference,amount,currency,state)
    VALUES ($3,$4,'cli-fixture',$3::uuid::text,250,'PLN','succeeded') RETURNING id
  ), operation AS (
    INSERT INTO commerce_operations (id,actor_id,target_id,command,reason_sha256,requires_approval)
    VALUES ($1,$2,$4,'{"kind":"refund"}',repeat('0',64),false) RETURNING id
  ), refund AS (
    INSERT INTO commerce_refunds (operation_id,intent_id,account_id,amount,currency,provider)
    SELECT operation.id,intent.id,$4,250,'PLN','cli-fixture' FROM operation,intent
  )
  INSERT INTO outbox (id,kind,payload,idempotency_key,state,attempts)
  VALUES ($5,'commerce.refund',jsonb_build_object('operation',$1::uuid),$3::uuid::text,'dead',5)`,
  [operation, actor, payment, outsider, job]);
  const command = ['commerce-refunds-reconcile'];
  const usage = 'usage: oteryn-portal commerce-refunds-reconcile [--dry-run] --actor <uuid>';
  refused(command, 2, usage);
  refused([...command, '--wrong', actor], 2, usage);
  for (const denied of [outsider, await id(), 'malformed']) {
    refused([...command, '--actor', denied], 2, 'Denied');
  }
  const authorized = [...command, '--actor', actor];
  const expected = `operation=${operation} job=${job} state=pending attempt=- action=`;
  const lookup = () => query('SELECT state,attempts,payload,idempotency_key FROM outbox WHERE id=$1', [job]);
  const payload = { operation };
  const deadJob = [{ state: 'dead', attempts: 5, payload, idempotency_key: payment }];
  const refunds = () => query('SELECT * FROM commerce_refunds WHERE operation_id=$1', [operation]);
  const pendingRefund = await refunds();
  assert.equal(pendingRefund.length, 1);
  assert.equal(pendingRefund[0].state, 'pending');
  for (const permission of ['admin.access', 'payments.reconcile']) {
    const partial = (await account()).accountId;
    await grant(partial, [permission]);
    refused([...command, '--actor', partial], 2, 'Denied');
    assert.deepEqual(await lookup(), deadJob);
    assert.deepEqual(await refunds(), pendingRefund);
    assert.deepEqual(await audit(operation), []);
  }
  await whileSuspended(actor, () => refused(authorized, 2, 'Denied'));
  assert.deepEqual(await lookup(), deadJob);
  assert.deepEqual(await refunds(), pendingRefund);
  assert.deepEqual(await audit(operation), []);
  assert.equal(success([...authorized, '--dry-run']), `${expected}would_revive\n`);
  assert.deepEqual(await lookup(), [{ state: 'dead', attempts: 5, payload, idempotency_key: payment }]);
  assert.deepEqual(await audit(operation), []);
  assert.equal(success(authorized), `${expected}revived\n`);
  assert.equal(success(authorized), '');
  assert.deepEqual(await lookup(), [{ state: 'pending', attempts: 0, payload, idempotency_key: payment }]);
  assert.deepEqual(await audit(operation), [{
    actor, action: 'commerce.refund.reconciled', request_id: operation,
    before: { state: 'pending', attempt_id: null, job, job_state: 'dead' },
    after: { operation, job, job_state: 'pending' },
  }]);
});

test('downloads-publish records the publisher and rejects a replay of signed metadata', async () => {
  const actor = (await account()).accountId;
  const outsider = (await account()).accountId;
  await grant(actor, ['downloads.manage']);
  const bundle = path.join(portal, 'crates/downloads/tests/fixtures/origin-v1/stable');
  const env = {
    OTERYN_DOWNLOADS_ENABLED: 'true',
    OTERYN_DOWNLOADS_ROOT_STABLE: path.join(bundle, 'metadata/1.root.json'),
  };
  const command = ['downloads-publish', '--channel', 'stable'];
  const args = (operator, dir = bundle) => [...command, '--actor', operator, '--dir', dir];
  const refusal = (words, reason, config = env) => {
    const result = run(words, 1, { env: config });
    assert.equal(result.stdout, '');
    assert.equal(result.stderr, `Publication refused: ${reason}\n`);
  };
  refusal([...command, '--dir', bundle], 'invalid actor UUID');
  refusal([...args(actor), '--wrong', 'value'], 'invalid command arguments');
  refusal(args(actor), 'downloads are disabled', { ...env, OTERYN_DOWNLOADS_ENABLED: 'false' });
  for (const denied of [outsider, await id()]) refusal(args(denied), 'publication denied');

  const broken = path.join(scratch, 'broken-download');
  fs.cpSync(bundle, broken, { recursive: true });
  fs.writeFileSync(path.join(broken, 'policy-v1.json'), '{broken');
  refusal(args(actor, broken), 'invalid policy JSON');

  const releases = () => query(`SELECT r.id,r.published_by,r.previous_publication_id,c.active_publication_id
    FROM client_releases r JOIN client_channels c ON c.id=r.channel_id WHERE c.name='stable'`);
  const publicationAudit = await query('SELECT action FROM audit_log WHERE actor=$1', [actor]);
  await whileSuspended(actor, () => refusal(args(actor), 'publication denied'));
  assert.deepEqual(await releases(), []);
  assert.deepEqual(await query('SELECT action FROM audit_log WHERE actor=$1', [actor]), publicationAudit);
  const stdout = success(args(actor), { env });
  assert.match(stdout, /^Published [a-f0-9-]{36}\n$/);
  const release = stdout.trim().slice('Published '.length);
  refusal(args(actor), 'publication would replay or rewrite trusted history');
  assert.deepEqual(await releases(), [{
    id: release, published_by: actor, previous_publication_id: null, active_publication_id: release,
  }]);
});

test('check-image reports the binary version with packaged static files', () => {
  const env = { OTERYN_STATIC_DIR: path.join(portal, 'static') };
  const stdout = success(['check-image'], { env });
  assert.match(stdout, /^oteryn-portal \d+\.\d+\.\d+: serve worker migrate gateway\n$/);
  assert.equal(success(['check-image'], { env }), stdout);
  const missing = run(['check-image'], 1, { env: { OTERYN_STATIC_DIR: path.join(scratch, 'missing') } });
  assert.equal(missing.stdout, '');
  assert.equal(missing.stderr, '');
});
