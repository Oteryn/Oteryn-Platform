import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { once } from 'node:events';
import fs from 'node:fs';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { setImmediate as next } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';
import { readStaffFixture, runStaffSeed, startStaffConsoleHarness } from '../admin-agent.mjs';
import { password } from '../helpers.mjs';
import { fixtureDatabasePool, localDatabaseClient, localDatabaseURL } from '../local-database.mjs';

async function until(probe, signal) {
  for (;;) {
    signal.throwIfAborted();
    const result = await probe();
    if (result) return result;
    await next(undefined, { signal });
  }
}

async function listen(port) {
  const server = net.createServer(socket => socket.destroy()).listen(port, '127.0.0.1');
  await once(server, 'listening');
  return server;
}

test('managed staff retries a real bind conflict with logging off and bounds a blocked seed', { timeout: 900_000 }, async t => {
  const admin = localDatabaseURL(process.env.PORTAL_E2E_ADMIN_DATABASE_URL);
  const fixtureURL = new URL(admin);
  fixtureURL.pathname = '/portal_e2e_staff_regression';
  const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'portal-staff-regression-'));
  const env = { TMPDIR: scratch, RUST_LOG: 'off', APP_ENV: 'testing', OTERYN_ENV: 'testing' };
  const previous = Object.fromEntries(Object.keys(env).map(key => [key, process.env[key]]));
  Object.assign(process.env, env);
  const starting = startStaffConsoleHarness(fixtureURL.href);
  const startupFailed = new AbortController();
  starting.catch(error => startupFailed.abort(error));
  let contender;
  t.after(async () => {
    contender?.close();
    try {
      const harness = await starting.catch(() => undefined);
      await harness?.close();
    } finally {
      for (const [key, value] of Object.entries(previous)) {
        if (value === undefined) delete process.env[key];
        else process.env[key] = value;
      }
      fs.rmSync(scratch, { recursive: true, force: true });
    }
  });

  const deadline = AbortSignal.any([AbortSignal.timeout(600_000), startupFailed.signal, t.signal]);
  const first = await until(() => {
    const [directory] = fs.readdirSync(scratch);
    if (!directory) return;
    const out = path.join(scratch, directory, 'run');
    let settings;
    try {
      settings = JSON.parse(fs.readFileSync(path.join(out, 'secrets', 'env.json'), 'utf8'));
    } catch (error) {
      if (error.code === 'ENOENT' || error instanceof SyntaxError) return;
      throw error;
    }
    return { out, address: settings.OTERYN_BIND, database: new URL(settings.OTERYN_DATABASE_URL).pathname.slice(1) };
  }, deadline);
  const port = Number(new URL('http://' + first.address).port);
  await assert.rejects(listen(port), { code: 'EADDRINUSE' });
  admin.pathname = '/postgres';
  const { client: catalogLock } = localDatabaseClient(admin, 5_000);
  const { client: observer } = localDatabaseClient(admin, 5_000);
  try {
    await Promise.all([catalogLock.connect(), observer.connect()]);
    await catalogLock.query('BEGIN');
    const locked = await catalogLock.query('SELECT oid FROM pg_database WHERE datname = $1 FOR UPDATE', [first.database]);
    assert.equal(locked.rowCount, 1);
    contender = await until(async () => {
      try {
        return await listen(port);
      } catch (error) {
        if (error.code !== 'EADDRINUSE') throw error;
      }
    }, deadline);
    const blockedDrop = async () => (await observer.query(
      "SELECT pid, EXTRACT(epoch FROM clock_timestamp() - query_start)::float8 AS age FROM pg_stat_activity WHERE wait_event_type = 'Lock' AND query = $1",
      ['DROP DATABASE IF EXISTS "' + first.database + '" WITH (FORCE)'],
    )).rows[0];
    const dropDeadline = AbortSignal.any([deadline, AbortSignal.timeout(20_000)]);
    const firstDrop = await until(blockedDrop, dropDeadline);
    await until(async () => {
      const drop = await blockedDrop();
      if (!drop) return;
      assert.equal(drop.pid, firstDrop.pid, 'cleanup keeps the same DROP in flight through contention');
      return drop.age > 6;
    }, dropDeadline);
  } finally {
    await Promise.all([catalogLock.end(), observer.end()]);
  }

  const harness = await starting;
  assert.notEqual(harness.out, first.out);
  assert.equal(fs.statSync(path.join(first.out, 'logs', 'serve.log')).size, 0, 'the failed portal had no log output');
  assert.equal(fs.existsSync(path.join(first.out, 'secrets')), false);
  const { client: check } = localDatabaseClient(admin, 5_000);
  try {
    await check.connect();
    assert.equal((await check.query('SELECT 1 FROM pg_database WHERE datname = $1', [first.database])).rowCount, 0);
  } finally {
    await check.end();
  }

  const fixture = readStaffFixture(harness.out);
  const db = fixtureDatabasePool(fixture.env.OTERYN_DATABASE_URL);
  const locker = await db.connect();
  const input = { email: 'staff-seed-' + randomUUID() + '@example.invalid', password: password(), locale: 'en' };
  try {
    await locker.query('BEGIN');
    await locker.query('LOCK TABLE accounts IN ACCESS EXCLUSIVE MODE');
    const seed = runStaffSeed(fixture.seedBinary, fixture.env, 'account', input, 6_000);
    const timeout = assert.rejects(seed, { message: 'Portal seed account exceeded its 6000 ms deadline' });
    timeout.catch(() => {});
    await until(async () => {
      const { rows } = await db.query(
        "SELECT 1 FROM pg_stat_activity WHERE datname = current_database() AND wait_event_type = 'Lock' AND query LIKE '%accounts%' AND pid <> $1",
        [locker.processID],
      );
      return rows.length > 0;
    }, AbortSignal.timeout(5_000));
    await timeout;
  } finally {
    try {
      await locker.query('ROLLBACK');
    } finally {
      locker.release();
      await db.end();
    }
  }
  const { accountId } = await runStaffSeed(fixture.seedBinary, fixture.env, 'account', input);
  assert.match(accountId, /^[0-9a-f-]{36}$/);
});

function startSeedPlan(t, steps) {
  const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'portal-staff-seed-plan-'));
  const plan = path.join(scratch, 'plan.json');
  const out = path.join(scratch, 'run');
  fs.writeFileSync(plan, JSON.stringify({ steps }), { mode: 0o600 });
  const child = spawn(process.execPath, [
    fileURLToPath(new URL('../harness.mjs', import.meta.url)), '--staff-console', '--parent-lifetime-stdin', '--seed-plan', plan,
  ], {
    env: { ...process.env, OTERYN_ENV: 'testing', APP_ENV: 'testing', PORTAL_E2E_KEEP_DATABASE: '0', PORTAL_E2E_OUT: out, RUST_LOG: 'info' },
    stdio: ['pipe', 'pipe', 'pipe'],
  });
  child.stdin.on('error', () => {});
  const ended = once(child, 'close');
  ended.catch(() => {});
  const output = { stdout: '', stderr: '' };
  for (const key of Object.keys(output)) {
    child[key].setEncoding('utf8').on('data', chunk => { output[key] += chunk; });
  }
  t.after(async () => {
    child.stdin.end();
    try { await ended; } finally { fs.rmSync(scratch, { recursive: true, force: true }); }
  });
  return { out, output, child, ended };
}

function parseFixtureOutput(value, label) {
  try {
    return JSON.parse(value);
  } catch {
    throw new Error(`${label} must contain valid JSON`);
  }
}

test('staff seed plans keep MFA results and dependent results out of both ready reports', { timeout: 900_000 }, async t => {
  const player = { email: randomUUID() + '@example.invalid', password: password(), locale: 'en' };
  const { out, output, child, ended } = startSeedPlan(t, [
    { name: 'player', entity: 'account', args: player },
    { name: 'mfa', entity: 'account-mfa', args: { accountId: '${player.accountId}', password: player.password } },
    { name: 'world', entity: 'world', args: { worldId: '019b0760-0000-7000-8000-000000000003', name: '${mfa.accountId}' } },
  ]);
  const report = await until(() => {
    if (child.exitCode !== null) throw new Error('Staff harness failed before readiness');
    if (output.stdout.endsWith('\n')) return parseFixtureOutput(output.stdout, 'Staff harness stdout');
  }, t.signal);
  const file = path.join(out, 'secrets', 'seed-results.json');
  assert.equal(fs.statSync(file).mode & 0o777, 0o600);
  const privateResults = parseFixtureOutput(fs.readFileSync(file, 'utf8'), 'Private seed results');
  assert.equal(typeof privateResults.mfa.totpSecret, 'string');
  assert.equal(privateResults.mfa.recoveryCodes.length, 8);
  const reportFile = fs.readFileSync(path.join(out, 'report.json'), 'utf8');
  for (const secret of [privateResults.mfa.totpSecret, ...privateResults.mfa.recoveryCodes]) {
    assert.ok(!output.stdout.includes(secret) && !output.stderr.includes(secret) && !reportFile.includes(secret),
      'authentication material remains private');
  }
  assert.ok(privateResults.world.name === privateResults.mfa.accountId, 'private references still resolve');
  assert.ok(report.seed.player.accountId === privateResults.player.accountId, 'public account results are retained');
  assert.ok(JSON.stringify(report.seed.mfa) === '{"status":"private"}', 'MFA has only a private-result marker');
  assert.ok(JSON.stringify(report.seed.world) === '{"status":"private"}', 'dependent steps have only a private-result marker');
  child.stdin.end();
  assert.deepEqual(await ended, [0, null]);
});

test('a blocked staff seed-plan command reaches its deadline and tears down', { timeout: 900_000 }, async t => {
  const input = { email: randomUUID() + '@example.invalid', password: password(), locale: 'en' };
  const { out, output, child, ended } = startSeedPlan(t, [
    { name: 'blocked-account', entity: 'account', args: input },
  ]);
  const databaseURL = await until(() => {
    if (child.exitCode !== null) throw new Error('Staff harness failed before migration');
    try {
      return localDatabaseURL(JSON.parse(fs.readFileSync(path.join(out, 'secrets', 'env.json'), 'utf8')).OTERYN_DATABASE_URL);
    } catch (error) {
      if (error.code !== 'ENOENT' && !(error instanceof SyntaxError)) throw error;
    }
  }, t.signal);
  const database = databaseURL.pathname.slice(1);
  const admin = new URL(databaseURL);
  admin.pathname = '/postgres';
  const { client: locker } = localDatabaseClient(databaseURL, 5_000);
  const { client: observer } = localDatabaseClient(admin, 5_000);
  locker.on('error', () => {}); // DROP DATABASE WITH (FORCE) disconnects the locker.
  try {
    await Promise.all([locker.connect(), observer.connect()]);
    await until(() => {
      if (child.exitCode !== null) throw new Error('Staff harness failed during migration');
      try {
        return fs.readFileSync(path.join(out, 'logs', 'migrate.log'), 'utf8').includes('migrations applied');
      } catch (error) {
        if (error.code !== 'ENOENT') throw error;
      }
    }, t.signal);
    await locker.query('BEGIN');
    await locker.query('LOCK TABLE accounts IN ACCESS EXCLUSIVE MODE');
    await until(async () => {
      if (child.exitCode !== null) throw new Error('Staff harness exited before a seed was observed waiting');
      return (await observer.query(
        "SELECT 1 FROM pg_stat_activity WHERE datname = $1 AND wait_event_type = 'Lock' AND query LIKE 'SELECT id, password_hash FROM accounts%'",
        [database],
      )).rowCount;
    }, t.signal);
    assert.deepEqual(await ended, [1, null]);
    assert.ok(output.stdout === '', 'the failed seed plan prints no ready report');
    assert.ok(output.stderr.includes('seed step blocked-account exceeded its 30000 ms deadline'), 'the seed deadline is reported');
    assert.equal(fs.existsSync(path.join(out, 'secrets')), false);
    assert.equal((await observer.query('SELECT 1 FROM pg_database WHERE datname = $1', [database])).rowCount, 0);
  } finally {
    await Promise.all([locker.end(), observer.end()]);
  }
});
