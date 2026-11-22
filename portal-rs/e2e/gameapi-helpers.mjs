import { spawnSync } from 'node:child_process';
import { createHash, createHmac, randomBytes } from 'node:crypto';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect } from '@playwright/test';
import { fixtureDatabaseURL, pool } from './helpers.mjs';
import { uuid7 } from './game-sim-helpers.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const portal = path.resolve(here, '..');
export { callback, clientId, gateway, nativeTls, origin, packageId } from './gameapi-stack.mjs';

export function sha256(bytes) {
  return createHash('sha256').update(bytes);
}

function binary(args, extraEnv = {}) {
  const databaseURL = fixtureDatabaseURL();
  const result = spawnSync(process.env.PORTAL_GAMEAPI_REAL_BINARY, args, {
    cwd: portal,
    env: { ...process.env, ...extraEnv, OTERYN_DATABASE_URL: databaseURL },
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

// Keep the previous dedicated launcher usable for local diagnosis.
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await import('./local.mjs');
}
