import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { staffSeedEnvironment, startStaffConsoleHarness } from './admin-agent.mjs';
import { localDatabaseURL } from './local-database.mjs';

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

test('local PostgreSQL URL checks all host values and accepts loopback', () => {
  for (const host of ['localhost', '127.0.0.1', '[::1]']) {
    assert.equal(localDatabaseURL(`postgres://${host}/postgres`).hostname, host);
  }
  assert.equal(localDatabaseURL('postgres://localhost/postgres?host=127.0.0.1').hostname, 'localhost');
  for (const option of ['%68ost=remote.invalid', 'host=remote.invalid&host=localhost', 'hostaddr=192.0.2.1']) {
    assert.throws(() => localDatabaseURL(`postgres://localhost/postgres?${option}`), /Local PostgreSQL only/);
  }
  assert.throws(() => localDatabaseURL('postgres://localhost/portal_e2e_guard?dbname=other'),
    /database in the URL path/);
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
    OTERYN_ENV: 'testing', APP_ENV: 'testing', OTERYN_DATABASE_URL: databaseURL, OTERYN_MFA_KEY: 'test-key',
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
