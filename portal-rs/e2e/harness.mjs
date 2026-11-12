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
import { spawn, spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import pg from 'pg';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '..');
const log = (...line) => console.error('[harness]', ...line);

// ---- arguments and test.env -------------------------------------------------
const argv = process.argv.slice(2);
const option = name => { const i = argv.indexOf(name); return i >= 0 ? argv[i + 1] : undefined; };
const seedPlanFile = option('--seed-plan') || process.env.PORTAL_E2E_SEED_PLAN;
const exitAfterReady = argv.includes('--exit-after-ready');

const testEnv = {};
for (const line of fs.readFileSync(path.join(here, 'test.env'), 'utf8').split('\n')) {
  const match = /^([A-Z][A-Z0-9_]*)=(.*)$/.exec(line.trim());
  if (match) testEnv[match[1]] = process.env[match[1]] ?? match[2];
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

const adminURL = process.env.PORTAL_E2E_ADMIN_DATABASE_URL;
if (!adminURL) throw new Error('Set PORTAL_E2E_ADMIN_DATABASE_URL to a throwaway local PostgreSQL administrator URL');
if (!['localhost', '127.0.0.1', '[::1]'].includes(new URL(adminURL).hostname)) throw new Error('Local PostgreSQL only');

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
const admin = new pg.Client({ connectionString: adminURL });
await admin.connect();
await admin.query(`CREATE DATABASE "${dbName}"`);
const databaseURL = (() => { const url = new URL(adminURL); url.pathname = `/${dbName}`; return url.toString(); })();
log('database', dbName);

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
fs.writeFileSync(path.join(dirs.secrets, 'env.json'), JSON.stringify(env), { mode: 0o600 });

// ---- build, migrate ----------------------------------------------------------
function run(program, args, options = {}) {
  const result = spawnSync(program, args, { cwd: root, env, encoding: 'utf8', ...options });
  if (result.status !== 0) throw new Error(`${path.basename(program)} ${args[0] ?? ''} failed (${result.status})`);
  return result;
}
let binary = process.env.PORTAL_E2E_BINARY;
if (!binary) {
  log('cargo build --locked -p oteryn-portal');
  run('cargo', ['build', '--locked', '-p', 'oteryn-portal', '--quiet'], { stdio: ['ignore', 'inherit', 'inherit'], env: process.env });
  const meta = JSON.parse(run('cargo', ['metadata', '--format-version', '1', '--no-deps'], { env: process.env, maxBuffer: 64 << 20 }).stdout);
  // A private copy: a shared target directory may be rebuilt by another checkout.
  binary = path.join(dirs.bin, 'oteryn-portal');
  fs.copyFileSync(path.join(meta.target_directory, 'debug', 'oteryn-portal'), binary);
  fs.chmodSync(binary, 0o755);
}
log('migrate');
run(binary, ['migrate'], { stdio: ['ignore', fs.openSync(path.join(dirs.logs, 'migrate.log'), 'a'), fs.openSync(path.join(dirs.logs, 'migrate.log'), 'a')] });

// ---- services ----------------------------------------------------------------
const children = [];
let closing = false;
async function close(code = 0) {
  if (closing) return;
  closing = true;
  for (const child of children) if (child.exitCode === null) child.kill('SIGINT');
  await Promise.all(children.map(child => child.exitCode !== null ? null : new Promise(resolve => child.once('exit', resolve))));
  if (process.env.PORTAL_E2E_KEEP_DATABASE !== '1') await admin.query(`DROP DATABASE IF EXISTS "${dbName}" WITH (FORCE)`).catch(() => {});
  await admin.end().catch(() => {});
  process.exit(code);
}
process.on('SIGINT', () => close(0));
process.on('SIGTERM', () => close(0));
for (const role of ['serve', 'worker', 'gateway']) {
  const file = path.join(dirs.logs, `${role}.log`);
  const child = spawn(binary, [role], { cwd: root, env, stdio: ['ignore', fs.openSync(file, 'a'), fs.openSync(file, 'a')] });
  child.once('exit', status => { if (!closing) { log(`${role} exited (${status}); see ${file}`); close(1); } });
  children.push(child);
}

const deadline = Date.now() + 120_000;
async function until(probe, what) {
  for (;;) {
    if (await probe().catch(() => false)) return;
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
log('services ready');

// ---- seed plan ---------------------------------------------------------------
// {"steps":[{"name":"player","entity":"account","args":{...}}, ...]}; a string
// "${player.accountId}" in later args is replaced by that step's output field.
const seed = {};
const pending = [];
if (seedPlanFile) {
  const plan = JSON.parse(fs.readFileSync(seedPlanFile, 'utf8'));
  for (const step of plan.steps ?? plan) {
    const args = JSON.parse(JSON.stringify(step.args ?? {}), (key, value) => typeof value !== 'string' ? value : value.replace(/\$\{([A-Za-z0-9_-]+)\.([A-Za-z0-9_]+)\}/g, (_, name, field) => {
      if (seed[name]?.[field] === undefined) throw new Error(`seed step ${step.name}: unknown reference ${name}.${field}`);
      return String(seed[name][field]);
    }));
    const result = spawnSync(binary, ['seed', step.entity, JSON.stringify(args)], { cwd: root, env, encoding: 'utf8' });
    let output;
    try { output = JSON.parse(result.stdout); } catch { output = { error: 'unavailable', reason: 'no JSON output' }; }
    if (result.status === 3) { pending.push({ step: step.name, entity: step.entity, ...output }); seed[step.name] = output; continue; }
    if (result.status !== 0) { log(`seed step ${step.name} failed: ${JSON.stringify(output)}`); await close(1); }
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
