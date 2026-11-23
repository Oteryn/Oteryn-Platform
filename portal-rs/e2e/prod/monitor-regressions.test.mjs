import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const file = path => fileURLToPath(new URL(path, import.meta.url));
const source = path => readFileSync(file(path), 'utf8');

test('reconciliation continues past an empty cancelled rerun even if an earlier attempt had checks', async () => {
  const { latestProbe } = await import('./reconcile.mjs');
  const queried = [];
  const outcome = await latestProbe(async (run, attempt) => {
    queried.push([run, attempt]);
    if (attempt === 2) return { run: { id: run, run_attempt: attempt, status: 'completed', conclusion: 'cancelled',
      check_suite_id: 5, latest_check_runs_count: 17 }, jobs: [] };
    return { run: { id: run, run_attempt: attempt }, jobs: [
      { run_id: run, run_attempt: attempt, name: 'monitor', status: 'completed', conclusion: 'failure' },
    ] };
  }, [{ id: 123, run_attempt: 2 }]);
  assert.deepEqual(queried, [[123, 2], [123, 1]]);
  assert.deepEqual(outcome, { runId: 123, attempt: 1, conclusion: 'failure' });
  await assert.rejects(latestProbe(async () => ({
    run: { id: 456, run_attempt: 2, status: 'completed', conclusion: 'cancelled' }, jobs: [],
  }), [{ id: 123, run_attempt: 2 }]), /unexpected monitor attempt/);
});

test('the workflow selects credentialed production mode with named secrets and explicit missing status', () => {
  const workflow = source('../../../.github/workflows/prod-synthetic-monitor.yml');
  assert.match(workflow, /run: node prod\/run\.mjs --production-auth/);
  for (const name of ['OTERYN_MONITOR_EMAIL', 'OTERYN_MONITOR_PASSWORD', 'OTERYN_MONITOR_TOTP_SEED']) {
    assert.ok(workflow.includes(name + ': ${{ secrets.' + name + ' }}'));
  }
  const output = execFileSync(process.execPath, [file('./run.mjs'), '--production-auth'], {
    env: { PATH: process.env.PATH }, encoding: 'utf8',
  });
  assert.equal(output.trim(), 'skipped: monitor account not configured');
  const foreign = spawnSync(process.execPath, [file('./run.mjs'), '--production-auth'], {
    env: { PATH: process.env.PATH, MONITOR_BASE_URL: 'https://example.invalid', OTERYN_MONITOR_EMAIL: 'fixture@example.invalid',
      OTERYN_MONITOR_PASSWORD: 'synthetic-fixture', OTERYN_MONITOR_TOTP_SEED: 'SYNTHETICFIXTURE' },
    encoding: 'utf8',
  });
  assert.equal(foreign.status, 1);
  assert.match(foreign.stderr, /requires the canonical portal origin/);
});

test('Privacy rejects redirected documents and foreign form actions before submission', async () => {
  const { guardPrivacyOrigin, privacyFormAction } = await import('./privacy-origin.mjs');
  const sent = [];
  let handler;
  const page = { url: () => 'https://localhost:18443/pl/login', on: () => {} };
  const session = { send: async (command, args) => sent.push([command, args]), on: (_, callback) => { handler = callback; } };
  const verify = await guardPrivacyOrigin(page, session, 'https://localhost:18443');
  await handler({ requestId: 'first', request: { url: page.url() } });
  await handler({ requestId: 'redirect', resourceType: 'Document', request: { url: 'https://example.invalid/form' } });
  assert.deepEqual(sent.slice(1), [
    ['Fetch.continueRequest', { requestId: 'first' }],
    ['Fetch.failRequest', { requestId: 'redirect', errorReason: 'BlockedByClient' }],
  ]);
  assert.throws(verify, /foreign origin/);
  assert.throws(() => privacyFormAction(page.url(), '//example.invalid/form', 'https://localhost:18443'), /foreign origin/);
  assert.equal(privacyFormAction(page.url(), '/pl/account/privacy', 'https://localhost:18443'),
    'https://localhost:18443/pl/account/privacy');
});

test('TOTP submission polls fresh counters with a deadline and avoids near-expiry codes', async () => {
  const { freshTotpTimestamp } = await import('./totp-window.mjs');
  let now = 29_000;
  let polls = 0;
  const poll = async (ready, timeout) => {
    assert.equal(timeout, 35_000);
    assert.equal(ready(), false);
    polls++;
    now = 30_300;
    assert.equal(ready(), true);
  };
  assert.equal(await freshTotpTimestamp(poll, 0, () => now), 30_300);
  assert.equal(polls, 1);
  assert.equal(await freshTotpTimestamp(async ready => assert.equal(ready(), true), -1, () => 4_000), 4_000);
  assert.match(source('./monitor.spec.mjs'), /expect\.poll/);
  assert.doesNotMatch(source('./monitor.spec.mjs'), /node:timers\/promises|await delay\(/);
});
