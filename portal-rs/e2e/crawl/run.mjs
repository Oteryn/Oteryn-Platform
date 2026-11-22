import fs from 'node:fs';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';
import { parseEnv } from 'node:util';

function validateAdminUrl(value) {
  let url;
  try {
    url = new URL(value);
  } catch {
    throw new Error('Set PORTAL_E2E_ADMIN_DATABASE_URL to a disposable local PostgreSQL URL');
  }
  if (!['postgres:', 'postgresql:'].includes(url.protocol)
    || !['localhost', '127.0.0.1', '[::1]'].includes(url.hostname)
    || url.hash || [...url.searchParams.keys()].some(key => key !== 'sslmode')) {
    throw new Error('The crawl requires local PostgreSQL with no connection destination overrides');
  }
}
validateAdminUrl(process.env.PORTAL_E2E_ADMIN_DATABASE_URL);

const e2e = fileURLToPath(new URL('..', import.meta.url));
const parent = path.join(e2e, '.harness');
fs.mkdirSync(parent, { recursive: true, mode: 0o700 });
const scratch = fs.mkdtempSync(path.join(parent, 'crawl-'));
const out = path.join(scratch, 'portal');
const harnessEnv = { ...process.env, PORTAL_E2E_OUT: out, PORTAL_E2E_KEEP_DATABASE: '0' };
// This launcher seeds only Platform actors and a registry world via the CLI.
delete harnessEnv.PORTAL_E2E_SEED_PLAN;
delete harnessEnv.PORTAL_E2E_NATIVE_SCOPES;
delete harnessEnv.PORTAL_E2E_IDENTITY_DOMAIN;
const harnessDefaults = parseEnv(fs.readFileSync(path.join(e2e, 'test.env'), 'utf8'));
for (const key of Object.keys(harnessDefaults)) {
  if (!key.startsWith('PORTAL_E2E_')) delete harnessEnv[key];
}
const harness = spawn(process.execPath, ['harness.mjs', '--browser-crawl'], {
  cwd: e2e, env: harnessEnv, stdio: ['ignore', 'pipe', 'inherit'],
});
const harnessExit = once(harness, 'exit');
const output = createInterface({ input: harness.stdout });
let runner;
let signalCode;
function stop(code) {
  signalCode ??= code;
  runner?.kill('SIGTERM');
  harness.kill('SIGTERM');
}
const sigint = () => stop(130);
const sigterm = () => stop(143);
process.on('SIGINT', sigint);
process.on('SIGTERM', sigterm);

try {
  const ready = await Promise.race([
    once(output, 'line').then(([line]) => JSON.parse(line).status === 'ready'),
    harnessExit.then(() => false),
  ]);
  if (!ready || signalCode) throw new Error('Crawl harness did not become ready');
  const env = JSON.parse(fs.readFileSync(path.join(out, 'secrets', 'env.json'), 'utf8'));
  const runnerEnvironment = { ...process.env };
  for (const key of ['PORTAL_E2E_ADMIN_DATABASE_URL', 'DATABASE_URL', 'PORTAL_TEST_DATABASE_URL', 'OTERYN_DATABASE_URL']) {
    delete runnerEnvironment[key];
  }
  runner = spawn(process.execPath, [
    'node_modules/@playwright/test/cli.js', 'test', '--config=crawl/playwright.config.mjs',
    ...process.argv.slice(2),
  ], {
    cwd: e2e,
    env: {
      ...runnerEnvironment,
      PORTAL_E2E_ORIGIN: env.PORTAL_E2E_ORIGIN,
      PORTAL_E2E_DATABASE_URL: env.OTERYN_DATABASE_URL,
      PORTAL_E2E_BINARY: env.PORTAL_E2E_BINARY,
      PORTAL_E2E_CRAWL_OUTPUT: path.join(scratch, 'results'),
      PORTAL_E2E_AUTH_DIRECTORY: scratch,
    },
    stdio: 'inherit',
  });
  const runnerExit = once(runner, 'exit');
  process.exitCode = await Promise.race([
    runnerExit.then(([code]) => code ?? 1),
    harnessExit.then(async () => {
      runner.kill('SIGTERM');
      await runnerExit;
      return 1;
    }),
  ]);
} catch {
  console.error('Crawl launcher failed to start or run. See its local harness logs.');
  process.exitCode = 1;
} finally {
  output.close();
  runner?.kill('SIGTERM');
  harness.kill('SIGTERM');
  try {
    const [code] = await harnessExit;
    if (code !== 0) process.exitCode ||= 1;
  } catch {
    process.exitCode ||= 1;
  }
  process.exitCode = signalCode ?? process.exitCode;
  process.off('SIGINT', sigint);
  process.off('SIGTERM', sigterm);
  console.error(`Crawl diagnostics: ${scratch}`);
}
