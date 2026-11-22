import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { once } from 'node:events';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';
import { readStaffFixture, staffSeedEnvironment, startStaffConsoleHarness } from './admin-agent.mjs';
import { HarnessProcesses } from './harness-processes.mjs';
import { fixtureDatabasePool, localDatabaseURL } from './local-database.mjs';

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
