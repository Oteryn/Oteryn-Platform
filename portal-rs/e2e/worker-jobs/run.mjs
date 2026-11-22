// Start the documented harness once, preserving its database until Playwright
// and its owned worker subprocesses have exited.
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { closeSync, existsSync, mkdtempSync, openSync, readFileSync, rmSync, watch, writeFileSync } from 'node:fs';
import { createConnection } from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';

const [nodeMajor, nodeMinor] = process.versions.node.split('.').map(Number);
if (nodeMajor < 22 || (nodeMajor === 22 && nodeMinor < 15) || typeof process.execve !== 'function') {
  throw new Error('Worker jobs require native Node.js 22.15+ with process.execve support');
}

const here = path.dirname(fileURLToPath(import.meta.url));
const binary = process.env.PORTAL_E2E_BINARY;
if (!binary || !path.isAbsolute(binary)) {
  throw new Error('Set PORTAL_E2E_BINARY to the absolute path to the built oteryn-portal');
}

const scratch = mkdtempSync(path.join(os.tmpdir(), 'worker-jobs-'));
const output = path.join(scratch, 'harness');
const wrapper = path.join(scratch, 'portal');
const quote = value => "'" + value.replaceAll("'", "'\\''") + "'";
const invocation = [process.execPath, path.join(here, 'role.mjs'), binary].map(quote).join(' ');
writeFileSync(wrapper, `#!/bin/sh\nexec ${invocation} "$@"\n`, { mode: 0o700 });
const harness = spawn(process.execPath, [path.join(here, '../harness.mjs'), '--outbox'], {
  env: { ...process.env, PORTAL_E2E_BINARY: wrapper, PORTAL_E2E_OUT: output },
  stdio: ['ignore', 'pipe', 'inherit'],
});
const stopped = once(harness, 'close');
const lines = createInterface({ input: harness.stdout });
let tests;
let passed = false;

async function waitForWorkerControl(workerEnv) {
  if (existsSync(workerEnv)) return;
  const { promise, resolve, reject } = Promise.withResolvers();
  const watcher = watch(path.dirname(workerEnv), { signal: AbortSignal.timeout(30_000) }, check);
  function check() {
    if (existsSync(workerEnv)) resolve();
  }
  watcher.once('error', reject);
  watcher.once('close', () => reject(new Error('Worker control socket did not become ready')));
  try {
    check();
    await Promise.race([
      promise,
      stopped.then(([code]) => { throw new Error(`Harness exited before worker control readiness (${code})`); }),
    ]);
  } finally {
    watcher.close();
  }
}

for (const signal of ['SIGINT', 'SIGTERM']) {
  process.once(signal, () => {
    if (tests) tests.kill('SIGINT');
    else harness.kill('SIGTERM');
  });
}

try {
  const [line] = await Promise.race([
    once(lines, 'line'),
    stopped.then(([code]) => { throw new Error(`Harness exited before readiness (${code})`); }),
  ]);
  const report = JSON.parse(line);
  if (report.status !== 'ready') throw new Error('Harness readiness failed');

  // The supervisor publishes its environment after the control socket binds.
  // The harness report only establishes readiness of the HTTP listeners.
  const workerEnv = path.join(output, 'secrets', 'worker-env.json');
  await waitForWorkerControl(workerEnv);
  const socket = createConnection(path.join(scratch, 'worker.sock'));
  socket.setEncoding('utf8');
  let drained = '';
  for await (const chunk of socket) drained += chunk;
  if (drained !== '0') throw new Error('Initial harness worker failed to drain');

  const env = JSON.parse(readFileSync(workerEnv, 'utf8'));
  const logPath = path.join(scratch, 'playwright.log');
  const log = openSync(logPath, 'wx', 0o600);
  tests = spawn(process.execPath, [
    path.join(here, '../node_modules/@playwright/test/cli.js'),
    'test', '--config', path.join(here, 'playwright.config.mjs'), ...process.argv.slice(2),
  ], {
    cwd: here,
    env: {
      ...process.env,
      PORTAL_E2E_ORIGIN: env.OTERYN_PUBLIC_ORIGIN,
      PORTAL_E2E_DATABASE_URL: env.OTERYN_DATABASE_URL,
      PORTAL_WORKER_JOBS_ENV: workerEnv,
      PORTAL_WORKER_JOBS_ARTIFACTS: path.join(scratch, 'playwright-artifacts'),
      PORTAL_E2E_OUTBOX_BINARY: binary,
      PORTAL_E2E_OUTBOX_REPORT: path.join(output, 'report.json'),
      OTERYN_DATABASE_URL: env.OTERYN_DATABASE_URL,
      OTERYN_ENV: env.OTERYN_ENV,
      OTERYN_MFA_KEY: env.OTERYN_MFA_KEY,
    },
    stdio: ['ignore', log, log],
  });
  closeSync(log);
  process.stdout.write('Running worker-jobs Playwright tests\n');
  const [code] = await once(tests, 'close');
  process.exitCode = code ?? 1;
  passed = code === 0;
  for (const line of readFileSync(logPath, 'utf8').split('\n')) {
    const summary = line.trim();
    if (/^\d+ (?:passed|failed|did not run)(?: \([\d.msh ]+\))?$/.test(summary)) {
      process.stdout.write(`${summary}\n`);
    }
  }
} finally {
  lines.close();
  harness.kill('SIGTERM');
  const [code] = await stopped;
  if (code !== 0) process.exitCode = 1;
  if (passed && code === 0) rmSync(scratch, { recursive: true, force: true });
  else process.stderr.write(`Private harness diagnostics retained in ${scratch}\n`);
}
