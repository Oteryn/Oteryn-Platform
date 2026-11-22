import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import { once } from 'node:events';
import fs from 'node:fs';
import os from 'node:os';
import { createConnection, createServer } from 'node:net';
import path from 'node:path';
import { createInterface } from 'node:readline';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import pg from 'pg';
import { localPostgresUrl } from '../postgres-url.mjs';
import { spawnFixtureHarness, stopFixtureHarness } from '../support/fixture-harness.mjs';

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

async function withHarness(adminURL, check, extraEnv = {}) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'portal-cli-safety-'));
  const harness = fileURLToPath(new URL('../harness.mjs', import.meta.url));
  let child;
  let closed;
  try {
    child = spawn(process.execPath, [harness], {
      env: {
        ...process.env,
        PORTAL_E2E_ADMIN_DATABASE_URL: adminURL.href,
        PORTAL_E2E_OUT: path.join(directory, 'run'),
        PORTAL_E2E_KEEP_DATABASE: '0',
        ...extraEnv,
      },
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    closed = once(child, 'close');
    let stderr = '';
    let stdout = '';
    child.stderr.setEncoding('utf8').on('data', chunk => { stderr += chunk; });
    child.stdout.setEncoding('utf8').on('data', chunk => { stdout += chunk; });
    await check(child, () => ({ stderr, stdout }));
  } finally {
    if (child && child.exitCode === null && child.signalCode === null) {
      const cleanup = once(child, 'close', { signal: AbortSignal.timeout(20_000) });
      child.kill('SIGTERM');
      await cleanup.catch(async () => {
        child.kill('SIGKILL');
        await closed;
      });
    }
    fs.rmSync(directory, { recursive: true, force: true });
  }
}

// Forward to real PostgreSQL. The callback can withhold server bytes to model
// a lost response while letting PostgreSQL carry out the original operation.
async function withPostgresProxy(onClientData, withhold, check) {
  const adminURL = localPostgresUrl(process.env.PORTAL_E2E_ADMIN_DATABASE_URL);
  const sockets = new Set();
  let connections = 0;
  const server = createServer(client => {
    const index = ++connections;
    const backend = createConnection({ host: adminURL.hostname.replace(/^\[|\]$/g, ''), port: Number(adminURL.port) });
    for (const [socket, peer] of [[client, backend], [backend, client]]) {
      sockets.add(socket);
      socket.on('error', () => socket.destroy());
      socket.on('close', () => {
        sockets.delete(socket);
        peer.destroy();
      });
    }
    client.on('data', chunk => {
      onClientData(index, chunk);
      backend.write(chunk);
    });
    backend.on('data', chunk => {
      if (!withhold(index, chunk)) client.write(chunk);
    });
  });
  try {
    server.listen(0, '127.0.0.1');
    await once(server, 'listening');
    const endpoint = new URL(adminURL);
    endpoint.hostname = '127.0.0.1';
    endpoint.port = String(server.address().port);
    await check(endpoint, adminURL, () => connections);
  } finally {
    for (const socket of sockets) socket.destroy();
    await new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
  }
}

async function cancelAfter(child, milestone) {
  const earlyExit = once(child, 'close', { signal: AbortSignal.timeout(15_000) });
  await Promise.race([
    milestone,
    earlyExit.then(() => { throw new Error('Harness exited before the PostgreSQL milestone'); }),
  ]);
  assert.ok(child.kill('SIGTERM'));
  assert.deepEqual(await earlyExit, [0, null]);
}

test('SIGTERM finishes when PostgreSQL handshake bytes never reach the harness', async () => {
  const handshake = Promise.withResolvers();
  await withPostgresProxy(
    () => {},
    (index) => {
      assert.equal(index, 1);
      handshake.resolve();
      return true;
    },
    async (endpoint, _adminURL, connectionCount) => {
      await withHarness(endpoint, async (child, output) => {
        await cancelAfter(child, handshake.promise);
        assert.equal(connectionCount(), 1, 'No CREATE was issued, so no reconciliation connection is needed');
        assert.ok(output().stdout === '', 'Harness did not report readiness');
      }, { PGSSLMODE: 'disable' });
    },
  );
});

test('SIGTERM reconciles a committed CREATE whose response was lost', async () => {
  let database;
  let frontendTail = '';
  let backendTail = '';
  const commit = Promise.withResolvers();
  await withPostgresProxy(
    (index, chunk) => {
      if (index !== 1 || database) return;
      const data = frontendTail + chunk.toString('latin1');
      database = data.match(/CREATE DATABASE "(portal_e2e_[a-f0-9]{12})"/)?.[1];
      frontendTail = data.slice(-80);
    },
    (index, chunk) => {
      if (index !== 1 || !database) return false;
      const data = backendTail + chunk.toString('latin1');
      if (data.includes('CREATE DATABASE\0')) commit.resolve();
      backendTail = data.slice(-32);
      return true;
    },
    async (endpoint, adminURL, connectionCount) => {
      const admin = new pg.Client({ connectionString: adminURL.href });
      try {
        await admin.connect();
        await withHarness(endpoint, async (child, output) => {
          const databaseRows = async () => (await admin.query(
            'SELECT datname FROM pg_database WHERE datname=$1', [database]
          )).rows;
          await cancelAfter(child, commit.promise.then(async () => {
            assert.deepEqual(await databaseRows(), [{ datname: database }]);
          }));
          assert.equal(connectionCount(), 2, 'Reconciliation used a fresh connection');
          assert.deepEqual(await databaseRows(), []);
          assert.ok(output().stdout === '', 'Harness did not report readiness');
        }, { PGSSLMODE: 'disable' });
      } finally {
        await admin.end();
      }
    },
  );
});

test('cleanup retries a lost DROP reply and verifies the database is gone', { timeout: 900_000 }, async () => {
  let dropping = false;
  let firstDropConnection;
  let retryConnection;
  let dropCommitted = false;
  const frontendTails = new Map();
  let backendTail = '';
  await withPostgresProxy(
    (index, chunk) => {
      if (!dropping || index === firstDropConnection) return;
      const data = (frontendTails.get(index) ?? '') + chunk.toString('latin1');
      if (/DROP DATABASE IF EXISTS "portal_e2e_[a-f0-9]{12}"/.test(data)) {
        if (firstDropConnection === undefined) firstDropConnection = index;
        else retryConnection = index;
        frontendTails.delete(index);
      } else {
        frontendTails.set(index, data.slice(-80));
      }
    },
    (index, chunk) => {
      if (index !== firstDropConnection) return false;
      const data = backendTail + chunk.toString('latin1');
      dropCommitted ||= data.includes('DROP DATABASE\0');
      backendTail = data.slice(-32);
      return true;
    },
    async (endpoint, adminURL) => {
      const admin = new pg.Client({ connectionString: adminURL.href });
      try {
        await admin.connect();
        await withHarness(endpoint, async (child, output) => {
          const lines = createInterface({ input: child.stdout });
          const exited = once(child, 'close');
          let report;
          try {
            const [line] = await Promise.race([
              once(lines, 'line', { signal: AbortSignal.timeout(120_000) }),
              exited.then(() => { throw new Error('Harness exited before readiness'); }),
            ]);
            report = JSON.parse(line);
          } finally {
            lines.close();
          }
          assert.equal(report.status, 'ready');
          const { database } = report;
          assert.match(database, /^portal_e2e_[a-f0-9]{12}$/);
          dropping = true;
          assert.ok(child.kill('SIGTERM'));
          const shutdown = once(child, 'close', { signal: AbortSignal.timeout(30_000) });
          assert.deepEqual(await shutdown, [0, null]);
          assert.ok(dropCommitted, 'PostgreSQL completed the first drop');
          assert.ok(retryConnection > firstDropConnection, 'Cleanup retried on a fresh connection');
          assert.ok(output().stderr.includes(`retrying disposable database cleanup ${database}`));
          const { rows } = await admin.query('SELECT datname FROM pg_database WHERE datname=$1', [database]);
          assert.deepEqual(rows, []);
        }, { PGSSLMODE: 'disable' });
      } finally {
        await admin.end();
      }
    },
  );
});

test('fixture reaps services and drops the database when the harness is stopped', { timeout: 900_000 }, async () => {
  const adminURL = localPostgresUrl(process.env.PORTAL_E2E_ADMIN_DATABASE_URL);
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'portal-cli-fixture-'));
  const output = path.join(directory, 'run');
  const admin = new pg.Client({ connectionString: adminURL.href });
  const child = spawnFixtureHarness(output, adminURL.href, process.env);
  const exited = once(child, 'close');
  const lines = createInterface({ input: child.stdout });
  const listeners = [];
  child.stderr.resume();
  try {
    const [line] = await Promise.race([
      once(lines, 'line', { signal: AbortSignal.timeout(120_000) }),
      exited.then(() => { throw new Error('Harness exited before readiness'); }),
    ]);
    const report = JSON.parse(line);
    assert.equal(report.status, 'ready');
    assert.match(report.database, /^portal_e2e_[a-f0-9]{12}$/);
    await admin.connect();
    const databases = async () => (await admin.query('SELECT datname FROM pg_database WHERE datname=$1', [report.database])).rows;
    assert.deepEqual(await databases(), [{ datname: report.database }]);

    assert.ok(child.kill('SIGSTOP'));
    const stopDeadline = AbortSignal.timeout(3_000);
    for (;;) {
      stopDeadline.throwIfAborted();
      const status = await fs.promises.readFile(`/proc/${child.pid}/status`, 'utf8');
      if (/^State:\s+T\b/m.test(status)) break;
    }
    await assert.rejects(
      stopFixtureHarness(child, exited, output, adminURL.href, 100),
      /harness shutdown timed out/,
    );
    assert.deepEqual(await exited, [null, 'SIGKILL']);
    assert.deepEqual(await databases(), []);

    for (const address of Object.values(report.baseUrls)) {
      const listener = createServer();
      listeners.push(listener);
      listener.listen(Number(new URL(address).port), '127.0.0.1');
      await once(listener, 'listening', { signal: AbortSignal.timeout(2_000) });
    }
  } finally {
    lines.close();
    if (child.exitCode === null && child.signalCode === null) {
      await stopFixtureHarness(child, exited, output, adminURL.href, 100).catch(error => {
        if (error.message !== 'harness shutdown timed out') throw error;
      });
    }
    await Promise.all(listeners.map(listener => new Promise(resolve => listener.close(resolve))));
    await admin.end();
    fs.rmSync(directory, { recursive: true, force: true });
  }
});

test('a failing second world seed names the step and keeps input out of diagnostics', { timeout: 900_000 }, async () => {
  const adminURL = localPostgresUrl(process.env.PORTAL_E2E_ADMIN_DATABASE_URL);
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'portal-cli-seed-failure-'));
  const plan = path.join(directory, 'plan.json');
  const marker = 'fixture-input-that-must-stay-private';
  const worldId = '019a0000-0000-7000-8000-00000000ab01';
  try {
    for (const [args, diagnostic] of [
      [{ worldId, name: '' }, 'name must be 1..=96 characters, trimmed'],
      [{ worldId, [marker]: marker }, 'unknown argument (name omitted)'],
    ]) {
      fs.writeFileSync(plan, JSON.stringify({ steps: [
        { name: 'first-world', entity: 'world', args: { worldId, name: 'Fixture world' } },
        { name: 'second-world', entity: 'world', args },
      ] }), { mode: 0o600 });
      await withHarness(adminURL, async (child, output) => {
        assert.deepEqual(await once(child, 'close'), [1, null]);
        const { stdout, stderr } = output();
        assert.ok(stdout === '', 'Failed seed plan does not report readiness');
        assert.ok(stderr.includes('seed step second-world failed (2): ' + diagnostic),
          'Harness names the failing step and explains the validation failure');
        assert.ok(!stderr.includes(marker) && !stdout.includes(marker), 'Seed arguments stay private');
      }, {
        PORTAL_E2E_SEED_PLAN: plan,
        PORTAL_E2E_HTTP_PORT: '28180',
        PORTAL_E2E_INTERNAL_PORT: '28643',
        PORTAL_E2E_ADMISSION_PORT: '28191',
        PORTAL_E2E_GATEWAY_PORT: '28190',
      });
    }
  } finally {
    fs.rmSync(directory, { recursive: true, force: true });
  }
});
