import { spawn, spawnSync } from 'node:child_process';
import { once } from 'node:events';
import { randomBytes } from 'node:crypto';
import fs from 'node:fs';
import http from 'node:http';
import https from 'node:https';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const e2e = fileURLToPath(new URL('../', import.meta.url));
const root = path.dirname(e2e);
const admin = process.env.PORTAL_E2E_ADMIN_DATABASE_URL ?? '';
const database = URL.canParse(admin) ? new URL(admin) : undefined;
if (!database || !['postgres:', 'postgresql:'].includes(database.protocol) ||
    !['localhost', '127.0.0.1', '[::1]'].includes(database.hostname)) {
  throw new Error('PORTAL_E2E_ADMIN_DATABASE_URL must point to disposable local PostgreSQL');
}
const cargoEnv = { ...process.env, CARGO_BUILD_JOBS: '2' };
let binary = process.env.PORTAL_E2E_BINARY;
if (!binary) {
  const build = spawnSync('cargo', ['build', '--locked', '-p', 'oteryn-portal'], {
    cwd: root, env: cargoEnv, stdio: 'inherit',
  });
  if (build.status !== 0) throw new Error('Portal build failed');
  const metadata = spawnSync('cargo', ['metadata', '--format-version=1', '--no-deps'], {
    cwd: root, env: cargoEnv, encoding: 'utf8',
  });
  if (metadata.status !== 0) throw new Error('Cargo metadata failed');
  binary = path.join(JSON.parse(metadata.stdout).target_directory, 'debug', 'oteryn-portal');
}
const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'portal-quality-'));
const out = path.join(scratch, 'harness');
const sourceScan = path.join(scratch, 'routes');
fs.mkdirSync(path.join(sourceScan, 'e2e'), { recursive: true });
for (const directory of ['crates', 'bin']) {
  fs.symlinkSync(path.join(root, directory), path.join(sourceScan, directory), 'dir');
}
const scanner = path.join(sourceScan, 'e2e/inventory.py');
fs.copyFileSync(path.join(e2e, 'inventory.py'), scanner);
if (spawnSync('python3', [scanner], { stdio: 'inherit' }).status !== 0) {
  throw new Error('Portal route inventory generation failed');
}
const proxyPort = Number(process.env.PORTAL_QUALITY_HTTPS_PORT ?? 18443);
const origin = `https://localhost:${proxyPort}`;
const world = '019a0000-0000-7000-8000-00000000e001';
const seedPlan = path.join(scratch, 'seed.json');
fs.writeFileSync(seedPlan, JSON.stringify({
  steps: [{ name: 'world', entity: 'world', args: { worldId: world, name: 'Quality World' } }],
}), { mode: 0o600 });

// The cross-repository launcher forwards only its test.env allowlist. Apply the
// browser flags to the real executable through its documented binary override.
const quote = value => "'" + value.replaceAll("'", "'\\''") + "'";
const wrapper = path.join(scratch, 'portal');
fs.writeFileSync(wrapper, [
  '#!/bin/sh',
  'set -a',
  `. ${quote(path.join(e2e, 'quality/browser-flags.env'))}`,
  `OTERYN_ADMIN_HOST=localhost:${proxyPort}`,
  'OTERYN_SMTP_HOST=localhost',
  'OTERYN_SMTP_USERNAME=quality',
  `OTERYN_SMTP_PASSWORD=${randomBytes(24).toString('hex')}`,
  'OTERYN_MAIL_FROM=portal@example.invalid',
  'OTERYN_SMTP_FROM=portal@example.invalid',
  `OTERYN_RESET_DELIVERY_KEY=${randomBytes(32).toString('hex')}`,
  `OTERYN_NOTIFY_UNSUBSCRIBE_KEY=${randomBytes(32).toString('hex')}`,
  `OTERYN_NOTIFY_UNSUBSCRIBE_URL=${quote(origin + '/en/unsubscribe')}`,
  `exec ${quote(path.resolve(binary))} "$@"`,
  '',
].join('\n'), { mode: 0o700 });

const harnessLog = fs.openSync(path.join(scratch, 'harness.log'), 'w', 0o600);
const launcher = spawn(process.execPath, ['harness.mjs', '--seed-plan', seedPlan], {
  cwd: e2e,
  env: {
    ...cargoEnv,
    PORTAL_E2E_BINARY: wrapper,
    PORTAL_E2E_OUT: out,
    OTERYN_PUBLIC_ORIGIN: origin,
  },
  stdio: ['ignore', 'pipe', harnessLog],
});
const launcherExit = once(launcher, 'exit');
let proxy;
let tests;
const stop = () => {
  if (tests) tests.kill('SIGINT');
  else launcher.kill('SIGTERM');
};
process.once('SIGINT', stop);
process.once('SIGTERM', stop);

try {
  let reportText = '';
  const report = await Promise.race([
    once(launcher.stdout, 'end').then(() => { throw new Error('Harness closed before readiness'); }),
    new Promise((resolve, reject) => launcher.stdout.on('data', bytes => {
      reportText += bytes;
      const end = reportText.indexOf('\n');
      if (end === -1) return;
      try {
        resolve(JSON.parse(reportText.slice(0, end)));
      } catch {
        reject(new Error('Harness readiness report is invalid'));
      }
    })),
  ]);
  const secrets = JSON.parse(fs.readFileSync(path.join(out, 'secrets/env.json'), 'utf8'));
  proxy = https.createServer({
    key: fs.readFileSync(secrets.OTERYN_INTERNAL_TLS_KEY_FILE),
    cert: fs.readFileSync(secrets.OTERYN_INTERNAL_TLS_CERT_FILE),
  }, (request, response) => {
    const upstream = http.request(new URL(report.baseUrls.public), {
      path: request.url, method: request.method,
      headers: { ...request.headers, 'x-forwarded-proto': 'https' },
    }, result => {
      response.writeHead(result.statusCode, result.headers);
      result.pipe(response);
    });
    upstream.on('error', () => response.destroy());
    response.on('close', () => {
      if (!response.writableFinished) upstream.destroy();
    });
    request.pipe(upstream);
  });
  proxy.listen(proxyPort, '127.0.0.1');
  await once(proxy, 'listening');
  tests = spawn(process.execPath, ['node_modules/@playwright/test/cli.js', 'test',
    '--config=quality/playwright.config.mjs', ...process.argv.slice(2)], {
    cwd: e2e,
    env: {
      ...process.env,
      PORTAL_E2E_ORIGIN: origin,
      PORTAL_E2E_DATABASE_URL: secrets.OTERYN_DATABASE_URL,
      PORTAL_QUALITY_SEED_BINARY: wrapper,
      PORTAL_QUALITY_SEED_ENV: path.join(out, 'secrets/env.json'),
      PORTAL_QUALITY_WORLD: report.seed.world.worldId,
      PORTAL_QUALITY_INVENTORY: path.join(sourceScan, 'e2e/inventory.json'),
    },
    stdio: 'inherit',
  });
  const testsExit = once(tests, 'exit');
  const [code] = await Promise.race([
    testsExit,
    launcherExit.then(async () => {
      tests.kill('SIGINT');
      await testsExit;
      throw new Error('Harness exited while browser audits were running');
    }),
  ]);
  process.exitCode = code ?? 1;
} catch (error) {
  console.error(`${error.message}. Local harness evidence: ${scratch}`);
  process.exitCode = 1;
} finally {
  proxy?.closeAllConnections();
  proxy?.close();
  launcher.kill('SIGTERM');
  await launcherExit;
  fs.closeSync(harnessLog);
}
