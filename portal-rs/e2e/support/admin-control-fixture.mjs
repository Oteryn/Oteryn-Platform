import crypto from 'node:crypto';
import fs from 'node:fs';
import https from 'node:https';
import path from 'node:path';

const fixture = JSON.parse(fs.readFileSync(new URL('admin-control-results.json', import.meta.url)));
export const agentId = 'e2e-local';
export const worldId = fixture.World.world_id;
export const nodeId = fixture.Node.node_id;
const build = 'gm-read-fixture';
const fixtures = {
  'ops.overview.get': fixture.Overview,
  'world.list': {worlds: [fixture.World], next_cursor: null},
  'world.get': fixture.World,
  'node.list': {nodes: [fixture.Node], next_cursor: null},
  'node.get': fixture.Node,
  'db.status': fixture.DatabaseStatus,
  'db.migrations.list': fixture.MigrationPage,
  'admin.ledger.query': fixture.LedgerPage,
  'admin.policy.get': fixture.PolicyView,
};
export const advertisedOperations = Object.keys(fixtures);

export function prepareAdminControl(scratch, openssl, internalOrigin) {
  const material = name => path.join(scratch, `admin-control-${name}`);
  const ca = path.join(scratch, 'ca.pem');
  const key = material('client.key'), cert = material('client.pem');
  const csr = material('client.csr'), extensions = material('client.ext');
  const signer = material('platform.seed'), publicKey = material('platform-public.pem');
  const resultKey = material('result.key');
  const cn = `oteryn-admin-agent/${agentId}`;
  const san = 'admin-control.game.e2e.oteryn.test';

  openssl(['req', '-newkey', 'ec', '-pkeyopt', 'ec_paramgen_curve:P-256', '-nodes',
    '-keyout', key, '-out', csr, '-subj', `/CN=${cn.replace('/', '\\/')}`]);
  fs.chmodSync(key, 0o600);
  fs.writeFileSync(extensions,
    `subjectAltName=DNS:${san}\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=clientAuth\n`);
  openssl(['x509', '-req', '-in', csr, '-CA', ca, '-CAkey', path.join(scratch, 'ca-key.pem'),
    '-CAcreateserial', '-out', cert, '-days', '2', '-extfile', extensions]);

  // RFC 8410 Ed25519 PKCS#8 prefix for the raw seed consumed by the Platform signer.
  const seed = crypto.randomBytes(32);
  fs.writeFileSync(signer, seed, {mode: 0o600});
  const platformKey = crypto.createPrivateKey({
    key: Buffer.concat([Buffer.from('302e020100300506032b657004220420', 'hex'), seed]),
    type: 'pkcs8', format: 'der',
  });
  fs.writeFileSync(publicKey, crypto.createPublicKey(platformKey).export({type: 'spki', format: 'pem'}));
  const result = crypto.generateKeyPairSync('ed25519');
  fs.writeFileSync(resultKey, result.privateKey.export({type: 'pkcs8', format: 'pem'}), {mode: 0o600});
  const notBefore = Math.floor(Date.now() / 1000) - 60;
  const notAfter = notBefore + 172800;
  return {
    PORTAL_ADMIN_CONTROL_SIGNER_KEY_FILE: signer,
    PORTAL_ADMIN_CONTROL_SIGNER_KEY_ID: 'platform-admin-signer-1',
    PORTAL_ADMIN_CONTROL_SIGNER_NOT_BEFORE: String(notBefore),
    PORTAL_ADMIN_CONTROL_SIGNER_NOT_AFTER: String(notAfter),
    PORTAL_ADMIN_CONTROL_AGENTS_JSON: JSON.stringify([{
      cn, identity: {san}, allowed_worlds: [worldId], allowed_nodes: [nodeId],
      result_key: {
        public_key: result.publicKey.export({type: 'spki', format: 'der'}).subarray(-32).toString('base64url'),
        not_before: notBefore, not_after: notAfter,
      },
    }]),
    PORTAL_E2E_ADMIN_CONTROL_AGENT: JSON.stringify({origin: internalOrigin, ca, key, cert, publicKey, resultKey}),
  };
}

// JCS for the I-JSON admin-control envelopes, with ECMAScript primitive serialization.
function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`;
  if (value !== null && typeof value === 'object') {
    return `{${Object.keys(value).sort().map(k => `${JSON.stringify(k)}:${canonical(value[k])}`).join(',')}}`;
  }
  return JSON.stringify(value);
}

export function adminControlAgent() {
  const {origin, ca, key, cert, publicKey, resultKey} = JSON.parse(process.env.PORTAL_E2E_ADMIN_CONTROL_AGENT);
  const target = new URL(origin);
  if (target.protocol !== 'https:' || target.hostname !== '127.0.0.1') {
    throw new Error('Admin control fixture requires the disposable loopback TLS listener');
  }
  const tls = new https.Agent({
    ca: fs.readFileSync(ca), cert: fs.readFileSync(cert), key: fs.readFileSync(key),
    minVersion: 'TLSv1.3', maxVersion: 'TLSv1.3', servername: 'localhost',
    ALPNProtocols: ['http/1.1'],
  });
  const verifier = crypto.createPublicKey(fs.readFileSync(publicKey));
  const signer = crypto.createPrivateKey(fs.readFileSync(resultKey));
  const clock = () => new Date().toISOString();
  const epoch = Date.now();
  function post(endpoint, data) {
    return new Promise((resolve, reject) => {
      const body = JSON.stringify(data);
      const request = https.request(new URL(`/internal/v1/admin-control/${endpoint}`, target), {
        method: 'POST', agent: tls,
        headers: {'content-type': 'application/json', 'content-length': Buffer.byteLength(body)},
      }, response => {
        const chunks = [];
        response.on('data', chunk => chunks.push(chunk));
        response.on('error', reject);
        response.on('end', () => {
          if (response.statusCode !== 200) {
            reject(new Error(`Admin control ${endpoint} returned HTTP ${response.statusCode}`));
            return;
          }
          try {
            resolve(JSON.parse(Buffer.concat(chunks).toString('utf8')));
          } catch (error) {
            reject(error);
          }
        });
      });
      request.on('error', reject);
      request.setTimeout(15_000, () => request.destroy(new Error('Admin control fixture request timed out')));
      request.end(body);
    });
  }
  const register = (operations, owner_epoch) => post('hello', {
    agent_id: agentId, owner_epoch, build_revision: build,
    clock: clock(), catalog_digest: crypto.createHash('sha256').update(canonical(fixtures)).digest('base64url'),
    operations: operations.map(operation => ({operation, version: 1})),
    worlds: [worldId], nodes: [nodeId],
  });
  return {
    connect: () => register(advertisedOperations, epoch),
    heartbeat: () => post('heartbeat', {agent_id: agentId, owner_epoch: epoch, clock: clock()}),
    poll: () => post('channel', {agent_id: agentId, owner_epoch: epoch, wait_seconds: 0, limit: 32}),
    authentic(request) {
      const {signature, ...envelope} = request;
      return crypto.verify(null, Buffer.from(canonical(envelope)), verifier, Buffer.from(signature, 'base64url'));
    },
    complete(request, resultOverrides) {
      if (!Object.hasOwn(fixtures, request.operation)) throw new Error('Fixture operation has no result');
      const envelope = {
        request_id: request.request_id, admin_operation_id: request.admin_operation_id, status: 'succeeded',
        error: null, result: {...fixtures[request.operation], ...resultOverrides},
        dry_run_preview: null, game_ledger_id: null,
        audit_event_ids: [], effective_revision: 'verified-fixture-revision', progress: null,
        completed_at: clock(), agent_id: agentId, agent_build: build,
      };
      const signature = crypto.sign(null, Buffer.from(canonical(envelope)), signer).toString('base64url');
      return post('results', {...envelope, signature});
    },
    async close() {
      try {
        await register([], epoch + 1);
      } finally {
        tls.destroy();
      }
    },
  };
}
