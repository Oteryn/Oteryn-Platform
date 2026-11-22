import assert from 'node:assert/strict';
import { execFile, spawn } from 'node:child_process';
import { once } from 'node:events';
import { access, constants, mkdtemp, readFile, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { promisify } from 'node:util';
import { expect } from '@playwright/test';
import pg from 'pg';
import setupGameData from './game-data.setup.mjs';

const execute = promisify(execFile);

test('shutdown during database creation waits and drops the new database', { timeout: 30_000 }, async () => {
  const adminUrl = new URL(process.env.PORTAL_E2E_ADMIN_DATABASE_URL);
  assert.ok(['localhost', '127.0.0.1', '[::1]'].includes(adminUrl.hostname));
  const scratch = await mkdtemp(path.join(os.tmpdir(), 'portal-e2e-interrupted-create-'));
  const blocker = new pg.Client({ connectionString: adminUrl.href });
  const observer = new pg.Client({ connectionString: adminUrl.href });
  adminUrl.searchParams.set('application_name', path.basename(scratch));
  await blocker.connect();
  await observer.connect();
  // Hold a real PostgreSQL catalog lock so SIGTERM arrives during CREATE DATABASE.
  await blocker.query('BEGIN; LOCK TABLE pg_database IN SHARE MODE');
  const child = spawn(process.execPath, ['harness.mjs', '--game-data'], {
    cwd: new URL('.', import.meta.url),
    env: {
      ...process.env,
      PORTAL_E2E_ADMIN_DATABASE_URL: adminUrl.href,
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

test('failed Game-data migration drops the disposable database and keeps logs', async () => {
  const adminUrl = process.env.PORTAL_E2E_ADMIN_DATABASE_URL;
  assert.ok(adminUrl, 'a disposable PostgreSQL administrator URL is required');
  assert.ok(['localhost', '127.0.0.1', '[::1]'].includes(new URL(adminUrl).hostname));
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

  const admin = new pg.Client({ connectionString: adminUrl });
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
  const admin = new pg.Pool({ connectionString: process.env.PORTAL_E2E_ADMIN_DATABASE_URL });
  async function subscription(statement) {
    const db = new pg.Client({ connectionString: databaseUrl });
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
