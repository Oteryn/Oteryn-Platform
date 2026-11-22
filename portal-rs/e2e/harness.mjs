// One-command portal-rs for the cross-repository E2E harness (Game server +
// client + portal-rs + PostgreSQL). Test-only; see README.md "Cross-repository
// harness". Run through ./start.sh, or directly:
//
//   PORTAL_E2E_ADMIN_DATABASE_URL=... node harness.mjs [--seed-plan plan.json] [--exit-after-ready]
//
// It creates a fresh portal_e2e_* database, migrates it, generates a throwaway
// CA with an internal-listener server certificate and one client certificate
// per internal Game identity, starts serve (with the internal mTLS listener),
// worker and gateway, optionally runs a seed plan through `oteryn-portal seed`,
// then prints ONE JSON object on stdout. Progress goes to stderr; service logs
// to <out>/logs. SIGINT/SIGTERM stops the services and drops the database.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import https from 'node:https';
import http from 'node:http';
import { Socket } from 'node:net';
import { spawn, spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import pg from 'pg';
import { localPostgresUrl } from './postgres-url.mjs';
import { withDeadline } from './support/harness-deadline.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '..');
const log = (...line) => console.error('[harness]', ...line);

// Pass through only validation text whose fields and format are known. Some
// seed errors include rejected input, such as an unknown argument name.
function seedDiagnostic(output) {
  if (output?.error === 'conflict') return 'conflicts with existing seed data';
  if (output?.error !== 'bad_input') return 'seed command unavailable or invalid response';
  const reason = output.reason;
  if (typeof reason === 'string') {
    if (reason.startsWith('unknown argument ')) return 'unknown argument (name omitted)';
    if (reason.startsWith('unknown entity ')) return 'unknown seed entity';
    if (/^(?:name|region) must be 1\.\.=96 characters, trimmed$/.test(reason) ||
        /^(?:name|region|worldId|channelId|accountId|email|password|locale|operationId|directory) (?:is required|must be a string|is not a UUID|must be a lower-case RFC 4122 UUIDv7)$/.test(reason)) {
      return reason;
    }
  }
  return 'invalid seed arguments';
}

// ---- arguments and test.env -------------------------------------------------
const argv = process.argv.slice(2);
const option = name => { const i = argv.indexOf(name); return i >= 0 ? argv[i + 1] : undefined; };
const seedPlanFile = option('--seed-plan') || process.env.PORTAL_E2E_SEED_PLAN;
const exitAfterReady = argv.includes('--exit-after-ready');
const outbox = argv.includes('--outbox');

const testEnv = {};
for (const line of fs.readFileSync(path.join(here, 'test.env'), 'utf8').split('\n')) {
  const match = /^([A-Z][A-Z0-9_]*)=(.*)$/.exec(line.trim());
  if (match) testEnv[match[1]] = process.env[match[1]] ?? match[2];
}
if (outbox) {
  Object.assign(testEnv, {
    OTERYN_ACCOUNT_LOGIN: 'true',
    OTERYN_ACCOUNT_CREDENTIALS: 'true',
    OTERYN_CHARACTER_SERVICES_ENABLED: 'true',
    OTERYN_CHARACTER_SERVICES_SCHEDULE_ENABLED: 'true',
    OTERYN_CHARACTER_SERVICES_CANCEL_ENABLED: 'true',
    OTERYN_CHARACTER_SERVICES_RENAME_ENABLED: 'true',
    OTERYN_CHARACTER_RENAME_PRODUCT_ID: 'character.rename',
    OTERYN_CHARACTER_RENAME_PRODUCT_VERSION: '1',
    OTERYN_CHARACTER_RENAME_PRICE: '70',
    OTERYN_WALLET_ENABLED: 'true',
    OTERYN_SHOP_ENABLED: 'true',
    OTERYN_SHOP_TOP_UPS_ENABLED: 'true',
    OTERYN_SHOP_TEST_PAYMENTS_ENABLED: 'true',
    OTERYN_SHOP_TOP_UP_PACKAGES: JSON.stringify([{ key: 'e2e-coins', coins: 1234, minor_units: 250, currency: 'PLN' }]),
    OTERYN_PAYMENTS_ENABLED: 'true',
    OTERYN_PREMIUM_ENABLED: 'true',
    PANEL_PREMIUM_PAYMENTS: 'true',
    PRODUCTS_ENTITLEMENTS_STORE_PURCHASE_ENABLED: 'true',
    PRODUCTS_ENTITLEMENTS_STORE_ENABLED_UNIT_KINDS: '["ITEM","PREMIUM_TIME"]',
    PLATFORM_BUILD_REVISION: '0000000000000000000000000000000000000001',
    OTERYN_CHARACTER_SERVICES_EXCHANGE_ENABLED: 'true',
    OTERYN_CHARACTER_SERVICES_PROJECTION_ENABLED: 'true',
    OTERYN_CHARACTER_SERVICES_COMMAND_AUTHORITY: testEnv.PORTAL_CHARACTER_COMMAND_SOURCE_AUTHORITY,
    OTERYN_CHARACTER_SERVICES_PROJECTION_AUTHORITY: testEnv.PORTAL_CHARACTER_SERVICES_PROJECTION_AUTHORITY,
    OTERYN_CHARACTER_SERVICES_CLOCK_UNCERTAINTY_SECONDS: '0',
    PORTAL_CHARACTER_SERVICES_CLOCK_UNCERTAINTY_SECONDS: '0',
    PORTAL_CHARACTER_SERVICES_FRESHNESS_SECONDS: '30',
  });
}
const port = name => {
  const value = Number(testEnv[name]);
  if (!Number.isInteger(value) || value < 1024 || value > 65535) throw new Error(`${name} must be a port`);
  return value;
};
const ports = {
  http: port('PORTAL_E2E_HTTP_PORT'),
  internal: port('PORTAL_E2E_INTERNAL_PORT'),
  admission: Number(process.env.PORTAL_E2E_ADMISSION_PORT || 18091),
  gateway: Number(process.env.PORTAL_E2E_GATEWAY_PORT || 18090),
};

const adminURL = localPostgresUrl(process.env.PORTAL_E2E_ADMIN_DATABASE_URL);

// The isolated puzzle app uses the existing local SMTP sink for real login.
if (argv.includes('--puzzle-browser')) {
  for (const key of ['OTERYN_SMTP_HOST', 'OTERYN_SMTP_USERNAME', 'OTERYN_SMTP_PASSWORD',
    'OTERYN_SMTP_FROM', 'OTERYN_MAIL_FROM', 'SSL_CERT_FILE']) {
    if (!process.env[key]) throw new Error('Puzzle browser harness requires ' + key + ' from local.mjs');
    testEnv[key] = process.env[key];
  }
  Object.assign(testEnv, {
    OTERYN_ACCOUNT_LOGIN: 'true',
    OTERYN_WORLD_ENABLED: 'true',
    OTERYN_DAILY_PUZZLES_ENABLED: 'true',
  });
}

// ---- output directory (ignored by git) -------------------------------------
const out = path.resolve(process.env.PORTAL_E2E_OUT || path.join(here, '.harness', 'run'));
if (fs.existsSync(out)) {
  if (!fs.existsSync(path.join(out, '.portal-e2e-harness'))) throw new Error(`${out} exists and is not a harness directory`);
  fs.rmSync(out, { recursive: true, force: true });
}
const dirs = { certs: path.join(out, 'certs'), clients: path.join(out, 'certs', 'clients'), witness: path.join(out, 'witness'), logs: path.join(out, 'logs'), secrets: path.join(out, 'secrets'), bin: path.join(out, 'bin') };
for (const dir of Object.values(dirs)) fs.mkdirSync(dir, { recursive: true, mode: 0o700 });
fs.writeFileSync(path.join(out, '.portal-e2e-harness'), '');

// ---- certificates (openssl, as local.mjs) ----------------------------------
function openssl(args) {
  const result = spawnSync('openssl', args, { stdio: ['ignore', 'ignore', 'pipe'] });
  if (result.status !== 0) throw new Error(`openssl ${args[0]} failed: ${result.stderr}`);
}
const caKey = path.join(dirs.secrets, 'ca-key.pem');
const caFile = path.join(dirs.certs, 'ca.pem');
openssl(['req', '-x509', '-newkey', 'ec', '-pkeyopt', 'ec_paramgen_curve:P-256', '-nodes', '-keyout', caKey, '-out', caFile, '-days', '2', '-subj', '/CN=Oteryn E2E Internal CA', '-addext', 'basicConstraints=critical,CA:TRUE', '-addext', 'keyUsage=critical,keyCertSign,cRLSign']);
function leaf(dir, name, san, usage) {
  const key = path.join(dir, `${name}.key`), cert = path.join(dir, `${name}.pem`), csr = path.join(dirs.secrets, `${name}.csr`), ext = path.join(dirs.secrets, `${name}.ext`);
  openssl(['req', '-newkey', 'ec', '-pkeyopt', 'ec_paramgen_curve:P-256', '-nodes', '-keyout', key, '-out', csr, '-subj', `/CN=${name}`]);
  fs.writeFileSync(ext, `subjectAltName=${san}\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=${usage}\n`);
  openssl(['x509', '-req', '-in', csr, '-CA', caFile, '-CAkey', caKey, '-CAcreateserial', '-out', cert, '-days', '2', '-extfile', ext]);
  fs.chmodSync(key, 0o600);
  return { cert, key };
}
const server = leaf(dirs.secrets, 'internal-server', 'DNS:localhost,IP:127.0.0.1', 'serverAuth');

// One client certificate per internal Game identity, each with a single DNS SAN.
// `env` names the variable that binds it; `map` marks a JSON-object variable.
const scopes = JSON.parse(process.env.PORTAL_E2E_NATIVE_SCOPES || '["019a0000-0000-7000-8000-00000000e001/019a0000-0000-7000-8000-00000000c401"]');
const identities = [
  ['native-evidence', 'GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY'],
  ['runtime-status', 'GAME_AUTH_NATIVE_RUNTIME_STATUS_IDENTITIES', scopes],
  ['scope-assignment', 'GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_IDENTITIES', scopes],
  ['bootstrap-intents', 'GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_MTLS_CLIENT_IDENTITY'],
  ['lcfa', 'PORTAL_LCFA_INTERNAL_IDENTITY'],
  ['journal-items', 'PORTAL_PLAYERDATA_JOURNAL_ITEMS_SAN'],
  ['journal-progress', 'PORTAL_PLAYERDATA_JOURNAL_PROGRESS_SAN'],
  ['character-commands', 'PORTAL_CHARACTER_COMMAND_IDENTITIES', []],
  ['character-services', 'PORTAL_CHARACTER_SERVICES_IDENTITY'],
  ['restore-notice', 'PORTAL_GAME_RESTORE_NOTICE_IDENTITY'],
  ['store', 'PRODUCTS_ENTITLEMENTS_STORE_MTLS_CLIENT_IDENTITY'],
  ['premium', 'PRODUCTS_ENTITLEMENTS_PREMIUM_SNAPSHOT_MTLS_CLIENT_IDENTITY'],
  ['loyalty-snapshot', 'PRODUCTS_ENTITLEMENTS_LOYALTY_SNAPSHOT_MTLS_CLIENT_IDENTITY'],
  ['loyalty-stream', 'PRODUCTS_ENTITLEMENTS_LOYALTY_STREAM_MTLS_CLIENT_IDENTITY'],
  ['coins', 'PORTAL_GAME_COINS_IDENTITIES', []],
  ['sanctions', 'PORTAL_SANCTION_EXPORT_IDENTITY'],
  ['security-revocations', 'PORTAL_SECURITY_REVOCATION_EXPORT_IDENTITY'],
  ['public-projections', 'PORTAL_PUBLIC_PROJECTIONS_SAN'],
];
const domain = process.env.PORTAL_E2E_IDENTITY_DOMAIN || 'game.e2e.oteryn.test';
const clients = {};
const identityEnv = {};
for (const [name, variable, members] of identities) {
  const san = `${name}.${domain}`;
  clients[name] = { san, env: variable, ...leaf(dirs.clients, name, `DNS:${san}`, 'clientAuth') };
  identityEnv[variable] = members ? JSON.stringify({ [san]: members }) : san;
}
if (new Set(Object.values(clients).map(c => c.san)).size !== identities.length) throw new Error('internal identities overlap');

// ---- database ----------------------------------------------------------------
const dbName = `portal_e2e_${crypto.randomBytes(6).toString('hex')}`;
async function withAdminConnection(operation) {
  const socket = new Socket();
  const admin = new pg.Client({
    connectionString: adminURL.href,
    stream: socket,
    connectionTimeoutMillis: 3_000,
    statement_timeout: 5_000,
    query_timeout: 6_500,
  });
  // A disconnect can arrive between queries. An active operation rejects
  // through its own promise, and the socket is always closed before returning.
  admin.on('error', () => {});
  try {
    await admin.connect();
    return await operation(admin);
  } finally {
    const ended = admin.end();
    socket.destroy();
    await ended;
  }
}

// Hold the same session lock for CREATE and reconciliation. A disconnected
// creator can still be finishing its statement when the cleanup client connects.
function lockDatabaseName(admin) {
  return admin.query('SELECT pg_advisory_lock(hashtextextended($1, 0))', [dbName]);
}
const children = new Set();
const useProcessGroups = process.platform !== 'win32';
const shutdownGraceMs = 2_000;

function launch(program, args, options) {
  if (closing) throw new Error('Harness is shutting down');
  const child = spawn(program, args, { cwd: root, detached: useProcessGroups, ...options });
  // A failed spawn also emits close. Resolve with the failure status so the
  // same cleanup path covers both failed spawns and processes that exit.
  child.on('error', () => {});
  const closed = new Promise(resolve => child.once('close', (status, signal) => {
    children.delete(tracked);
    resolve({ status, signal });
  }));
  const tracked = { child, closed };
  children.add(tracked);
  return tracked;
}

function signalChild(child, signal) {
  if (!child.pid) return;
  try {
    if (useProcessGroups) process.kill(-child.pid, signal);
    else child.kill(signal);
  } catch (error) {
    if (error.code !== 'ESRCH') throw error;
  }
}

async function stopChild({ child, closed }) {
  signalChild(child, 'SIGINT');
  const escalation = setTimeout(() => signalChild(child, 'SIGKILL'), shutdownGraceMs);
  try {
    await closed;
  } finally {
    clearTimeout(escalation);
    // A build can exit before a compiler subprocess in its group.
    if (useProcessGroups) signalChild(child, 'SIGKILL');
  }
}
let closing = false;
let outboxBrowser;
let createRequested = false;
let creation;
async function close(code = 0) {
  if (closing) return;
  closing = true;
  // Wait for an in-flight CREATE while stopping services. Reconciliation
  // requires the creator to finish and uses a separate bounded connection.
  const [, services] = await Promise.allSettled([
    creation,
    withDeadline(Promise.all([
      ...[...children].map(stopChild),
      Promise.resolve(outboxBrowser).then(closeBrowser => closeBrowser?.()),
    ]), 6_000, 'service shutdown timed out'),
  ]);
  if (services.status === 'rejected') {
    log('failed to stop services for database', dbName);
    code = 1;
  }
  if (createRequested && process.env.PORTAL_E2E_KEEP_DATABASE !== '1') {
    try {
      await withAdminConnection(async admin => {
        await lockDatabaseName(admin);
        await admin.query('DROP DATABASE IF EXISTS "' + dbName + '" WITH (FORCE)');
      });
    } catch {
      log('failed to reconcile disposable database', dbName);
      code = 1;
    }
  }
  if (code !== 0) log('harness failed for database', dbName);
  process.exit(code);
}
process.on('SIGINT', () => close(0));
process.on('SIGTERM', () => close(0));

async function createDatabase() {
  await withAdminConnection(async admin => {
    if (closing) return;
    await lockDatabaseName(admin);
    if (closing) return;
    const existing = await admin.query('SELECT 1 FROM pg_database WHERE datname=$1', [dbName]);
    if (closing) return;
    if (existing.rowCount !== 0) throw new Error('Disposable database name already exists');
    createRequested = true;
    try {
      await admin.query('CREATE DATABASE "' + dbName + '"');
    } catch (error) {
      // A definite name collision belongs to the existing database owner.
      if (error.code === '42P04') createRequested = false;
      throw error;
    }
  });
}
log('database', dbName);
creation = createDatabase();

async function start() {
  await creation;
  if (closing) return;
  const databaseURL = new URL(`/${dbName}`, adminURL).href;

  // Gateway service token for the native admission listener (sha256 hex on the portal side).
  const token = crypto.randomBytes(32).toString('base64url');
  const tokenFile = path.join(dirs.secrets, 'gateway-token');
  const tokenHashFile = path.join(dirs.secrets, 'gateway-token.sha256');
  fs.writeFileSync(tokenFile, `${token}\n`, { mode: 0o600 });
  fs.writeFileSync(tokenHashFile, `${crypto.createHash('sha256').update(token).digest('hex')}\n`, { mode: 0o600 });

  const env = {
    PATH: process.env.PATH, HOME: process.env.HOME, ...testEnv,
    ...identityEnv,
    OTERYN_DATABASE_URL: databaseURL,
    OTERYN_BIND: `127.0.0.1:${ports.http}`,
    OTERYN_STATIC_DIR: path.join(root, 'static'),
    OTERYN_INTERNAL_TLS_BIND: `127.0.0.1:${ports.internal}`,
    OTERYN_INTERNAL_TLS_CERT_FILE: server.cert,
    OTERYN_INTERNAL_TLS_KEY_FILE: server.key,
    OTERYN_INTERNAL_TLS_CLIENT_CA_FILE: caFile,
    GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY: dirs.witness,
    PORTAL_NATIVE_ADMISSION_BIND: `127.0.0.1:${ports.admission}`,
    PORTAL_GATEWAY_TOKEN_SHA256_FILE: tokenHashFile,
    OTERYN_PLATFORM_BASE_URL: `http://127.0.0.1:${ports.admission}`,
    OTERYN_PLATFORM_SERVICE_TOKEN_FILE: tokenFile,
    GATEWAY_LISTEN_ADDR: `127.0.0.1:${ports.gateway}`,
    GATEWAY_NATIVE_LOGIN_ENABLED: 'true',
    OTERYN_MFA_KEY: crypto.randomBytes(32).toString('hex'),
  };
  for (const key of ['PORTAL_E2E_HTTP_PORT', 'PORTAL_E2E_INTERNAL_PORT']) delete env[key];

  // ---- build, migrate ----------------------------------------------------------
  async function run(program, args, {
    commandEnv = env, stdio = ['ignore', 'pipe', 'ignore'],
    maxBuffer = 1 << 20, allowed = [0], reportPid = false, label,
  } = {}) {
    const { child, closed } = launch(program, args, { env: commandEnv, stdio });
    const command = `${path.basename(program)} ${args[0] ?? ''}`;
    if (reportPid && child.pid) log(`${command} started (pid ${child.pid})`);
    const output = [];
    let bytes = 0;
    child.stdout?.on('data', chunk => {
      bytes += chunk.length;
      if (bytes <= maxBuffer) output.push(chunk);
      else signalChild(child, 'SIGKILL');
    });
    const { status, signal } = await closed;
    if (closing) throw new Error('Harness is shutting down');
    if (!allowed.includes(status) || bytes > maxBuffer) {
      log(`${label ?? command} failed (${signal ?? status})`);
      throw new Error('Harness command failed');
    }
    return { status, stdout: Buffer.concat(output).toString('utf8') };
  }
  let binary = process.env.PORTAL_E2E_BINARY;
  if (!binary) {
    log('cargo build --locked -p oteryn-portal');
    await run('cargo', ['build', '--locked', '-p', 'oteryn-portal', '--quiet'], { stdio: ['ignore', 'inherit', 'inherit'], commandEnv: process.env });
    const meta = JSON.parse((await run('cargo', ['metadata', '--format-version', '1', '--no-deps'], { commandEnv: process.env, maxBuffer: 64 << 20 })).stdout);
    // A private copy: a shared target directory may be rebuilt by another checkout.
    binary = path.join(dirs.bin, 'oteryn-portal');
    fs.copyFileSync(path.join(meta.target_directory, 'debug', 'oteryn-portal'), binary);
    fs.chmodSync(binary, 0o755);
  }
  binary = path.resolve(root, binary);
  env.PORTAL_E2E_BINARY = binary;
  if (outbox) {
    env.OTERYN_SHOP_TEST_PAYMENT_KEY = crypto.randomBytes(32).toString('hex');
    env.PORTAL_E2E_OUTBOX_REPORT = path.join(out, 'report.json');
    env.PORTAL_E2E_OUTBOX_BINARY = binary;
    env.PORTAL_E2E_DATABASE_URL = databaseURL;
    env.OTERYN_SMTP_HOST = 'localhost';
    env.OTERYN_SMTP_USERNAME = 'local';
    env.OTERYN_SMTP_PASSWORD = crypto.randomBytes(24).toString('hex');
    env.OTERYN_SMTP_FROM = 'portal@local.test';
    env.OTERYN_MAIL_FROM = env.OTERYN_SMTP_FROM;
    const { startOutboxBrowser } = await import('./support/outbox-browser.mjs');
    if (closing) return;
    outboxBrowser = startOutboxBrowser({ upstreamPort: ports.http, certificate: server, caFile, env });
    await outboxBrowser;
    if (closing) return;
  }
  fs.writeFileSync(path.join(dirs.secrets, 'env.json'), JSON.stringify(env), { mode: 0o600 });

  log('migrate');
  const migrationLog = fs.openSync(path.join(dirs.logs, 'migrate.log'), 'a');
  try {
    await run(binary, ['migrate'], { stdio: ['ignore', migrationLog, migrationLog], reportPid: true });
  } finally {
    fs.closeSync(migrationLog);
  }

  // ---- services ----------------------------------------------------------------
  for (const role of ['serve', 'worker', 'gateway']) {
    const file = path.join(dirs.logs, `${role}.log`);
    const fd = fs.openSync(file, 'a');
    try {
      const { child, closed } = launch(binary, [role], { env, stdio: ['ignore', fd, fd] });
      child.once('error', () => { log(`failed to start ${role}; see ${file}`); void close(1); });
      void closed.then(({ status, signal }) => {
        if (!closing) { log(`${role} exited (${signal ?? status}); see ${file}`); void close(1); }
      });
    } finally {
      fs.closeSync(fd);
    }
  }

  const deadline = Date.now() + 120_000;
  async function until(probe, what) {
    for (;;) {
      const ready = await probe().catch(() => false);
      if (closing) throw new Error('Harness is shutting down');
      if (ready) return;
      if (Date.now() > deadline) { log(`${what} not ready`); await close(1); }
      await new Promise(resolve => setTimeout(resolve, 250));
    }
  }
  const status = (request, options, body) => new Promise((resolve, reject) => {
    const req = request(options, res => { res.resume(); res.on('end', () => resolve(res.statusCode)); });
    req.on('error', reject);
    req.end(body);
  });
  // Internal listener: a probe with a generated client certificate must get an HTTP answer.
  const probe = clients['bootstrap-intents'];
  const internalRequest = (client, pathName, body) => status(https.request, {
    host: '127.0.0.1', port: ports.internal, path: pathName, method: 'POST', servername: 'localhost',
    ca: fs.readFileSync(caFile), ...(client ? { cert: fs.readFileSync(client.cert), key: fs.readFileSync(client.key) } : {}),
    minVersion: 'TLSv1.3', headers: { 'content-type': 'application/json', 'content-length': Buffer.byteLength(body) },
  }, body);
  await until(() => status(http.request, { host: '127.0.0.1', port: ports.http, path: '/', method: 'GET' }).then(() => true), 'public listener');
  await until(() => internalRequest(probe, '/internal/v1/', '{}').then(() => true), 'internal mTLS listener');
  await until(() => status(http.request, { host: '127.0.0.1', port: ports.admission, path: '/health', method: 'GET' }).then(code => code === 200), 'native admission listener');
  await until(() => status(http.request, { host: '127.0.0.1', port: ports.gateway, path: '/health', method: 'GET' }).then(code => code === 200), 'gateway listener');
  log('services ready');

  // ---- seed plan ---------------------------------------------------------------
  // {"steps":[{"name":"player","entity":"account","args":{...}}, ...]}; a string
  // "${player.accountId}" in later args is replaced by that step's output field.
  const seed = {};
  const pending = [];
  if (seedPlanFile) {
    const plan = JSON.parse(fs.readFileSync(seedPlanFile, 'utf8'));
    for (const [index, step] of (plan.steps ?? plan).entries()) {
      const name = typeof step.name === 'string' && /^[a-zA-Z0-9_.-]{1,64}$/.test(step.name)
        ? step.name : '#' + (index + 1);
      const label = 'seed step ' + name;
      const args = JSON.parse(JSON.stringify(step.args ?? {}), (key, value) => typeof value !== 'string' ? value : value.replace(/\$\{([A-Za-z0-9_-]+)\.([A-Za-z0-9_]+)\}/g, (_, name, field) => {
        if (seed[name]?.[field] === undefined) {
          log(label + ' failed: unknown seed reference');
          throw new Error('Harness seed reference failed');
        }
        return String(seed[name][field]);
      }));
      const result = await run(binary, ['seed', step.entity, JSON.stringify(args)], { allowed: [0, 1, 2, 3], label });
      let output;
      try { output = JSON.parse(result.stdout); } catch { output = { error: 'unavailable', reason: 'no JSON output' }; }
      if (result.status === 1 || result.status === 2) {
        log(label + ' failed (' + result.status + '): ' + seedDiagnostic(output));
        throw new Error('Harness seed step failed');
      }
      if (result.status === 3) { pending.push({ step: step.name, entity: step.entity, ...output }); seed[step.name] = output; continue; }
      seed[step.name ?? step.entity] = output;
    }
  }

  // ---- report ----------------------------------------------------------------
  const report = {
    status: 'ready',
    database: dbName,
    baseUrls: {
      public: `http://127.0.0.1:${ports.http}`,
      internal: `https://127.0.0.1:${ports.internal}`,
      nativeAdmission: `http://127.0.0.1:${ports.admission}`,
      gateway: `http://127.0.0.1:${ports.gateway}`,
    },
    outputDirectory: out,
    certificateDirectory: dirs.clients,
    caFile,
    internalServerName: 'localhost',
    clients: Object.fromEntries(Object.entries(clients).map(([name, c]) => [name, { san: c.san, env: c.env, cert: c.cert, key: c.key }])),
    witnessDirectory: dirs.witness,
    logs: Object.fromEntries(['migrate', 'serve', 'worker', 'gateway'].map(role => [role, path.join(dirs.logs, `${role}.log`)])),
    seed,
    pending,
    pid: process.pid,
  };
  fs.writeFileSync(path.join(out, 'report.json'), JSON.stringify(report, null, 2));
  console.log(JSON.stringify(report));
  if (exitAfterReady) await close(0);
}
try {
  await start();
} catch (error) {
  if (!closing) {
    log('startup failed', error.code ?? error.name);
    await close(1);
  }
}
