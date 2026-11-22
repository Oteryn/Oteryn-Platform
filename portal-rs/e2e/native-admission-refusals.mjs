// Run against a live ./start.sh report whose seed plan includes an account step named player.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash, randomBytes } from 'node:crypto';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import http from 'node:http';
import https from 'node:https';
import { join, resolve } from 'node:path';
import { query } from './staff-rbac-helpers.mjs';

assert.equal(process.argv.length, 3, 'Pass the live harness report.json path');
const reportFile = resolve(process.argv[2]);
const report = JSON.parse(readFileSync(reportFile));
const privateDir = join(report.outputDirectory, 'secrets');
assert.equal(report.status, 'ready');
for (const base of Object.values(report.baseUrls)) {
  assert.equal(new URL(base).hostname, '127.0.0.1');
}
const fixture = JSON.parse(readFileSync(new URL(
  '../crates/gateway/tests/fixtures/game_client_native_login/request.json', import.meta.url,
)));
const account = report.seed.player.accountId;
const env = JSON.parse(readFileSync(join(privateDir, 'env.json')));
process.env.PORTAL_E2E_DATABASE_URL = env.OTERYN_DATABASE_URL;
const serviceToken = readFileSync(join(privateDir, 'gateway-token'), 'utf8').trim();
const admissionPath = '/internal/v1/game-auth/native-admissions';

function request(base, path, body, { token, ...transport } = {}) {
  const target = new URL(path, base);
  const client = target.protocol === 'https:' ? https : http;
  return new Promise((resolve, reject) => {
    const req = client.request(target, {
      ...transport,
      agent: false,
      method: 'POST',
      signal: AbortSignal.timeout(10_000),
      headers: {
        'content-type': 'application/json',
        'content-length': Buffer.byteLength(body),
        ...(token === undefined ? {} : { authorization: `Bearer ${token}` }),
      },
    }, res => {
      const chunks = [];
      res.on('data', chunk => chunks.push(chunk));
      res.on('error', reject);
      res.on('end', () => resolve([res.statusCode, Buffer.concat(chunks), res.headers['content-type']]));
    });
    req.on('error', reject);
    req.end(body);
  });
}

function assertResponse(result, status, body) {
  assert.equal(result[0], status);
  assert.ok(result[1].equals(Buffer.from(body)), 'unexpected refusal bytes');
}

const issuer = (body, token = serviceToken) =>
  request(report.baseUrls.nativeAdmission, admissionPath, body, { token });
assertResponse(await issuer('{}', 'synthetic-wrong-token'), 401, '{"error":"unauthorized_service"}');

const tls = {
  ca: readFileSync(report.caFile),
  servername: report.internalServerName,
  minVersion: 'TLSv1.3',
};
await assert.rejects(request(report.baseUrls.internal, admissionPath, '{}', tls),
  { code: 'ERR_SSL_TLSV13_ALERT_CERTIFICATE_REQUIRED' });
const untrustedDir = mkdtempSync(join(privateDir, 'untrusted-client-'));
try {
  const cert = join(untrustedDir, 'client.pem');
  const key = join(untrustedDir, 'client.key');
  execFileSync('openssl', [
    'req', '-x509', '-newkey', 'ec', '-pkeyopt', 'ec_paramgen_curve:P-256',
    '-nodes', '-keyout', key, '-out', cert, '-days', '1', '-subj', '/CN=Untrusted E2E Client',
    '-addext', 'basicConstraints=critical,CA:FALSE', '-addext', 'extendedKeyUsage=clientAuth',
  ], { stdio: 'ignore' });
  await assert.rejects(request(report.baseUrls.internal, admissionPath, '{}', {
    ...tls, cert: readFileSync(cert), key: readFileSync(key),
  }), { code: 'ERR_SSL_TLSV1_ALERT_UNKNOWN_CA' });
} finally {
  rmSync(untrustedDir, { recursive: true });
}
const unrelatedClient = report.clients['bootstrap-intents'];
const [internalStatus, , internalType] = await request(report.baseUrls.internal, admissionPath, '{}', {
  ...tls,
  cert: readFileSync(unrelatedClient.cert),
  key: readFileSync(unrelatedClient.key),
});
assert.equal(internalStatus, 404);
assert.equal(internalType, 'text/html; charset=utf-8');

const malformed = '{"protocol_version":2,"error":{"code":"NATIVE_LOGIN_REQUEST_MALFORMED","public_class":"RETRY_LOGIN"},"attempt_ref":null}';
for (const body of [Buffer.from('{]'), Buffer.from('{"protocol_version":2,'),
  Buffer.from([0x22, 0xff, 0x22]), Buffer.alloc(2049, 0x20)]) {
  assertResponse(await issuer(body), 400, malformed);
}

const unsupported = `{"protocol_version":2,"error":{"code":"NATIVE_LOGIN_UNSUPPORTED_VERSION","public_class":"CLIENT_UPDATE_REQUIRED"},"attempt_ref":"${fixture.attempt_ref}"}`;
assertResponse(await issuer(JSON.stringify({ ...fixture, protocol_version: 4 })), 400, unsupported);

const authentication = `{"protocol_version":2,"error":{"code":"NATIVE_LOGIN_AUTHENTICATION_REQUIRED","public_class":"AUTHENTICATION_REQUIRED"},"attempt_ref":"${fixture.attempt_ref}"}`;
for (const used of [false, true]) {
  // Set the two historical ticket states directly so the checks use the real
  // ticket authority without waiting for expiry or requiring a grant issuer.
  const ticket = randomBytes(32).toString('base64url');
  const hash = createHash('sha256').update(ticket).digest();
  const [created] = await query(`
    INSERT INTO game_login_tickets
      (ticket_hash, account_id, game_generation, native_generation, audience, expires_at, used_at, attempt_ref)
    SELECT $1, id, game_auth_generation::text, native_security_generation::text,
      'oteryn-native-game-gateway',
      clock_timestamp() + CASE WHEN $3 THEN interval '1 hour' ELSE interval '-1 hour' END,
      CASE WHEN $3 THEN clock_timestamp() ELSE NULL END,
      CASE WHEN $3 THEN uuidv7() ELSE NULL END
    FROM accounts WHERE id = $2
    RETURNING expires_at, used_at, attempt_ref
  `, [hash, account, used]);
  assert.ok(created, 'the seed CLI must create the player account');
  const body = JSON.stringify({ ...fixture, game_login_ticket: ticket });
  const [first, duplicate, gateway] = await Promise.all([
    issuer(body),
    issuer(body),
    request(report.baseUrls.gateway, '/v1/login', body),
  ]);
  assertResponse(first, 401, authentication);
  assertResponse(duplicate, 401, authentication);
  assertResponse(gateway, 401, authentication + '\n');
  assert.deepEqual(await query(
    'SELECT expires_at, used_at, attempt_ref FROM game_login_tickets WHERE ticket_hash = $1',
    [hash],
  ), [created]);
  const [rows] = await query(
    'SELECT count(*)::int AS count FROM native_admissions WHERE ticket_hash = $1', [hash],
  );
  assert.equal(rows.count, 0);
}
console.log('Native admission refusals and gateway authentication mapping passed');
