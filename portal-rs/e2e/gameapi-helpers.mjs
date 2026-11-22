import { spawn, spawnSync } from 'node:child_process';
import { createHash, createHmac, randomBytes } from 'node:crypto';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect } from '@playwright/test';
import { pool } from './helpers.mjs';
import { characterSourceAuthority } from './game-closure-producer.mjs';
import { uuid7 } from './game-sim-helpers.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const portal = path.resolve(here, '..');
export const clientId = '019a0000-0000-7000-8000-0000000c11e7';
export const origin = 'https://localhost:18443';
export const gateway = 'http://127.0.0.1:18090';
export const nativeTls = 'https://localhost:18447';
export const callback = origin + '/__oauth/callback';
export const packageId = '019a0000-0000-7000-8000-000000000123';

export function sha256(bytes) {
  return createHash('sha256').update(bytes);
}

function binary(args, extraEnv = {}) {
  // pool() checks that the database belongs to the disposable loopback harness.
  const db = pool();
  void db.end();
  const result = spawnSync(process.env.PORTAL_GAMEAPI_REAL_BINARY, args, {
    cwd: portal,
    env: { ...process.env, ...extraEnv },
    encoding: 'utf8',
    timeout: 30000,
  });
  if (result.status !== 0) throw new Error(`${args[0]} ${args[1] ?? ''} failed (exit ${result.status})`);
  return result.stdout;
}

export function seed(entity, args) {
  return JSON.parse(binary(['seed', entity, JSON.stringify(args)]));
}

export function seedPlayer(locale) {
  const email = `gameapi-${uuid7()}@example.invalid`;
  const password = randomBytes(24).toString('base64url') + 'Aa9!';
  const { accountId } = seed('account', { email, password, locale });
  return { email, password, accountId };
}

export function fixture(relative) {
  return readFileSync(path.join(portal, 'crates', relative));
}

export async function publishDownloads() {
  const db = pool();
  try {
    const published = await db.query('SELECT name FROM client_channels WHERE active_publication_id IS NOT NULL');
    const active = new Set(published.rows.map(row => row.name));
    const missing = ['stable', 'beta'].filter(channel => !active.has(channel));
    if (missing.length === 0) return;

    const publisher = seedPlayer('en');
    const grants = await db.query(
      "INSERT INTO account_roles(account_id,role_id) SELECT $1,role_id FROM role_permissions WHERE permission='downloads.manage' ORDER BY role_id LIMIT 1",
      [publisher.accountId],
    );
    expect(grants.rowCount).toBe(1);
    for (const channel of missing) {
      const dir = path.join(portal, 'crates/downloads/tests/fixtures/origin-v1', channel);
      binary(['downloads-publish', '--channel', channel, '--actor', publisher.accountId, '--dir', dir], {
        [`OTERYN_DOWNLOADS_ROOT_${channel.toUpperCase()}`]: path.join(dir, 'metadata/1.root.json'),
      });
    }
  } finally {
    await db.end();
  }
}

export async function waitForPortal(request) {
  await expect.poll(async () => {
    try {
      return (await request.get('/healthz', { timeout: 2000 })).status();
    } catch {
      return 0;
    }
  }, { timeout: 30000 }).toBe(200);
}

export function requestSource(info, lane) {
  const locale = info.project.metadata.locale;
  const project = ['pl-1440', 'pl-390', 'en-1440', 'en-390'].indexOf(info.project.name) + 1;
  return { 'x-forwarded-for': `127.0.${lane}.${project}`, 'accept-language': locale };
}

export async function json(response, status, body) {
  expect(response.status(), new URL(response.url()).pathname).toBe(status);
  expect(response.headers()['content-type']).toContain('application/json');
  expect(await response.json()).toEqual(body);
}

export async function empty(response, status, cache) {
  expect(response.status(), new URL(response.url()).pathname).toBe(status);
  expect(await response.text()).toBe('');
  if (cache) expect(response.headers()['cache-control']).toBe(cache);
}

export function bearer(token) {
  return { authorization: `Bearer ${token}` };
}

export function providerEvent(request, body, offset = 0) {
  const time = String(Math.floor(Date.now() / 1000) + offset);
  const signature = createHmac('sha256', process.env.PORTAL_PAYMENTS_PROVIDER_WEBHOOK_SECRET)
    .update(time + '.' + body).digest('hex');
  return request.post('/api/v1/payments/provider-webhook', {
    data: body,
    headers: {
      'content-type': 'application/json',
      'oteryn-webhook-timestamp': time,
      'oteryn-webhook-signature': 'v1=' + signature,
    },
  });
}

// From e2e/ with the local database administrator URL and a built binary:
// node gameapi-helpers.mjs
// node run.mjs <scratch>/env.json test 'gameapi-.*\.spec\.mjs' --retries=0
// Run with PORTAL_E2E_ADMIN_DATABASE_URL and a built portal binary, as in README.md.
// local.mjs owns the browser, mail, database, and documented Game producers.
function start() {
  const executable = path.resolve(process.env.PORTAL_E2E_BINARY ?? path.join(portal, 'target/debug/oteryn-portal'));
  const scratch = mkdtempSync(path.join(tmpdir(), 'portal-gameapi-'));
  const wrapper = path.join(scratch, 'portal');
  writeFileSync(wrapper, '#!/bin/sh\nexport PORTAL_OAUTH_CONFIG="$PORTAL_GAMEAPI_OAUTH_CONFIG"\nexec "$PORTAL_GAMEAPI_REAL_BINARY" "$@"\n', { mode: 0o700 });
  const token = randomBytes(32).toString('base64url');
  const tokenFile = path.join(scratch, 'gateway-token');
  const digestFile = path.join(scratch, 'gateway-token.sha256');
  writeFileSync(tokenFile, token, { mode: 0o600 });
  writeFileSync(digestFile, sha256(token).digest('hex'), { mode: 0o600 });
  const env = {
    ...process.env,
    PORTAL_E2E_SCRATCH: scratch,
    PORTAL_E2E_BINARY: wrapper,
    PORTAL_GAMEAPI_REAL_BINARY: executable,
    PORTAL_GAMEAPI_OAUTH_CONFIG: JSON.stringify({
      enabled: true,
      clients: [clientId, 'e2e-local'].map(client_id => ({ client_id, redirect_uris: [callback] })),
    }),
    OTERYN_NATIVE_TLS_BIND: '127.0.0.1:18447',
    OTERYN_NATIVE_TLS_CERT_FILE: path.join(scratch, 'cert.pem'),
    OTERYN_NATIVE_TLS_KEY_FILE: path.join(scratch, 'key.pem'),
    OTERYN_NATIVE_DEVICE_PROJECTION_AUTHORITY: characterSourceAuthority,
    PORTAL_PAYMENTS_PROVIDER_WEBHOOK_SECRET: randomBytes(32).toString('base64url'),
    PORTAL_NATIVE_ADMISSION_BIND: '127.0.0.1:18091',
    PORTAL_GATEWAY_TOKEN_SHA256_FILE: digestFile,
    PORTAL_COIN_PACKAGES: JSON.stringify([{
      id: packageId, title: 'E2E coin package', coins: 500,
      price: { minor_units: 499, currency: 'PLN', exponent: 2 }, valid_for_seconds: 300,
    }]),
    TRUSTED_PROXIES: '127.0.0.1',
  };
  const local = spawn(process.execPath, [path.join(here, 'local.mjs')], { env, stdio: 'inherit' });
  const edge = spawn(executable, ['gateway'], {
    env: {
      ...process.env,
      GATEWAY_LISTEN_ADDR: new URL(gateway).host,
      GATEWAY_TRUSTED_PROXIES: '127.0.0.1',
      GATEWAY_NATIVE_LOGIN_ENABLED: 'true',
      GATEWAY_NATIVE_PROTOCOL3_ENABLED: 'false',
      GATEWAY_VERSION: 'e2e-gameapi',
      OTERYN_PLATFORM_BASE_URL: 'http://127.0.0.1:18091',
      OTERYN_PLATFORM_SERVICE_TOKEN_FILE: tokenFile,
    },
    stdio: 'inherit',
  });
  for (const signal of ['SIGINT', 'SIGTERM']) {
    process.on(signal, () => { local.kill(signal); edge.kill(signal); });
  }
  for (const [child, other] of [[local, edge], [edge, local]]) {
    child.on('exit', code => {
      other.kill('SIGTERM');
      process.exitCode ||= code ?? 0;
    });
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) start();
