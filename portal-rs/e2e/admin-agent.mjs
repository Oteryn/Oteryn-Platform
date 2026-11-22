// Run harness.mjs --staff-console, then from e2e/ run
// PORTAL_E2E_ADMIN_AGENT_DIRECTORY=.harness/run npm test -- gm-player-console.spec.mjs
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
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
import { SMTPServer } from 'smtp-server';

const agentId = 'staff-console-e2e';
const worldId = '019b0760-0000-7000-8000-000000000001';
export const nodeId = 'e2e-console-node';
export const characterId = 'e2e-console-knight';
export const characterName = 'E2E Console Knight';
const build = 'staff-console-e2e-build';
const player = { character_id: characterId, name: characterName, world_id: worldId, level: 42, online: true };
export const itemId = 'gm-e2e-inventory-item';
export const sanctionId = 'gm-e2e-sanction';
export const wrongCharacterId = 'gm-e2e-wrong-character';
export const wrongSanctionCharacterId = 'gm-e2e-wrong-sanction-target';
export const accountSanctionId = 'gm-e2e-account-sanction';

const results = {
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
  'sanction.list': ({ target }) => ({
    sanctions: [{
      sanction_id: sanctionId, account_ref: null,
      character_id: target.character_id === wrongSanctionCharacterId ? characterId : target.character_id,
      kind: 'warning', active: true, starts_at: '2026-11-15T00:00:00Z', expires_at: null,
    }, {
      sanction_id: accountSanctionId, account_ref: null, character_id: null,
      kind: 'notation', active: false, starts_at: '2026-11-15T00:00:00Z', expires_at: null,
    }],
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

// The browser qualification launcher already supplies a disposable database.
// Start a separate harness so attaching an agent cannot change another spec.
export async function startStaffConsoleHarness(databaseURL) {
  assert(databaseURL, 'Set PORTAL_E2E_ADMIN_AGENT_DIRECTORY or run browser qualification with a disposable database');
  const adminURL = new URL(databaseURL);
  assert(['localhost', '127.0.0.1', '[::1]'].includes(adminURL.hostname)
    && adminURL.pathname.startsWith('/portal_e2e_'), 'Staff console fixtures require a local portal_e2e_ database');
  adminURL.pathname = '/postgres';
  const listeners = [];
  const portEnv = {};
  try {
    for (const name of ['HTTP', 'INTERNAL', 'ADMISSION', 'GATEWAY']) {
      const listener = net.createServer().listen(0, '127.0.0.1');
      listeners.push(listener);
      await once(listener, 'listening');
      portEnv[`PORTAL_E2E_${name}_PORT`] = String(listener.address().port);
    }
  } finally {
    for (const listener of listeners) listener.close();
  }
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'staff-console-e2e-'));
  const out = path.join(directory, 'run');
  const log = fs.openSync(path.join(directory, 'harness.log'), 'a', 0o600);
  const env = { ...process.env, ...portEnv, PORTAL_E2E_OUT: out, PORTAL_E2E_ADMIN_DATABASE_URL: adminURL.href };
  delete env.PORTAL_E2E_SEED_PLAN;
  delete env.PORTAL_E2E_KEEP_DATABASE;
  const child = spawn(process.execPath, [fileURLToPath(new URL('./harness.mjs', import.meta.url)), '--staff-console'],
    { env, stdio: ['ignore', 'pipe', log] });
  fs.closeSync(log);
  const closed = once(child, 'close');
  const lines = createInterface({ input: child.stdout });
  async function stop() {
    if (child.exitCode === null && child.signalCode === null) child.kill('SIGTERM');
    await closed;
  }
  try {
    const ready = await Promise.race([
      once(lines, 'line').then(([line]) => JSON.parse(line).status),
      closed.then(([code]) => { throw new Error(`Staff console harness exited (${code}). See ${directory}/harness.log`); }),
    ]);
    assert.equal(ready, 'ready');
    return { out, async close() { await stop(); fs.rmSync(directory, { recursive: true, force: true }); } };
  } catch (error) {
    await stop();
    throw error;
  } finally {
    lines.close();
  }
}

export async function prepareAdminAgent({ out, dirs, openssl, server, caKey, caFile, ports }) {
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
      response.writeHead(incoming.statusCode, incoming.headers);
      incoming.pipe(response);
    });
    upstream.on('error', () => { response.writeHead(502).end(); });
    request.pipe(upstream);
  });
  await once(proxy.listen(0, '127.0.0.1'), 'listening');
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
  await once(smtp.listen(0, '127.0.0.1'), 'listening');
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
      OTERYN_ACCOUNT_LOGIN: 'true', OTERYN_ACCOUNT_MFA: 'true',
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
    close() {
      smtp.close();
      proxy.close();
    },
  };
}

export function adminAgent({ internalPort, caFile, client, resultKey, signerKey }) {
  const tls = new https.Agent({
    ca: fs.readFileSync(caFile), cert: fs.readFileSync(client.cert), key: fs.readFileSync(client.key),
    servername: 'localhost', minVersion: 'TLSv1.3',
  });
  const platformKey = createPublicKey({ key: signerKey, format: 'jwk' });
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
    hello: () => post('hello', {
      ...session, build_revision: build, catalog_digest: catalogDigest, operations,
      worlds: [worldId], nodes: [nodeId], clock: new Date().toISOString(),
    }),
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
    async answer(request, outcome) {
      assert(['success', 'refusal'].includes(outcome));
      const success = outcome === 'success';
      const clock = new Date().toISOString();
      const completed = Date.parse(clock) > Date.parse(request.issued_at) ? clock : request.issued_at;
      const envelope = {
        request_id: request.request_id, admin_operation_id: request.admin_operation_id,
        status: success ? 'succeeded' : 'rejected',
        error: success ? null : { code: 'ADM_FORBIDDEN', message: 'AdmForbidden', retryable: false },
        result: success ? results[request.operation](request) : null,
        dry_run_preview: null, game_ledger_id: 'gm-e2e-ledger-' + request.request_id, audit_event_ids: [],
        effective_revision: success ? 'gm-e2e-revision' : null, progress: null,
        completed_at: completed, agent_id: agentId, agent_build: build,
      };
      const signature = sign(null, signingBytes(envelope), { key: resultKey, format: 'jwk' }).toString('base64url');
      assert.deepEqual(await post('results', { ...envelope, signature }), { status: 'recorded' });
    },
    close: () => tls.destroy(),
  };
}
