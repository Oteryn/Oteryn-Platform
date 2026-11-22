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
import { spawnSync } from 'node:child_process';
import { setTimeout as delay } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';
import { HarnessProcesses } from './harness-processes.mjs';
import { localDatabaseClient, localDatabaseURL } from './local-database.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '..');
const log = (...line) => console.error('[harness]', ...line);

// ---- arguments and test.env -------------------------------------------------
const argv = process.argv.slice(2);
const option = name => { const i = argv.indexOf(name); return i >= 0 ? argv[i + 1] : undefined; };
const seedPlanFile = option('--seed-plan') || process.env.PORTAL_E2E_SEED_PLAN;
const exitAfterReady = argv.includes('--exit-after-ready');
const staffMode = argv.includes('--staff-console');
const parentLifetime = argv.includes('--parent-lifetime-stdin');

const testEnv = {};
for (const line of fs.readFileSync(path.join(here, 'test.env'), 'utf8').split('\n')) {
  const match = /^([A-Z][A-Z0-9_]*)=(.*)$/.exec(line.trim());
  if (match) testEnv[match[1]] = process.env[match[1]] ?? match[2];
}
if (staffMode && (testEnv.OTERYN_ENV !== 'testing' || testEnv.APP_ENV !== 'testing')) {
  throw new Error('Staff console harness requires OTERYN_ENV=testing and APP_ENV=testing');
}
const port = name => {
  const value = Number(testEnv[name]);
  if (!Number.isInteger(value) || value < 1024 || value > 65535) throw new Error(`${name} must be a port`);
  return value;
};
const configuredPorts = parentLifetime && staffMode ? undefined : {
  http: port('PORTAL_E2E_HTTP_PORT'),
  internal: port('PORTAL_E2E_INTERNAL_PORT'),
  admission: Number(process.env.PORTAL_E2E_ADMISSION_PORT || 18091),
  gateway: Number(process.env.PORTAL_E2E_GATEWAY_PORT || 18090),
};

const adminURL = process.env.PORTAL_E2E_ADMIN_DATABASE_URL;
if (!adminURL) throw new Error('Set PORTAL_E2E_ADMIN_DATABASE_URL to a throwaway local PostgreSQL administrator URL');
const adminDatabaseURL = localDatabaseURL(adminURL);
const adminConnectionString = adminDatabaseURL.href;
const databaseLock = 'SELECT pg_advisory_lock(hashtextextended($1, 0))';

const abort = new AbortController();
const processes = new HarnessProcesses();
const cleanup = [];
let ready = false;
let stopping = false;
let childrenStopped;
const onCleanup = action => cleanup.push(action);

function readJSON(source, label) {
  try {
    return JSON.parse(source);
  } catch {
    throw new Error(`${label} contains invalid JSON`);
  }
}

async function dropDatabase(dbName, reconcile) {
  const deadline = performance.now() + 75_000;
  let failure;
  for (let attempt = 0; ; attempt++) {
    const remaining = Math.floor(deadline - performance.now());
    if (remaining <= 0) throw failure;
    const budget = Math.min(5_000 * 2 ** attempt, 30_000, remaining);
    const attemptDeadline = performance.now() + budget;
    const url = new URL(adminConnectionString);
    url.searchParams.set('statement_timeout', String(budget));
    const { client, socket } = localDatabaseClient(url, budget);
    // An unexpected disconnect can also emit an idle-client error between
    // connect and query. The awaited operations determine whether to retry.
    client.on('error', () => {});
    const query = (text, values) => {
      const remaining = Math.floor(attemptDeadline - performance.now());
      if (remaining <= 0) throw new Error('PostgreSQL cleanup attempt timed out');
      return client.query({ text, values, query_timeout: remaining });
    };
    try {
      await client.connect();
      if (reconcile) {
        // Wait until the creating backend releases its session lock, including
        // when CREATE committed before the client lost its response.
        await query(databaseLock, [dbName]);
        const { rows } = await query(
          'SELECT pg_get_userbyid(datdba) = current_user AS owned FROM pg_database WHERE datname = $1', [dbName],
        );
        if (!rows.length) return;
        if (!rows[0].owned) throw Object.assign(new Error('Database belongs to another role'), { code: '42501' });
      }
      await query(`DROP DATABASE IF EXISTS "${dbName}" WITH (FORCE)`);
      return;
    } catch (error) {
      if (error.code === '42501' || /^28/.test(error.code)) throw error;
      failure = error;
    } finally {
      socket.destroy();
      await client.end();
    }
    const backoff = Math.min(250 * 2 ** attempt, 2_000);
    if (performance.now() + backoff >= deadline) throw failure;
    log('retrying database cleanup', failure.code ?? failure.name);
    await delay(backoff);
  }
}

function launch(program, args, options) {
  abort.signal.throwIfAborted();
  return processes.launch(program, args, { cwd: root, ...options });
}

async function close(code) {
  if (stopping) return;
  stopping = true;
  if (!await childrenStopped) code = 1;
  for (const action of cleanup.reverse()) {
    try {
      await action(code);
    } catch (error) {
      log('cleanup failed', error.code ?? error.name);
      code = 1;
    }
  }
  process.exit(code);
}

function stop(code, message) {
  if (abort.signal.aborted) return;
  abort.abort({ code, message });
  childrenStopped = processes.stop(log);
  if (ready) {
    if (message) log(message);
    void close(code);
  }
}
process.on('SIGINT', () => stop(0));
process.on('SIGTERM', () => stop(0));
if (parentLifetime) {
  process.stdin.once('end', () => stop(0));
  process.stdin.on('error', () => stop(1, 'Managed harness lifetime pipe failed'));
  process.stdout.on('error', error => error.code === 'EPIPE' ? stop(0) : stop(1, 'Managed harness report pipe failed'));
  process.stdin.resume();
}

function lockOutputDirectory(out) {
  fs.mkdirSync(out, { recursive: true, mode: 0o700 });
  const fd = fs.openSync(out, fs.constants.O_RDONLY | fs.constants.O_DIRECTORY | fs.constants.O_NOFOLLOW);
  try {
    // flock acts on the inherited open directory descriptor. Node retains the
    // lock after flock exits, including throughout database and file cleanup.
    const lock = spawnSync('flock', ['-n', '-E', '75', '3'], { stdio: ['ignore', 'ignore', 'ignore', fd] });
    if (lock.status === 75) throw new Error(`${out} is in use by another harness run`);
    if (lock.status !== 0) throw new Error('Could not lock the harness output directory (requires flock)');
    const locked = fs.fstatSync(fd, { bigint: true });
    const current = fs.lstatSync(out, { bigint: true, throwIfNoEntry: false });
    if (current?.ino !== locked.ino || current.dev !== locked.dev) {
      throw new Error(`${out} was replaced during lock acquisition`);
    }
  } catch (error) {
    fs.closeSync(fd);
    throw error;
  }
  onCleanup(() => fs.closeSync(fd));
}

async function start() {
  // ---- output directory (ignored by git) -------------------------------------
  const out = path.resolve(process.env.PORTAL_E2E_OUT || path.join(here, '.harness', 'run'));
  lockOutputDirectory(out);
  const marker = path.join(out, '.portal-e2e-harness');
  const marked = fs.lstatSync(marker, { throwIfNoEntry: false });
  if (!marked && fs.readdirSync(out).length === 0) {
    fs.writeFileSync(marker, '', { flag: 'wx', mode: 0o600 });
  } else if (!marked?.isFile()) {
    throw new Error(`${out} exists and is not a harness directory`);
  }
  const dirs = { certs: path.join(out, 'certs'), clients: path.join(out, 'certs', 'clients'), witness: path.join(out, 'witness'), logs: path.join(out, 'logs'), secrets: path.join(out, 'secrets'), bin: path.join(out, 'bin') };
  const reportFile = path.join(out, 'report.json');
  for (const file of [dirs.certs, dirs.witness, dirs.logs, dirs.secrets, dirs.bin, reportFile]) {
    fs.rmSync(file, { recursive: true, force: true });
  }
  const privatePaths = new Set([dirs.secrets, dirs.clients, dirs.witness, dirs.bin]);
  const owned = [];
  onCleanup(code => {
    if (parentLifetime || !ready || code !== 0) {
      for (const file of owned) fs.rmSync(file, { recursive: true, force: true });
    }
    if (parentLifetime && ready && code === 0) {
      // Remove the managed run's remaining files under its lock. The parent
      // only owns the launcher log and can remove the wrapper if it is empty.
      for (const dir of [dirs.certs, dirs.logs]) fs.rmSync(dir, { recursive: true, force: true });
      const remaining = fs.readdirSync(out);
      if (remaining.length === 1 && remaining[0] === path.basename(marker)) {
        fs.unlinkSync(marker);
        try {
          fs.rmdirSync(out);
        } catch (error) {
          if (error.code !== 'ENOTEMPTY') throw error;
        }
      }
    }
  });
  for (const dir of Object.values(dirs)) {
    fs.mkdirSync(dir, { mode: 0o700 });
    if (privatePaths.has(dir)) owned.push(dir);
  }
  const staffTools = staffMode ? await import('./admin-agent.mjs') : undefined;
  const reservations = parentLifetime && staffTools ? await staffTools.reserveStaffPorts(abort.signal) : undefined;
  if (reservations) onCleanup(reservations.release);
  const ports = reservations?.ports ?? configuredPorts;

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
  const scopes = readJSON(process.env.PORTAL_E2E_NATIVE_SCOPES || '["019a0000-0000-7000-8000-00000000e001/019a0000-0000-7000-8000-00000000c401"]',
    'PORTAL_E2E_NATIVE_SCOPES');
  const identities = [
    ['native-evidence', 'GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY'],
    ['runtime-status', 'GAME_AUTH_NATIVE_RUNTIME_STATUS_IDENTITIES', scopes],
    ['scope-assignment', 'GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_IDENTITIES', scopes],
    ['bootstrap-intents', 'GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_MTLS_CLIENT_IDENTITY'],
    ['lcfa', 'PORTAL_LCFA_INTERNAL_IDENTITY'],
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
  const { client: admin, socket: adminSocket } = localDatabaseClient(adminDatabaseURL, 10_000);
  admin.on('error', () => stop(1, 'PostgreSQL administrator connection lost'));
  abort.signal.addEventListener('abort', () => adminSocket.destroy(), { once: true });
  let creation = 'not-issued';
  onCleanup(async () => {
    if ((creation === 'created' || creation === 'pending')
        && (!ready || process.env.PORTAL_E2E_KEEP_DATABASE !== '1')) {
      await dropDatabase(dbName, creation === 'pending');
    }
  });
  onCleanup(() => {
    adminSocket.destroy();
    return admin.end();
  });
  await admin.connect();
  abort.signal.throwIfAborted();
  await admin.query({ text: databaseLock, values: [dbName], query_timeout: 15_000 });
  abort.signal.throwIfAborted();
  creation = 'pending';
  try {
    await admin.query({ text: `CREATE DATABASE "${dbName}"`, query_timeout: 60_000 });
    creation = 'created';
  } catch (error) {
    // A PostgreSQL statement error establishes that CREATE failed. Connection
    // loss and client-side timeouts leave the result unknown.
    if (error.code === '42P04' || error.code === '23505' || error.severity === 'ERROR') creation = 'failed';
    throw error;
  }
  abort.signal.throwIfAborted();
  adminDatabaseURL.pathname = `/${dbName}`;
  const databaseURL = adminDatabaseURL.href;
  log('database', dbName);

  // Gateway service token for the native admission listener (sha256 hex on the portal side).
  const token = crypto.randomBytes(32).toString('base64url');
  const tokenFile = path.join(dirs.secrets, 'gateway-token');
  const tokenHashFile = path.join(dirs.secrets, 'gateway-token.sha256');
  fs.writeFileSync(tokenFile, `${token}\n`, { mode: 0o600 });
  fs.writeFileSync(tokenHashFile, `${crypto.createHash('sha256').update(token).digest('hex')}\n`, { mode: 0o600 });

  const staffConsole = await staffTools?.prepareAdminAgent({ out, dirs, openssl, server, caKey, caFile, ports, signal: abort.signal });
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
    ...staffConsole?.env,
  };
  for (const key of ['PORTAL_E2E_HTTP_PORT', 'PORTAL_E2E_INTERNAL_PORT']) delete env[key];
  fs.writeFileSync(path.join(dirs.secrets, 'env.json'), JSON.stringify(env), { mode: 0o600 });

  // ---- build, migrate ----------------------------------------------------------
  async function run(program, args, options = {}) {
    const { child, done } = launch(program, args, { env, ...options });
    let stdout = '';
    child.stdout?.setEncoding('utf8').on('data', chunk => { stdout += chunk; });
    let cancel;
    const interrupted = new Promise((_, reject) => {
      cancel = () => reject(abort.signal.reason);
      abort.signal.addEventListener('abort', cancel, { once: true });
    });
    try {
      const result = await Promise.race([done, interrupted]);
      abort.signal.throwIfAborted();
      return { ...result, stdout };
    } finally {
      abort.signal.removeEventListener('abort', cancel);
    }
  }

  async function checkedRun(program, args, options) {
    const result = await run(program, args, options);
    if (result.status !== 0) {
      throw new Error(`${path.basename(program)} ${args[0]} failed (${result.error?.code ?? result.signal ?? result.status})`);
    }
    return result;
  }

  function withLog(name, startProcess) {
    const fd = fs.openSync(path.join(dirs.logs, name + '.log'), 'a');
    try {
      return startProcess(['ignore', fd, fd]);
    } finally {
      fs.closeSync(fd);
    }
  }

  const configuredBinary = process.env.PORTAL_E2E_BINARY;
  let binary = configuredBinary && path.resolve(root, configuredBinary);
  if (!binary) {
    log('cargo build --locked -p oteryn-portal');
    await checkedRun('cargo', ['build', '--locked', '-p', 'oteryn-portal', '--quiet'], { stdio: ['ignore', 'inherit', 'inherit'], env: process.env });
    const meta = JSON.parse((await checkedRun('cargo', ['metadata', '--format-version', '1', '--no-deps'], { env: process.env, stdio: ['ignore', 'pipe', 'inherit'] })).stdout);
    // A private copy: a shared target directory may be rebuilt by another checkout.
    binary = path.join(dirs.bin, 'oteryn-portal');
    fs.copyFileSync(path.join(meta.target_directory, 'debug', 'oteryn-portal'), binary);
    fs.chmodSync(binary, 0o755);
  }
  log('migrate');
  if (staffConsole) fs.writeFileSync(path.join(dirs.secrets, 'seed-binary'), binary, { mode: 0o600 });
  await withLog('migrate', stdio => checkedRun(binary, ['migrate'], { stdio }));

  // ---- services ----------------------------------------------------------------
  if (reservations) await reservations.release();
  for (const role of ['serve', 'worker', 'gateway']) {
    const { done } = withLog(role, stdio => launch(binary, [role], { env, stdio }));
    void done.then(({ status, signal, error }) => {
      if (abort.signal.aborted) return;
      const file = path.join(dirs.logs, role + '.log');
      let code = 1;
      if (reservations && !ready && status === 1 && (role === 'serve' || role === 'gateway')) {
        try {
          if (/"error":"[^"\n]*Address already in use \(os error 98\)"/.test(fs.readFileSync(file, 'utf8'))) {
            code = staffTools.startupBindConflict;
          }
        } catch {
          // If the service log is unreadable, report the original service failure.
        }
      }
      stop(code, `${role} exited (${error?.code ?? signal ?? status}); see ${file}`);
    });
  }

  const deadline = Date.now() + 120_000;
  async function until(probe, what) {
    for (;;) {
      abort.signal.throwIfAborted();
      const available = await probe().catch(() => false);
      abort.signal.throwIfAborted();
      if (available) return;
      if (Date.now() > deadline) throw new Error(`${what} not ready`);
      await delay(250, undefined, { signal: abort.signal });
    }
  }
  const status = (request, options, body) => new Promise((resolve, reject) => {
    const req = request(options, res => {
      res.on('error', reject);
      res.on('end', () => resolve(res.statusCode));
      res.resume();
    });
    req.on('error', reject);
    req.setTimeout(5_000, () => req.destroy());
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
  if (reservations) await until(() => status(http.request, { host: '127.0.0.1', port: ports.gateway, path: '/ready', method: 'GET' }).then(code => code === 200), 'gateway listener');
  log('services ready');

  // ---- seed plan ---------------------------------------------------------------
  // {"steps":[{"name":"player","entity":"account","args":{...}}, ...]}; a string
  // "${player.accountId}" in later args is replaced by that step's output field.
  const seed = {};
  const pending = [];
  if (seedPlanFile) {
    const plan = readJSON(fs.readFileSync(seedPlanFile, 'utf8'), 'Seed plan');
    for (const step of plan.steps ?? plan) {
      const args = JSON.parse(JSON.stringify(step.args ?? {}), (key, value) => typeof value !== 'string' ? value : value.replace(/\$\{([A-Za-z0-9_-]+)\.([A-Za-z0-9_]+)\}/g, (_, name, field) => {
        if (seed[name]?.[field] === undefined) throw new Error(`seed step ${step.name}: unknown reference ${name}.${field}`);
        return String(seed[name][field]);
      }));
      const result = await run(binary, ['seed', step.entity, JSON.stringify(args)], { stdio: ['ignore', 'pipe', 'ignore'] });
      let output;
      try { output = JSON.parse(result.stdout); } catch { output = { error: 'unavailable', reason: 'no JSON output' }; }
      if (result.status === 3) { pending.push({ step: step.name, entity: step.entity, ...output }); seed[step.name] = output; continue; }
      if (result.status !== 0) throw new Error(`seed step ${step.name} failed (${result.error?.code ?? result.signal ?? result.status})`);
      seed[step.name ?? step.entity] = output;
    }
  }

  // ---- report ----------------------------------------------------------------
  const report = {
    status: 'ready',
    database: dbName,
    baseUrls: {
      public: `http://127.0.0.1:${ports.http}`,
      ...(staffConsole && { staff: staffConsole.origin }),
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
  const reportFd = fs.openSync(reportFile, 'wx', 0o600);
  owned.push(reportFile);
  try {
    fs.writeFileSync(reportFd, JSON.stringify(report, null, 2));
  } finally {
    fs.closeSync(reportFd);
  }
  console.log(JSON.stringify(report));
  ready = true;
}

try {
  await start();
  if (exitAfterReady) stop(0);
} catch (error) {
  if (!abort.signal.aborted) stop(1, error.message);
  const { code, message } = abort.signal.reason;
  if (message) log(message);
  await close(code);
}
