// Run harness.mjs --staff-console, then from e2e/ run
// PORTAL_E2E_ADMIN_AGENT_DIRECTORY=.harness/run npm test -- staff-console.spec.mjs
import assert from 'node:assert/strict';
import { execFile, spawn } from 'node:child_process';
import { createHash, createPublicKey, generateKeyPairSync, randomBytes, sign, verify } from 'node:crypto';
import { once } from 'node:events';
import fs from 'node:fs';
import http from 'node:http';
import https from 'node:https';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';
import { SMTPServer } from 'smtp-server';
import { staffDatabaseURL } from './local-database.mjs';

export const agentId = 'staff-console-e2e';
export const worldId = '019b0760-0000-7000-8000-000000000001';
export const nodeId = 'e2e-console-node';
export const characterId = '019b0760-0000-7000-8000-000000000010';
export const characterName = 'E2E Console Knight';
const build = 'staff-console-e2e-build';
const world = {
  world_id: worldId, name: 'E2E Console World', state: 'online', channels: [],
  ruleset_revision: 'rules-e2e', content_revision: 'content-e2e',
  map_revision: 'map-e2e', policy_revision: 'policy-e2e',
};
const node = {
  node_id: nodeId, host: 'node.e2e.test', build, uptime_seconds: 42, worlds: [worldId],
  ready: true, cpu_percent: 17, memory_bytes: '4096',
};
const player = { character_id: characterId, name: characterName, world_id: worldId, level: 42, online: true };
export const resultRevision = 'staff-console-e2e-revision';
export const ledgerId = 'staff-console-e2e-ledger';

export const itemId = 'gm-e2e-inventory-item';
export const sanctionId = 'gm-e2e-sanction';
export const wrongCharacterId = 'gm-e2e-wrong-character';
export const wrongSanctionCharacterId = 'gm-e2e-wrong-sanction-target';
export const accountSanctionId = 'gm-e2e-account-sanction';
export const wrongAccountSanctionCharacterId = '019b0760-0000-7000-8000-000000000011';

const results = {
  'ops.overview.get': () => ({ agents: [], worlds: [world], nodes: [node], players_online: 41, db_available: true, alerts: [] }),
  'world.list': () => ({ worlds: [world] }),
  'world.get': () => world,
  'node.list': () => ({ nodes: [node] }),
  'node.get': () => node,
  'db.status': () => ({
    tables: [], connections: [{ role: 'e2e-admin', connections: 3 }], locks_waiting: 0, cache_hit_ratio: 1,
  }),
  'db.migrations.list': () => ({
    migrations: [{ version: 'e2e-migration-42', description: 'Fixture migration', applied: true, checksum_drift: false }],
  }),
  'player.search': ({ params }) => ({
    players: characterName.startsWith(params.query) ? [player] : [],
  }),
  'player.character.get': ({ target }) => ({
    ...player,
    character_id: target.character_id === wrongCharacterId ? characterId : target.character_id,
    vocation: 'E2E Knight', skills: [{ name: 'sword', level: 17, progress: 0 }],
    position: { x: 42, y: 27, z: 7 }, premium: false, bank_balance: '42',
    sanctions: [], last_logins: [],
  }),
  'player.inventory.get': () => ({
    items: [{ instance_id: itemId, type_id: 42, parent_id: null, location: 'bag', count: '2' }],
  }),
  'sanction.list': ({ target }, accountRef, sanctionCharacter = target.character_id) => ({
    sanctions: [{
      sanction_id: sanctionId, account_ref: null,
      character_id: sanctionCharacter,
      kind: 'warning', active: true, starts_at: '2026-11-15T00:00:00Z', expires_at: null,
    }, ...(accountRef ? [{
      sanction_id: accountSanctionId, account_ref: accountRef, character_id: null,
      kind: 'notation', active: false, starts_at: '2026-11-15T00:00:00Z', expires_at: null,
    }] : [])],
  }),
  'admin.ledger.query': ({ params }, ledger) => ({
    entries: ledger.filter(entry =>
      (!params.request_id || params.request_id === entry.request_id)
      && (!params.admin_id || params.admin_id === entry.admin_id)
      && (!params.operation || params.operation === entry.operation)
      && (!params.target || Object.values(entry.target).includes(params.target))),
  }),
};
const operations = Object.keys(results).map(operation => ({ operation, version: 1 }));
const catalogDigest = createHash('sha256').update(JSON.stringify(operations)).digest('base64url');

// ADM-X1 signs RFC 8785 canonical JSON with the signature member omitted.
function canonical(value) {
  if (Array.isArray(value)) return '[' + value.map(canonical).join(',') + ']';
  if (value !== null && typeof value === 'object') {
    return '{' + Object.keys(value).sort().map(key => JSON.stringify(key) + ':' + canonical(value[key])).join(',') + '}';
  }
  return JSON.stringify(value);
}
function signingBytes({ signature, ...envelope }) {
  return Buffer.from(canonical(envelope));
}

export function staffSeedEnvironment(saved) {
  assert(saved.OTERYN_ENV === 'testing' && saved.APP_ENV === 'testing',
    'Staff console seeding requires OTERYN_ENV=testing and APP_ENV=testing');
  const database = staffDatabaseURL(saved.OTERYN_DATABASE_URL);
  return {
    OTERYN_ENV: 'testing',
    APP_ENV: 'testing',
    OTERYN_DATABASE_URL: database.href,
    OTERYN_MFA_KEY: saved.OTERYN_MFA_KEY,
  };
}

export function readStaffFixture(directory) {
  const secret = name => fs.readFileSync(path.join(directory, 'secrets', name), 'utf8');
  const json = name => {
    const contents = secret(name);
    try {
      return JSON.parse(contents);
    } catch {
      throw new Error(`Invalid staff fixture JSON in ${name}`);
    }
  };
  return {
    settings: json('admin-agent.json'),
    env: staffSeedEnvironment(json('env.json')),
    seedBinary: secret('seed-binary'),
  };
}

const execSeed = promisify(execFile);
export const staffSeedTimeoutMs = 30_000;

export async function runStaffSeed(binary, env, entity, input, timeout = staffSeedTimeoutMs) {
  let stdout;
  try {
    ({ stdout } = await execSeed(binary, ['seed', entity, JSON.stringify(input)], {
      env, timeout, killSignal: 'SIGKILL', maxBuffer: 1 << 20,
    }));
  } catch (error) {
    // The subprocess error holds the command arguments and can include credentials.
    if (error.code === 'ERR_CHILD_PROCESS_STDIO_MAXBUFFER') throw new Error(`Portal seed ${entity} exceeded its output limit`);
    if (error.killed) throw new Error(`Portal seed ${entity} exceeded its ${timeout} ms deadline`);
    throw new Error(`Portal seed ${entity} failed (${error.signal ?? error.code ?? 'unknown status'})`);
  }
  try {
    return JSON.parse(stdout);
  } catch {
    throw new Error(`Portal seed ${entity} returned invalid JSON`);
  }
}

export const startupBindConflict = 75;

export async function reserveStaffPorts(signal) {
  const listeners = [];
  const ports = {};
  const release = async () => {
    await Promise.all(listeners.splice(0).map(listener => new Promise(resolve => listener.close(resolve))));
  };
  try {
    for (const name of ['http', 'internal', 'admission', 'gateway']) {
      const listener = net.createServer(socket => socket.destroy()).listen({ port: 0, host: '127.0.0.1', signal });
      listeners.push(listener);
      await once(listener, 'listening', { signal });
      ports[name] = listener.address().port;
    }
  } catch (error) {
    await release();
    throw error;
  }
  return { ports, release };
}

export async function staffPortsOccupied(ports) {
  for (const port of ports) {
    const probe = net.createServer(socket => socket.destroy());
    try {
      await once(probe.listen(port, '127.0.0.1'), 'listening');
    } catch (error) {
      if (error.code === 'EADDRINUSE') return true;
      throw error;
    } finally {
      if (probe.listening) await new Promise(resolve => probe.close(resolve));
    }
  }
  return false;
}

// The browser qualification launcher already supplies a disposable database.
// Start a separate harness so attaching an agent cannot change another spec.
export async function startStaffConsoleHarness(databaseURL) {
  assert(databaseURL, 'Set PORTAL_E2E_ADMIN_AGENT_DIRECTORY or run browser qualification with a disposable database');
  const adminURL = staffDatabaseURL(databaseURL);
  adminURL.pathname = '/postgres';
  for (let attempt = 1; ; attempt++) {
    try {
      return await startStaffConsoleAttempt(adminURL);
    } catch (error) {
      if (error.exitCode !== startupBindConflict || attempt === 3) throw error;
    }
  }
}

async function startStaffConsoleAttempt(adminURL) {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'staff-console-e2e-'));
  const out = path.join(directory, 'run');
  const logPath = path.join(directory, 'harness.log');
  const log = fs.openSync(logPath, 'a', 0o600);
  const env = { ...process.env, PORTAL_E2E_OUT: out, PORTAL_E2E_ADMIN_DATABASE_URL: adminURL.href };
  delete env.PORTAL_E2E_SEED_PLAN;
  delete env.PORTAL_E2E_KEEP_DATABASE;
  const child = spawn(process.execPath, [fileURLToPath(new URL('./harness.mjs', import.meta.url)),
    '--staff-console', '--parent-lifetime-stdin'], { env, stdio: ['pipe', 'pipe', log] });
  fs.closeSync(log);
  // The child's exit reports a startup failure even if stdin closes first.
  child.stdin.on('error', () => {});
  const closed = once(child, 'close');
  const lines = createInterface({ input: child.stdout });
  async function stop() {
    child.stdin.end();
    return closed;
  }
  try {
    const ready = await Promise.race([
      once(lines, 'line').then(([line]) => JSON.parse(line).status),
      closed.then(([code, signal]) => {
        throw Object.assign(new Error(`Staff console harness exited (${signal || code}). See ${logPath}`), { exitCode: code });
      }),
    ]);
    assert.equal(ready, 'ready');
    return {
      out,
      async close() {
        const [code, signal] = await stop();
        if (code !== 0) throw new Error(`Staff console harness teardown failed (${signal || code}). See ${logPath}`);
        fs.rmSync(logPath, { force: true });
        try {
          fs.rmdirSync(directory);
        } catch (error) {
          if (error.code !== 'ENOTEMPTY' && error.code !== 'ENOENT') throw error;
        }
      },
    };
  } catch (error) {
    await stop();
    throw error;
  } finally {
    lines.close();
  }
}

export async function prepareAdminAgent({ out, dirs, openssl, server, caKey, caFile, ports, signal }) {
  signal.throwIfAborted();
  const cn = `oteryn-admin-agent/${agentId}`;
  const san = `${agentId}.e2e.test`;
  const client = { cert: path.join(dirs.secrets, 'admin-agent.pem'), key: path.join(dirs.secrets, 'admin-agent.key') };
  const csr = path.join(dirs.secrets, 'admin-agent.csr');
  const ext = path.join(dirs.secrets, 'admin-agent.ext');
  openssl(['req', '-newkey', 'ec', '-pkeyopt', 'ec_paramgen_curve:P-256', '-nodes',
    '-keyout', client.key, '-out', csr, '-subj', `/CN=${cn.replace('/', '\\/')}`]);
  fs.chmodSync(client.key, 0o600);
  fs.writeFileSync(ext, `subjectAltName=DNS:${san}\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=clientAuth\n`);
  openssl(['x509', '-req', '-in', csr, '-CA', caFile, '-CAkey', caKey,
    '-CAcreateserial', '-out', client.cert, '-days', '2', '-extfile', ext]);
  const resultKey = generateKeyPairSync('ed25519');
  const signer = generateKeyPairSync('ed25519');
  const rawPublicKey = key => key.export({ format: 'jwk' }).x;
  const now = Math.floor(Date.now() / 1000);
  const notBefore = now - 60, notAfter = now + 86400;
  const signerFile = path.join(dirs.secrets, 'admin-control-signer.key');
  fs.writeFileSync(signerFile, Buffer.from(signer.privateKey.export({ format: 'jwk' }).d, 'base64url'), { mode: 0o600 });

  // The staff CSRF guard requires its configured HTTPS origin. Reuse the
  // disposable harness TLS certificate and forward only to the public listener.
  const tls = {
    cert: fs.readFileSync(server.cert), key: fs.readFileSync(server.key),
  };
  const proxy = https.createServer(tls, (request, response) => {
    const upstream = http.request({
      host: '127.0.0.1', port: ports.http, path: request.url,
      method: request.method, headers: request.headers,
    }, incoming => {
      incoming.on('error', () => response.destroy());
      response.writeHead(incoming.statusCode, incoming.headers);
      incoming.pipe(response);
    });
    upstream.on('error', () => {
      if (response.headersSent) response.destroy();
      else if (!response.destroyed) response.writeHead(502).end();
    });
    response.once('close', () => {
      if (!response.writableFinished) upstream.destroy();
    });
    request.pipe(upstream);
  });
  signal.addEventListener('abort', () => {
    proxy.close();
    proxy.closeAllConnections();
  }, { once: true });
  await once(proxy.listen(0, '127.0.0.1'), 'listening', { signal });
  signal.throwIfAborted();
  const host = `localhost:${proxy.address().port}`;
  const origin = `https://${host}`;
  // Login configures the real security-notification consumer. Its disposable
  // mail sink accepts authenticated TLS delivery and discards the messages.
  const smtpUser = 'staff-console';
  const smtpPassword = randomBytes(24).toString('hex');
  const smtp = new SMTPServer({
    ...tls, secure: true,
    onAuth(auth, _session, callback) {
      if (auth.username !== smtpUser || auth.password !== smtpPassword) {
        callback(new Error('Invalid local SMTP credentials'));
        return;
      }
      callback(null, { user: smtpUser });
    },
    onData(stream, _session, callback) {
      stream.once('end', () => callback(null));
      stream.resume();
    },
  });
  await new Promise((resolve, reject) => {
    signal.addEventListener('abort', () => {
      smtp.close();
      reject(signal.reason);
    }, { once: true });
    smtp.once('error', reject);
    smtp.listen(0, '127.0.0.1', resolve);
  });
  signal.throwIfAborted();
  const file = path.join(out, 'secrets', 'admin-agent.json');
  fs.writeFileSync(file, JSON.stringify({
    origin, internalPort: ports.internal, caFile, client,
    resultKey: resultKey.privateKey.export({ format: 'jwk' }),
    signerKey: signer.publicKey.export({ format: 'jwk' }),
  }), { mode: 0o600 });
  return {
    origin,
    env: {
      OTERYN_PUBLIC_ORIGIN: origin, OTERYN_ADMIN_HOST: host,
      OTERYN_ADMIN_PANEL: 'true', OTERYN_ADMIN_CONSOLE: 'true',
      OTERYN_ACCOUNT_LOGIN: 'true', OTERYN_ACCOUNT_MFA: 'true', OTERYN_ACCOUNT_CREDENTIALS: 'true',
      OTERYN_SMTP_HOST: 'localhost', OTERYN_SMTP_TLS_MODE: 'implicit',
      OTERYN_SMTP_PORT: String(smtp.server.address().port), OTERYN_SMTP_USERNAME: smtpUser,
      OTERYN_SMTP_PASSWORD: smtpPassword, OTERYN_SMTP_FROM: 'portal@local.test',
      OTERYN_MAIL_FROM: 'portal@local.test', SSL_CERT_FILE: caFile,
      OTERYN_NOTIFY_UNSUBSCRIBE_KEY: randomBytes(32).toString('hex'),
      OTERYN_NOTIFY_UNSUBSCRIBE_URL: `${origin}/en/unsubscribe`,
      PORTAL_ADMIN_CONTROL_ENABLED: 'true',
      PORTAL_ADMIN_CONTROL_SIGNER_KEY_FILE: signerFile,
      PORTAL_ADMIN_CONTROL_SIGNER_KEY_ID: 'platform-admin-signer-1',
      PORTAL_ADMIN_CONTROL_SIGNER_NOT_BEFORE: String(notBefore),
      PORTAL_ADMIN_CONTROL_SIGNER_NOT_AFTER: String(notAfter),
      PORTAL_ADMIN_CONTROL_AGENTS_JSON: JSON.stringify([{
        cn, identity: { san }, allowed_worlds: [worldId], allowed_nodes: [nodeId],
        result_key: { public_key: rawPublicKey(resultKey.publicKey), not_before: notBefore, not_after: notAfter },
      }]),
    },
  };
}

export async function publishStaffPortfolio({ internalPort, caFile, lcfaClient, lcfaAuthority }, portfolios, epoch, lagSeconds = 0) {
  const peer = new https.Agent({
    ca: fs.readFileSync(caFile), cert: fs.readFileSync(lcfaClient.cert), key: fs.readFileSync(lcfaClient.key),
    servername: 'localhost', minVersion: 'TLSv1.3',
  });
  const timestamp = String(Math.floor(Date.now() / 1000));
  const common = { contract_version: 1, source_authority: lcfaAuthority, projection_epoch: epoch };
  const publish = (suffix, document) => new Promise((resolve, reject) => {
    const body = canonical(document);
    const request = https.request({
      host: '127.0.0.1', port: internalPort, agent: peer,
      path: '/internal/v1/game-auth/native-account-characters' + suffix, method: 'POST',
      headers: { 'content-type': 'application/json', 'content-length': Buffer.byteLength(body) },
    }, response => {
      response.resume();
      response.on('end', () => resolve(response.statusCode));
      response.on('error', reject);
    });
    request.setTimeout(15000, () => request.destroy(new Error('LCFA publication timed out')));
    request.on('error', reject);
    request.end(body);
  });
  try {
    for (const { account, character } of portfolios) {
      assert.equal(await publish('', {
        ...common, operation: 'PublishAccountCharactersV1', account_id: account,
        projection_revision: '1', source_observed_at: timestamp,
        characters: [{ character_id: character, world_id: worldId, name: characterName, availability: 'AVAILABLE' }],
      }), 200);
    }
    assert.equal(await publish('/watermark', {
      ...common, operation: 'PublishProjectionWatermarkV1', complete_through: String(Number(timestamp) - lagSeconds), observed_at: timestamp,
    }), 200);
  } finally {
    peer.destroy();
  }
}

export function adminAgent({ internalPort, caFile, client, resultKey, signerKey }) {
  const tls = new https.Agent({
    ca: fs.readFileSync(caFile), cert: fs.readFileSync(client.cert), key: fs.readFileSync(client.key),
    servername: 'localhost', minVersion: 'TLSv1.3',
  });
  const platformKey = createPublicKey({ key: signerKey, format: 'jwk' });
  const ledger = [];
  // Workers restart between projects. A newer epoch supersedes the last hello.
  const session = { agent_id: agentId, owner_epoch: Date.now() };
  function post(route, value) {
    return new Promise((resolve, reject) => {
      const body = JSON.stringify(value);
      const request = https.request({
        host: '127.0.0.1', port: internalPort, agent: tls,
        path: '/internal/v1/admin-control/' + route, method: 'POST',
        headers: { 'content-type': 'application/json', 'content-length': Buffer.byteLength(body) },
      }, response => {
        let data = '';
        response.setEncoding('utf8');
        response.on('data', chunk => { data += chunk; });
        response.on('error', reject);
        response.on('end', () => {
          if (response.statusCode !== 200) {
            reject(new Error(`Admin Agent ${route} returned HTTP ${response.statusCode}`));
            return;
          }
          try { resolve(JSON.parse(data)); } catch (error) { reject(error); }
        });
      });
      request.setTimeout(15000, () => request.destroy(new Error(`Admin Agent ${route} timed out`)));
      request.on('error', reject);
      request.end(body);
    });
  }
  return {
    hello() {
      session.owner_epoch = Math.max(Date.now(), session.owner_epoch + 1);
      return post('hello', {
        ...session, build_revision: build, catalog_digest: catalogDigest, operations,
        worlds: [worldId], nodes: [nodeId], clock: new Date().toISOString(),
      });
    },
    heartbeat: () => post('heartbeat', { ...session, clock: new Date().toISOString() }),
    async poll(waitSeconds = 0) {
      const requests = await post('channel', { ...session, wait_seconds: waitSeconds });
      for (const request of requests) {
        assert(verify(null, signingBytes(request), platformKey, Buffer.from(request.signature, 'base64url')),
          'Platform request has a valid ADM-X1 signature');
        assert(Object.hasOwn(results, request.operation), 'Platform requested an advertised operation');
      }
      return requests;
    },
    async answer(request, outcome, accountRef, sanctionCharacter) {
      assert(['success', 'refusal', 'timeout'].includes(outcome));
      if (outcome === 'timeout') return; // A delivered request expires without an agent result.
      const success = outcome === 'success';
      const clock = new Date().toISOString();
      const completed = Date.parse(clock) > Date.parse(request.issued_at) ? clock : request.issued_at;
      const entry = {
        request_id: request.request_id, admin_operation_id: request.admin_operation_id,
        ledger_id: ledgerId + '-' + request.request_id, status: success ? 'succeeded' : 'rejected',
        operation: request.operation, admin_id: request.actor.admin_id, risk_class: request.risk_class,
        target: request.target, started_at: request.issued_at, finished_at: completed,
      };
      const envelope = {
        request_id: request.request_id, admin_operation_id: request.admin_operation_id,
        status: entry.status,
        error: success ? null : { code: 'ADM_FORBIDDEN', message: 'AdmForbidden', retryable: false },
        result: success ? (
          request.operation === 'admin.ledger.query'
            ? results[request.operation](request, ledger)
            : results[request.operation](request, accountRef, sanctionCharacter)
        ) : null,
        dry_run_preview: null, game_ledger_id: entry.ledger_id, audit_event_ids: [],
        effective_revision: success ? resultRevision : null, progress: null,
        completed_at: completed, agent_id: agentId, agent_build: build,
      };
      const signature = sign(null, signingBytes(envelope), { key: resultKey, format: 'jwk' }).toString('base64url');
      assert.deepEqual(await post('results', { ...envelope, signature }), { status: 'recorded' });
      ledger.push(entry);
      return completed;
    },
    close: () => tls.destroy(),
  };
}
