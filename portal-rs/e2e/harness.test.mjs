import assert from 'node:assert/strict';
import { execFile, spawn, spawnSync } from 'node:child_process';
import { once } from 'node:events';
import fs from 'node:fs';
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

import { readStaffFixture, staffSeedEnvironment, startStaffConsoleHarness } from './admin-agent.mjs';
import { HarnessProcesses } from './harness-processes.mjs';
import { executeFixtureDdl } from './support/harness-database.mjs';
import { fixtureDatabasePool, localDatabaseURL } from './local-database.mjs';

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
    const createPattern = /^CREATE DATABASE (portal_e2e_[a-f0-9]{12})$/;
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

test('Playwright timing out during startup stops the harness and drops its database', { timeout: 45_000 }, async t => {
  const connectionString = adminUrl();
  assert.ok(process.env.PORTAL_E2E_BINARY, 'the built portal binary is required');
  const scratch = await mkdtemp(path.join(os.tmpdir(), 'portal-e2e-runner-timeout-'));
  const admin = new pg.Client({ connectionString });
  await admin.connect();
  const runner = spawn(process.execPath, [
    'node_modules/@playwright/test/cli.js', 'test', '--config', 'game-data.playwright.config.mjs',
    '--project', 'en-390', '--global-timeout', '15000', '--reporter', 'line',
    '--output', path.join(scratch, 'test-results'),
  ], {
    cwd: new URL('.', import.meta.url),
    env: {
      ...process.env, PORTAL_E2E_ADMIN_DATABASE_URL: connectionString,
      PORTAL_E2E_GAME_DATA_REPORT: '', TMPDIR: scratch,
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  // The harness inherits stderr, so a leaked harness could keep close pending.
  const runnerExited = once(runner, 'exit', { signal: t.signal });
  let result = '';
  runner.stdout.setEncoding('utf8').on('data', chunk => { result += chunk; });
  const lines = createInterface({ input: runner.stderr, signal: t.signal });
  let database, harnessPid, migrationPid;
  const processState = async pid => {
    try {
      return (await execute('ps', ['-o', 'stat=', '-p', String(pid)])).stdout.trim();
    } catch (error) {
      if (error.code !== 1) throw error;
      return '';
    }
  };
  try {
    for await (const line of lines) {
      database ??= line.match(/^\[harness\] database (portal_e2e_[a-f0-9]{12})$/)?.[1];
      const pid = line.match(/^\[harness\] .* migrate started \(pid (\d+)\)$/)?.[1];
      if (pid) {
        // Pause the real migration until Playwright's global deadline expires.
        process.kill(Number(pid), 'SIGSTOP');
        migrationPid = Number(pid);
        harnessPid = Number((await execute('ps', ['-o', 'ppid=', '-p', pid])).stdout.trim());
        break;
      }
    }
    assert.ok(database && migrationPid && harnessPid > 1, 'the harness is migrating the disposable database');
    const databaseCount = async () => (await admin.query('SELECT 1 FROM pg_database WHERE datname=$1', [database])).rowCount;
    assert.equal(await databaseCount(), 1);
    assert.deepEqual(await runnerExited, [1, null]);
    assert.match(result, /Timed out waiting 15s for the global setup to run/);
    await expect.poll(databaseCount, { timeout: 10_000 }).toBe(0);
    assert.throws(() => process.kill(migrationPid, 0), { code: 'ESRCH' });
    // The orphan may remain a zombie until the OS reaps it.
    await expect.poll(() => processState(harnessPid)).toMatch(/^(?:Z|$)/);
  } finally {
    try {
      if (runner.exitCode === null && runner.signalCode === null) runner.kill('SIGTERM');
      if (harnessPid > 1 && !/^(?:Z|$)/.test(await processState(harnessPid))) {
        try { process.kill(harnessPid, 'SIGTERM'); } catch (error) { if (error.code !== 'ESRCH') throw error; }
        await expect.poll(() => processState(harnessPid), { timeout: 15_000 }).toMatch(/^(?:Z|$)/);
      }
    } finally {
      lines.close();
      await admin.end();
    }
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
        await executeFixtureDdl(admin.query.bind(admin), 'dropDatabaseIfExists', report.database);
      } else {
        await teardown();
      }
    } finally {
      await admin.end();
    }
  }
  await rm(path.dirname(report.outputDirectory), { recursive: true, force: true });
});

function refusedBeforeProvisioning(t, databaseURL, overrides, message) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'portal-harness-guard-'));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  const out = path.join(directory, 'run');
  const run = spawnSync(process.execPath, [fileURLToPath(new URL('./harness.mjs', import.meta.url)), '--staff-console'], {
    env: {
      ...process.env, PORTAL_E2E_ADMIN_DATABASE_URL: databaseURL, PORTAL_E2E_OUT: out,
      OTERYN_ENV: 'testing', APP_ENV: 'testing', ...overrides,
    },
    encoding: 'utf8', timeout: 10_000,
  });
  assert.equal(run.status, 1);
  assert.match(run.stderr, message);
  assert.equal(run.stdout, '');
  assert.equal(fs.existsSync(out), false);
}

test('managed and direct launchers reject a remote PostgreSQL host override', async t => {
  const dsn = 'postgres://127.0.0.1/portal_e2e_guard?host=remote.invalid';
  await assert.rejects(startStaffConsoleHarness(dsn), /Local PostgreSQL only/);
  refusedBeforeProvisioning(t, dsn, {}, /Local PostgreSQL only/);
});

test('local PostgreSQL URL checks all host values and normalizes the loopback endpoint', () => {
  for (const input of ['localhost', '127.0.0.1', '[::1]']) {
    const url = localDatabaseURL(`postgres://${input}/postgres`);
    assert.equal(url.hostname, input);
    assert.equal(url.port, '5432');
  }
  assert.equal(localDatabaseURL('postgres://localhost/postgres?host=127.0.0.1&hostaddr=127.0.0.1&port=5433').href,
    'postgres://localhost:5433/postgres?hostaddr=127.0.0.1');
  for (const option of ['%68ost=remote.invalid', 'host=remote.invalid&host=localhost', 'hostaddr=192.0.2.1']) {
    assert.throws(() => localDatabaseURL(`postgres://localhost/postgres?${option}`), /Local PostgreSQL only/);
  }
  assert.throws(() => localDatabaseURL('postgres://localhost/portal_e2e_guard?dbname=other'),
    /database in the URL path/);
});

test('local PostgreSQL URL preserves its TLS name and pins the TCP address', () => {
  const url = localDatabaseURL('postgres://localhost/postgres?sslmode=verify-full');
  assert.equal(url.hostname, 'localhost');
  assert.equal(url.searchParams.get('hostaddr'), '127.0.0.1');
  assert.equal(url.searchParams.get('sslmode'), 'verify-full');
  assert.equal(localDatabaseURL(url.href).href, url.href);
});

test('fixture pool rejects PostgreSQL connection overrides before connecting', () => {
  assert.throws(() => fixtureDatabasePool('postgres://localhost/portal_e2e_guard?host=remote.invalid'), /Local PostgreSQL only/);
  assert.throws(() => fixtureDatabasePool('postgres://localhost/portal_e2e_guard?hostaddr=::1'), /Conflicting PostgreSQL host/);
  assert.throws(() => fixtureDatabasePool('postgres://localhost/postgres'), /require a local portal_e2e_ database/);
});

test('staff seeding, SQL and managed launch share the canonical database name check', async () => {
  for (const name of ['portal_e2e_encoded%3Fname', 'portal_e2e_%41', 'portal_e2e_%23', 'portal_e2e_', 'portal_e2e_dash-name']) {
    const dsn = 'postgres://127.0.0.1/' + name;
    const saved = { OTERYN_ENV: 'testing', APP_ENV: 'testing', OTERYN_DATABASE_URL: dsn };
    assert.throws(() => staffSeedEnvironment(saved), /local portal_e2e_ database/);
    assert.throws(() => fixtureDatabasePool(dsn), /local portal_e2e_ database/);
    await assert.rejects(startStaffConsoleHarness(dsn), /local portal_e2e_ database/);
  }
});

test('harness shutdown waits for a stalled descendant after its group leader exits', { timeout: 20_000 }, async t => {
  const processes = new HarnessProcesses();
  const descendant = `
    import net from 'node:net';
    process.on('SIGINT', () => {});
    net.createServer().listen(0, '127.0.0.1', () => console.log(process.pid));
  `;
  const leader = `
    import { spawn } from 'node:child_process';
    const child = spawn(process.execPath, ['--input-type=module', '-e', ${JSON.stringify(descendant)}],
      { stdio: ['ignore', 'pipe', 'inherit'] });
    child.stdout.once('data', data => process.stdout.write(data, () => process.exit(0)));
  `;
  const { child, done } = processes.launch(process.execPath, ['--input-type=module', '-e', leader],
    { stdio: ['ignore', 'pipe', 'inherit'] });
  const lines = createInterface({ input: child.stdout });
  t.after(async () => {
    lines.close();
    await processes.stop(() => {});
  });
  const [pid] = await once(lines, 'line', { signal: AbortSignal.timeout(5_000) });
  assert.match(pid, /^[1-9]\d*$/);
  assert.equal((await done).status, 0);
  const live = () => {
    try {
      const stat = fs.readFileSync(`/proc/${pid}/stat`, 'utf8');
      return !/^[ZX]/.test(stat.slice(stat.lastIndexOf(')') + 2));
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
      return false;
    }
  };
  assert.equal(live(), true);
  const messages = [];
  assert.equal(await processes.stop(message => messages.push(message)), true);
  assert.deepEqual(messages, ['forcing shutdown of stalled children']);
  assert.equal(live(), false);
});

test('local PostgreSQL URL rejects conflicting local endpoints and ignores ambient PGPORT', t => {
  const previous = process.env.PGPORT;
  t.after(() => {
    if (previous === undefined) delete process.env.PGPORT;
    else process.env.PGPORT = previous;
  });
  process.env.PGPORT = '5439';
  assert.equal(localDatabaseURL('postgres://localhost/postgres').href, 'postgres://localhost:5432/postgres?hostaddr=127.0.0.1');
  for (const dsn of [
    'postgres://127.0.0.1/postgres?hostaddr=%3A%3A1',
    'postgres://[::1]/postgres?host=localhost',
  ]) assert.throws(() => localDatabaseURL(dsn), /Conflicting PostgreSQL host/);
  for (const dsn of [
    'postgres://localhost:5432/postgres?port=5433',
    'postgres://localhost/postgres?port=5432&port=5433',
    'postgres://localhost/postgres?port=5432extra',
  ]) assert.throws(() => localDatabaseURL(dsn), /Invalid or conflicting PostgreSQL port/);
});

test('a contender leaves an exclusively locked harness directory untouched', t => {
  const out = fs.mkdtempSync(path.join(os.tmpdir(), 'portal-harness-locked-'));
  t.after(() => fs.rmSync(out, { recursive: true, force: true }));
  const marker = path.join(out, '.portal-e2e-harness');
  const fixture = path.join(out, 'secrets', 'fixture');
  const unrelated = path.join(out, 'unrelated');
  fs.mkdirSync(path.dirname(fixture));
  for (const file of [marker, fixture, unrelated]) fs.writeFileSync(file, 'untouched');
  const fd = fs.openSync(out, 'r');
  try {
    assert.equal(spawnSync('flock', ['-n', '3'], { stdio: ['ignore', 'ignore', 'ignore', fd] }).status, 0);
    for (const args of [[], ['--staff-console'], ['--outbox']]) {
      const run = spawnSync(process.execPath, [fileURLToPath(new URL('./harness.mjs', import.meta.url)), ...args], {
        env: {
          ...process.env, PORTAL_E2E_ADMIN_DATABASE_URL: 'postgres://localhost/postgres', PORTAL_E2E_OUT: out,
          OTERYN_ENV: 'testing', APP_ENV: 'testing',
        },
        encoding: 'utf8', timeout: 10_000,
      });
      assert.equal(run.status, 1);
      assert.match(run.stderr, /in use by another harness run/);
      assert.equal(run.stdout, '');
      for (const file of [marker, fixture, unrelated]) assert.equal(fs.readFileSync(file, 'utf8'), 'untouched');
    }
  } finally {
    fs.closeSync(fd);
  }
});

test('staff harness refuses either production environment flag before provisioning', t => {
  for (const flag of ['OTERYN_ENV', 'APP_ENV']) {
    refusedBeforeProvisioning(t, 'postgres://localhost/postgres', { [flag]: 'production' },
      /requires OTERYN_ENV=testing and APP_ENV=testing/);
  }
});

test('external staff seed fixture forwards only the validated URL, testing flags and MFA key', () => {
  const databaseURL = 'postgres://127.0.0.1/portal_e2e_guard';
  const env = staffSeedEnvironment({
    OTERYN_ENV: 'testing', APP_ENV: 'testing', OTERYN_DATABASE_URL: databaseURL, OTERYN_MFA_KEY: 'test-key',
    PGHOSTADDR: '192.0.2.1', PGHOST: 'remote.invalid', PGPORT: '1', PGDATABASE: 'other', PGOPTIONS: '-c statement_timeout=1',
  });
  assert.deepEqual(env, {
    OTERYN_ENV: 'testing', APP_ENV: 'testing', OTERYN_DATABASE_URL: 'postgres://127.0.0.1:5432/portal_e2e_guard', OTERYN_MFA_KEY: 'test-key',
  });
});

test('external staff seed fixture requires both testing flags and a disposable database', () => {
  const saved = {
    OTERYN_ENV: 'testing', APP_ENV: 'testing', OTERYN_DATABASE_URL: 'postgres://127.0.0.1/portal_e2e_guard',
  };
  for (const flag of ['OTERYN_ENV', 'APP_ENV']) {
    for (const value of [undefined, 'production']) {
      assert.throws(() => staffSeedEnvironment({ ...saved, [flag]: value }), /seeding requires .*testing/);
    }
  }
  assert.throws(() => staffSeedEnvironment({ ...saved, OTERYN_DATABASE_URL: 'postgres://127.0.0.1/postgres' }),
    /require a local portal_e2e_ database/);
  assert.throws(() => staffSeedEnvironment({ ...saved, OTERYN_DATABASE_URL: saved.OTERYN_DATABASE_URL + '?host=remote.invalid' }),
    /Local PostgreSQL only/);
});

test('malformed private fixture JSON reports only the filename', t => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'staff-fixture-invalid-json-'));
  t.after(() => fs.rmSync(directory, { recursive: true, force: true }));
  const secrets = path.join(directory, 'secrets');
  fs.mkdirSync(secrets);
  fs.writeFileSync(path.join(secrets, 'admin-agent.json'), '{}');
  for (const file of ['env.json', 'admin-agent.json']) {
    fs.writeFileSync(path.join(secrets, file), '{"key": invalid_fixture_value}');
    assert.throws(() => readStaffFixture(directory), { message: `Invalid staff fixture JSON in ${file}` });
  }
});
