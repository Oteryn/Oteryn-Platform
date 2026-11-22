import assert from 'node:assert/strict';
import { execFile, spawn } from 'node:child_process';
import { once } from 'node:events';
import { access, constants, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { promisify } from 'node:util';
import { expect } from '@playwright/test';
import pg from 'pg';
import setupGameData from './game-data.setup.mjs';
import { GamePublishers } from './game-publishers.mjs';
import { localPostgresUrl } from './postgres-url.mjs';

const execute = promisify(execFile);
const adminUrl = () => localPostgresUrl(process.env.PORTAL_E2E_ADMIN_DATABASE_URL).href;

test('the shared PostgreSQL validator rejects destination overrides', () => {
  const base = 'postgresql://fixture@127.0.0.1/portal_e2e_probe';
  assert.equal(localPostgresUrl(base).href, 'postgresql://fixture@127.0.0.1:5432/portal_e2e_probe');
  assert.equal(localPostgresUrl('postgres://fixture@[::1]:5543/postgres').port, '5543');
  for (const suffix of [
    '?host=remote.example.invalid', '?%68ost=remote.example.invalid', '?hostaddr=192.0.2.1',
    '?port=5543', '?dbname=existing', '?database=existing', '?service=alternate',
    '?options=alternate', '#fragment',
  ]) {
    assert.throws(() => localPostgresUrl(base + suffix), /Expected a local PostgreSQL URL/);
  }
});

test('the harness and Game publisher reject an overridden destination before use', async () => {
  const scratch = await mkdtemp(path.join(os.tmpdir(), 'portal-e2e-invalid-destination-'));
  const output = path.join(scratch, 'harness');
  const unsafeDatabase = 'postgres://fixture@127.0.0.1/portal_e2e_123abc?host=remote.example.invalid';
  try {
    await assert.rejects(execute(process.execPath, ['harness.mjs', '--game-data'], {
      cwd: new URL('.', import.meta.url),
      env: { ...process.env, PORTAL_E2E_ADMIN_DATABASE_URL: unsafeDatabase, PORTAL_E2E_OUT: output },
      timeout: 10_000,
    }), error => {
      assert.equal(error.code, 1);
      assert.match(error.stderr, /Expected a local PostgreSQL URL/);
      assert.equal(error.stderr.includes(unsafeDatabase), false);
      return true;
    });
    await assert.rejects(access(output), { code: 'ENOENT' });

    const report = path.join(scratch, 'report.json');
    await mkdir(path.join(scratch, 'secrets'));
    await writeFile(report, JSON.stringify({
      outputDirectory: scratch, baseUrls: { internal: 'https://127.0.0.1:18543' },
    }), { mode: 0o600 });
    await writeFile(path.join(scratch, 'secrets', 'env.json'), JSON.stringify({
      OTERYN_ENV: 'testing', OTERYN_PUBLIC_ORIGIN: 'https://127.0.0.1:18080',
      OTERYN_DATABASE_URL: unsafeDatabase,
    }), { mode: 0o600 });
    assert.throws(() => new GamePublishers(report), /Expected a local PostgreSQL URL/);
  } finally {
    await rm(scratch, { recursive: true, force: true });
  }
});

test('shutdown during database creation waits and drops the new database', { timeout: 30_000 }, async () => {
  const scratch = await mkdtemp(path.join(os.tmpdir(), 'portal-e2e-interrupted-create-'));
  const connectionString = adminUrl();
  const blocker = new pg.Client({ connectionString });
  const observer = new pg.Client({ connectionString });
  await blocker.connect();
  await observer.connect();
  // Hold a real PostgreSQL catalog lock so SIGTERM arrives during CREATE DATABASE.
  await blocker.query('BEGIN; LOCK TABLE pg_database IN SHARE MODE');
  const child = spawn(process.execPath, ['harness.mjs', '--game-data'], {
    cwd: new URL('.', import.meta.url),
    env: {
      ...process.env,
      PORTAL_E2E_ADMIN_DATABASE_URL: connectionString,
      PGAPPNAME: path.basename(scratch),
      PORTAL_E2E_OUT: path.join(scratch, 'harness'),
      PORTAL_E2E_KEEP_DATABASE: '0',
    },
    stdio: ['ignore', 'ignore', 'pipe'],
  });
  const stopped = once(child, 'close');
  const lines = createInterface({ input: child.stderr });
  try {
    const activeCreation = async () => (await observer.query(
      `SELECT query FROM pg_stat_activity
       WHERE application_name=$1 AND wait_event_type='Lock' AND query LIKE 'CREATE DATABASE %'`,
      [path.basename(scratch)],
    )).rows[0]?.query;
    const createPattern = /^CREATE DATABASE "(portal_e2e_[a-f0-9]{12})"$/;
    await expect.poll(activeCreation, { timeout: 15_000 }).toMatch(createPattern);
    const [, database] = createPattern.exec(await activeCreation());
    const shutdown = once(lines, 'line');
    assert.equal(child.kill('SIGTERM'), true);
    assert.deepEqual(await Promise.race([shutdown, stopped]), ['[harness] shutdown requested']);
    await blocker.query('COMMIT');
    assert.deepEqual(await stopped, [0, null]);
    assert.equal((await observer.query('SELECT 1 FROM pg_database WHERE datname=$1', [database])).rowCount, 0);
  } finally {
    await blocker.end(); // Also releases the lock if the test fails before COMMIT.
    if (child.exitCode === null && child.signalCode === null) child.kill('SIGTERM');
    await stopped;
    lines.close();
    await observer.end();
  }
  await rm(scratch, { recursive: true, force: true });
});

test('SIGTERM stops a stalled Game-data migration before dropping its database', { timeout: 30_000 }, async t => {
  const connectionString = adminUrl();
  assert.ok(process.env.PORTAL_E2E_BINARY, 'the built portal binary is required');
  const scratch = await mkdtemp(path.join(os.tmpdir(), 'portal-e2e-stalled-migration-'));
  const output = path.join(scratch, 'harness');
  const admin = new pg.Client({ connectionString });
  await admin.connect();
  const child = spawn(process.execPath, ['harness.mjs', '--game-data'], {
    cwd: new URL('.', import.meta.url),
    env: {
      ...process.env, PORTAL_E2E_ADMIN_DATABASE_URL: connectionString,
      PORTAL_E2E_OUT: output, PORTAL_E2E_KEEP_DATABASE: '0',
    },
    stdio: ['ignore', 'ignore', 'pipe'],
  });
  const stopped = once(child, 'close');
  const lines = createInterface({ input: child.stderr, signal: t.signal });
  let database, migrationPid;
  try {
    for await (const line of lines) {
      database ??= line.match(/^\[harness\] database (portal_e2e_[a-f0-9]{12})$/)?.[1];
      if (line === '[harness] services ready') break;
      const pid = line.match(/^\[harness\] .* migrate started \(pid (\d+)\)$/)?.[1];
      if (pid) {
        // Suspend the real binary so a graceful signal alone cannot stop it.
        process.kill(Number(pid), 'SIGSTOP');
        migrationPid = Number(pid);
        break;
      }
    }
    assert.ok(database && migrationPid, 'the real migration started');
    const shutdown = once(child, 'close', { signal: AbortSignal.timeout(10_000) });
    assert.equal(child.kill('SIGTERM'), true);
    assert.deepEqual(await shutdown, [0, null]);
    assert.throws(() => process.kill(migrationPid, 0), { code: 'ESRCH' });
    assert.equal((await admin.query('SELECT 1 FROM pg_database WHERE datname=$1', [database])).rowCount, 0);
    await access(path.join(output, 'logs', 'migrate.log'));
    await assert.rejects(access(path.join(output, 'report.json')), { code: 'ENOENT' });
  } finally {
    if (child.exitCode === null && child.signalCode === null) {
      if (migrationPid) {
        try { process.kill(migrationPid, 'SIGKILL'); } catch (error) { if (error.code !== 'ESRCH') throw error; }
      }
      child.kill('SIGTERM');
      await stopped;
    }
    await admin.end();
  }
  await rm(scratch, { recursive: true, force: true });
});

test('failed Game-data migration drops the disposable database and keeps logs', async () => {
  const connectionString = adminUrl();
  assert.ok(process.env.PORTAL_E2E_BINARY, 'the built portal binary is required');
  const scratch = await mkdtemp(path.join(os.tmpdir(), 'portal-e2e-migrate-failure-'));
  const output = path.join(scratch, 'harness');

  await assert.rejects(execute(process.execPath, ['harness.mjs', '--game-data'], {
    cwd: new URL('.', import.meta.url),
    env: {
      ...process.env,
      OTERYN_ENV: 'invalid-migration-environment',
      PORTAL_E2E_OUT: output,
      PORTAL_E2E_KEEP_DATABASE: '0',
    },
    timeout: 30_000,
  }), error => {
    assert.equal(error.code, 1);
    assert.match(error.stderr, /startup failed; diagnostics are in/);
    return true;
  });

  const env = JSON.parse(await readFile(path.join(output, 'secrets', 'env.json'), 'utf8'));
  const database = new URL(env.OTERYN_DATABASE_URL).pathname.slice(1);
  assert.match(database, /^portal_e2e_[a-f0-9]+$/);
  const log = await readFile(path.join(output, 'logs', 'migrate.log'), 'utf8');
  assert.ok(log.length > 0, 'migration diagnostics remain on disk');
  await assert.rejects(readFile(path.join(output, 'report.json')), { code: 'ENOENT' });

  const admin = new pg.Client({ connectionString });
  await admin.connect();
  try {
    const { rowCount } = await admin.query('SELECT 1 FROM pg_database WHERE datname=$1', [database]);
    assert.equal(rowCount, 0, 'the failed startup database is gone');
  } finally {
    await admin.end();
  }
  await rm(scratch, { recursive: true, force: true });
});


test('Game-data teardown rejects a failed database drop and retains diagnostics', async () => {
  assert.equal(process.env.PORTAL_E2E_GAME_DATA_REPORT, undefined, 'the test needs its own Game-data harness');
  const teardown = await setupGameData();
  const reportPath = process.env.PORTAL_E2E_GAME_DATA_REPORT;
  const report = JSON.parse(await readFile(reportPath, 'utf8'));
  assert.match(report.database, /^portal_e2e_[a-f0-9]{12}$/);
  const { OTERYN_DATABASE_URL: databaseUrl } = JSON.parse(
    await readFile(path.join(report.outputDirectory, 'secrets', 'env.json'), 'utf8'),
  );
  const admin = new pg.Pool({ connectionString: adminUrl() });
  async function subscription(statement) {
    const db = new pg.Client({ connectionString: localPostgresUrl(databaseUrl).href });
    await db.connect();
    try {
      await db.query(statement);
    } finally {
      await db.end();
    }
  }
  let subscribed = false;
  try {
    const portalRoot = fileURLToPath(new URL('../', import.meta.url));
    assert.equal(report.binary, path.resolve(portalRoot, process.env.PORTAL_E2E_BINARY));
    await access(report.binary, constants.X_OK);

    // PostgreSQL refuses DROP DATABASE WITH (FORCE) while any subscription
    // exists. This disconnected subscription has no publisher or remote slot.
    await subscription(`CREATE SUBSCRIPTION harness_shutdown_probe
      CONNECTION 'host=127.0.0.1 port=1 dbname=postgres' PUBLICATION shutdown_probe
      WITH (connect = false, slot_name = NONE)`);

    subscribed = true;
    const migrationLog = await readFile(report.logs.migrate);
    await assert.rejects(teardown(), {
      message: `Game-data harness shutdown failed (exit code 1). Harness logs are in ${path.join(report.outputDirectory, 'logs')}`,
    });
    assert.deepEqual(await readFile(report.logs.migrate), migrationLog);
    assert.equal((await admin.query('SELECT 1 FROM pg_database WHERE datname=$1', [report.database])).rowCount, 1);
  } finally {
    try {
      if (subscribed) {
        await subscription('DROP SUBSCRIPTION harness_shutdown_probe');
        await admin.query(`DROP DATABASE IF EXISTS "${report.database}" WITH (FORCE)`);
      } else {
        await teardown();
      }
    } finally {
      await admin.end();
    }
  }
  await rm(path.dirname(report.outputDirectory), { recursive: true, force: true });
});
