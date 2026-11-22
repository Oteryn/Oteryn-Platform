import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { once } from 'node:events';
import fs from 'node:fs';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { setImmediate as next } from 'node:timers/promises';
import { readStaffFixture, runStaffSeed, startStaffConsoleHarness } from '../admin-agent.mjs';
import { password } from '../helpers.mjs';
import { fixtureDatabasePool, localDatabaseURL } from '../local-database.mjs';

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
  admin.pathname = '/portal_e2e_staff_regression';
  const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'portal-staff-regression-'));
  const env = { TMPDIR: scratch, RUST_LOG: 'off', APP_ENV: 'testing', OTERYN_ENV: 'testing' };
  const previous = Object.fromEntries(Object.keys(env).map(key => [key, process.env[key]]));
  Object.assign(process.env, env);
  const starting = startStaffConsoleHarness(admin.href);
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
    return { out, address: settings.OTERYN_BIND };
  }, deadline);
  const port = Number(new URL('http://' + first.address).port);
  await assert.rejects(listen(port), { code: 'EADDRINUSE' });
  contender = await until(async () => {
    try {
      return await listen(port);
    } catch (error) {
      if (error.code !== 'EADDRINUSE') throw error;
    }
  }, deadline);

  const harness = await starting;
  assert.notEqual(harness.out, first.out);
  assert.equal(fs.statSync(path.join(first.out, 'logs', 'serve.log')).size, 0, 'the failed portal had no log output');
  assert.equal(fs.existsSync(path.join(first.out, 'secrets')), false);

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
