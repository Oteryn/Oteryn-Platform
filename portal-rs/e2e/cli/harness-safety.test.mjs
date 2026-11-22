import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { once } from 'node:events';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createInterface } from 'node:readline';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import pg from 'pg';
import { localPostgresUrl } from '../postgres-url.mjs';

test('canonical PostgreSQL URLs fence the disposable database and endpoint', () => {
  const base = 'postgresql://fixture@127.0.0.1/oteryn_test';
  assert.equal(localPostgresUrl(base).href, 'postgresql://fixture@127.0.0.1:5432/oteryn_test');
  assert.equal(localPostgresUrl('postgres://fixture@[::1]:5543/postgres').port, '5543');
  for (const suffix of [
    '?dbname=existing', '?%64bname=existing', '?database=existing',
    '?host=remote.example.invalid', '?hostaddr=192.0.2.1', '?port=5543',
    '?service=alternate', '?options=alternate', '#fragment',
  ]) {
    assert.throws(() => localPostgresUrl(base + suffix), /Expected a local PostgreSQL URL/);
  }
  for (const invalid of ['http://127.0.0.1/postgres', 'postgres://remote.example.invalid/postgres',
    'postgres://127.0.0.1/', 'postgres://127.0.0.1/%2Fpostgres']) {
    assert.throws(() => localPostgresUrl(invalid), /Expected a local PostgreSQL URL/);
  }
});

test('the harness drops its database when the migration executable cannot start', async () => {
  const adminURL = localPostgresUrl(process.env.PORTAL_E2E_ADMIN_DATABASE_URL);
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'portal-cli-early-failure-'));
  const admin = new pg.Client({ connectionString: adminURL.href });
  try {
    await admin.connect();
    const harness = fileURLToPath(new URL('../harness.mjs', import.meta.url));
    const result = spawnSync(process.execPath, [harness], {
      env: {
        ...process.env,
        PORTAL_E2E_ADMIN_DATABASE_URL: adminURL.href,
        PORTAL_E2E_BINARY: path.join(directory, 'missing-binary'),
        PORTAL_E2E_OUT: path.join(directory, 'run'),
        PORTAL_E2E_KEEP_DATABASE: '0',
      },
      encoding: 'utf8',
      timeout: 60_000,
    });
    assert.ok(!result.error, 'Harness terminates within the process deadline');
    assert.equal(result.status, 1);
    assert.ok(result.stdout === '', 'Failed migration does not report readiness');
    assert.ok(result.stderr.includes('missing-binary migrate failed'), 'Harness reports the migration failure');
    const database = result.stderr.match(/^\[harness\] database (portal_e2e_[a-f0-9]{12})$/m)?.[1];
    assert.ok(database, 'Harness created a disposable database before migration');
    const { rows } = await admin.query('SELECT datname FROM pg_database WHERE datname=$1', [database]);
    assert.deepEqual(rows, []);
  } finally {
    await admin.end();
    fs.rmSync(directory, { recursive: true, force: true });
  }
});

test('SIGTERM cancels a stopped migration and drops the disposable database', { timeout: 900_000 }, async t => {
  const adminURL = localPostgresUrl(process.env.PORTAL_E2E_ADMIN_DATABASE_URL);
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'portal-cli-cancel-'));
  const admin = new pg.Client({ connectionString: adminURL.href });
  const harness = fileURLToPath(new URL('../harness.mjs', import.meta.url));
  let child, closed, database, migrationPid;
  try {
    await admin.connect();
    child = spawn(process.execPath, [harness], {
      env: {
        ...process.env,
        PORTAL_E2E_ADMIN_DATABASE_URL: adminURL.href,
        PORTAL_E2E_OUT: path.join(directory, 'run'),
        PORTAL_E2E_KEEP_DATABASE: '0',
      },
      stdio: ['ignore', 'ignore', 'pipe'],
    });
    closed = once(child, 'close');
    const lines = createInterface({ input: child.stderr, signal: t.signal });
    for await (const line of lines) {
      database ??= line.match(/^\[harness\] database (portal_e2e_[a-f0-9]{12})$/)?.[1];
      if (line === '[harness] services ready') break;
      const pid = line.match(/^\[harness\] .* migrate started \(pid (\d+)\)$/)?.[1];
      if (pid) {
        // Stop the real migration process so graceful cancellation cannot
        // complete before the harness exercises its forced shutdown path.
        process.kill(Number(pid), 'SIGSTOP');
        migrationPid = Number(pid);
        break;
      }
    }
    assert.ok(database && migrationPid, 'Harness created a database and started a migration');
    const shutdown = once(child, 'close', { signal: AbortSignal.timeout(10_000) });
    assert.ok(child.kill('SIGTERM'), 'Harness received SIGTERM during migration');
    assert.deepEqual(await shutdown, [0, null]);
    assert.throws(() => process.kill(migrationPid, 0), { code: 'ESRCH' });
    const { rows } = await admin.query('SELECT datname FROM pg_database WHERE datname=$1', [database]);
    assert.deepEqual(rows, []);
  } finally {
    if (child && child.exitCode === null && child.signalCode === null) {
      if (migrationPid) {
        try { process.kill(migrationPid, 'SIGKILL'); } catch (error) { if (error.code !== 'ESRCH') throw error; }
      }
      child.kill('SIGTERM');
      await closed;
    }
    await admin.end();
    fs.rmSync(directory, { recursive: true, force: true });
  }
});
