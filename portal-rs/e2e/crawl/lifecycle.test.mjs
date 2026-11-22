import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createInterface } from 'node:readline';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import pg from 'pg';
import { localPostgresUrl } from '../postgres-url.mjs';

test('SIGTERM during the real cargo build removes the process, database, role and credentials', { timeout: 45_000 }, async t => {
  const adminURL = localPostgresUrl(process.env.PORTAL_E2E_ADMIN_DATABASE_URL);
  const admin = new pg.Client({ connectionString: adminURL.href });
  const target = fs.mkdtempSync(path.join(os.tmpdir(), 'portal-crawl-target-'));
  const e2e = fileURLToPath(new URL('..', import.meta.url));
  const env = { ...process.env, CARGO_TARGET_DIR: target, CARGO_BUILD_JOBS: '2' };
  delete env.PORTAL_E2E_BINARY;
  let launcher, closed, database, buildPid, output;
  try {
    await admin.connect();
    launcher = spawn(process.execPath, ['crawl/run.mjs'], { cwd: e2e, env, stdio: ['ignore', 'ignore', 'pipe'] });
    closed = once(launcher, 'close');
    const logs = createInterface({ input: launcher.stderr, signal: t.signal });
    for await (const line of logs) {
      database ??= line.match(/^\[harness\] database (portal_e2e_[a-f0-9]{12})$/)?.[1];
      const pid = line.match(/^\[harness\] cargo build started \(pid (\d+)\)$/)?.[1];
      if (pid && !buildPid) {
        buildPid = Number(pid);
        process.kill(buildPid, 'SIGSTOP');
        const { rows } = await admin.query(`
          SELECT r.rolname = d.datname AS owns_fixture,
            r.rolcanlogin AND NOT (r.rolsuper OR r.rolcreatedb OR r.rolcreaterole OR r.rolreplication OR r.rolbypassrls) AS limited
          FROM pg_database d JOIN pg_roles r ON r.oid = d.datdba WHERE d.datname=$1
        `, [database]);
        assert.deepEqual(rows, [{ owns_fixture: true, limited: true }]);
        assert.ok(launcher.kill('SIGTERM'));
      }
      output ??= line.match(/^Crawl diagnostics: (.+)$/)?.[1];
    }

    assert.ok(database && buildPid && output, 'The launcher cancelled a real build after creating its fixture');
    assert.deepEqual(await closed, [143, null]);
    assert.throws(() => process.kill(buildPid, 0), { code: 'ESRCH' });
    const { rows } = await admin.query(`
      SELECT datname AS name FROM pg_database WHERE datname=$1
      UNION ALL SELECT rolname FROM pg_roles WHERE rolname=$1
    `, [database]);
    assert.deepEqual(rows, []);
    for (const directory of ['certs', 'secrets']) {
      assert.equal(fs.existsSync(path.join(output, 'portal', directory)), false);
    }
  } finally {
    if (launcher && launcher.exitCode === null && launcher.signalCode === null) {
      launcher.kill('SIGTERM');
      try {
        await once(launcher, 'close', { signal: AbortSignal.timeout(12_000) });
      } catch {
        if (buildPid) {
          try { process.kill(-buildPid, 'SIGKILL'); } catch (error) {
            if (error.code !== 'ESRCH') throw error;
          }
        }
        launcher.kill('SIGKILL');
        await closed;
      }
    }
    await admin.end();
    fs.rmSync(target, { recursive: true, force: true });
    if (output) fs.rmSync(output, { recursive: true, force: true });
  }
});
