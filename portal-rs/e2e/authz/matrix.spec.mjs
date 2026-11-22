import { randomUUID } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import https from 'node:https';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { expect, test } from '@playwright/test';
import { actorDefinitions, csrfPage, openActor, sessionHash as hashSession } from './actors.mjs';
import { assertRefusalState, database, privateDatabase } from './database.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { waitForWikiBudget } from '../helpers.mjs';

const { routes, roles } = JSON.parse(readFileSync(new URL('./routes.json', import.meta.url), 'utf8'));

function registration(path, method) {
  const matches = routes.filter(route => route.listener === 'public' && route.path === path && route.method === method);
  if (matches.length !== 1 || matches[0].registrations.length !== 1) {
    throw new Error(`Expected one public registration for ${method} ${path}`);
  }
  return matches[0];
}

test('route sample GET /healthz', async ({ request }) => {
  const { path, registrations: [{ guard }] } = registration('/healthz', 'GET');
  expect(guard.kind).toBe('anonymous');
  const response = await request.get(path, { maxRedirects: 0 });
  expect(response.status()).toBe(200);
  expect(await response.text()).toBe('ok');
});

const privateRoutes = routes.filter(route => route.registrations.some(row => row.refusal)
  && route.listener === 'admission');
const isolatedIngressRoutes = routes.filter(({ listener }) => ['bazaar', 'closure', 'league'].includes(listener));
const listenerHealth = routes.filter(({ listener, method }) =>
  ['gateway', 'admission'].includes(listener) && ['GET', 'HEAD'].includes(method));
const browserRoutes = [
  ...[
    '/admin', '/admin/roles', '/admin/announcements', '/admin/announcements/create',
    '/admin/console/access', '/admin/console/approvals', '/admin/events', '/admin/events/create',
    '/admin/media', '/admin/moderation/enforcement', '/admin/moderation/enforcement/create',
    '/admin/moderation/reports', '/admin/news', '/admin/news/create', '/admin/pages',
    '/admin/players', '/admin/support/tickets', '/admin/wiki', '/admin/wiki/articles',
    '/admin/wiki/articles/create',
  ].flatMap(path => ['GET', 'HEAD'].map(method => registration(path, method))),
  registration('/__test/payments/webhook', 'POST'),
  registration('/admin/roles', 'POST'),
];

const readFixtures = JSON.parse(readFileSync(new URL('./public-reads.json', import.meta.url), 'utf8'));
const readKeys = new Set(readFixtures.map(({ method, path }) => `${method} ${path}`));
const publicReads = readFixtures.flatMap(row => {
  const head = routes.find(route => route.listener === 'public' && route.path === row.path && route.method === 'HEAD');
  return row.method === 'GET' && head && !readKeys.has(`HEAD ${row.path}`)
    ? [row, { ...row, method: 'HEAD' }] : [row];
});
for (const { method, path } of publicReads) {
  const route = routes.find(row => row.listener === 'public' && row.path === path && row.method === method);
  if (!route || route.registrations[0].guard.kind !== 'anonymous') {
    throw new Error(`Public read fixture has no matching public registration for ${method} ${path}`);
  }
}

async function assertPublicReads(request, locale, anonymous) {
  for (const row of publicReads) {
    const url = row.target.replaceAll('{lang}', locale).replaceAll('{locale}', locale);
    await test.step(`${row.method} ${row.path}`, async () => {
      await waitForWikiBudget(url);
      const response = await request.fetch(url, { method: row.method, maxRedirects: 0 });
      try {
        const status = anonymous ? row.status : row.signedInStatus ?? row.status;
        if (row.method === 'HEAD') expect((await response.body()).length).toBe(0);
        expect(response.status(), `${row.method} ${url} (${row.source})`).toBe(status);
        if (!anonymous && row.signedInLocation) {
          expect(response.headers().location).toBe(row.signedInLocation);
        }
        if (row.pendingChallenge) {
          const lang = /^\/(en|pl)\//.exec(url)?.[1] ?? locale;
          expect(response.headers().location).toBe(anonymous ? `/${lang}/login` : `/${lang}/`);
        }
      } finally {
        await response.dispose();
      }
    });
  }
}

test.describe('browser authorization', () => {
  for (const definition of actorDefinitions(roles)) {
    test(definition.key, async ({}, info) => {
      const anonymous = definition.key === 'anonymous';
      const issuer = definition.restricted
        ? (await query("SELECT id FROM accounts WHERE email='bazaar-escrow@local.test'"))[0].id
        : undefined;
      const { request } = await openActor(info.project.metadata.locale, definition, issuer);
      const db = database();
      try {
        await query('UPDATE registration_abuse_policy SET enabled=true WHERE singleton AND NOT enabled');
        const entry = anonymous ? 'login' : '';
        const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
        const token = await csrfPage(request, `/${info.project.metadata.locale}/${entry}`);
        const sessionHash = await hashSession(request);

        for (const { path, method, registrations: [{ guard, refusal }] } of browserRoutes) {
          await test.step(`${method} ${path}`, async () => {
            const mutation = method === 'POST';
            const allowed = guard.kind === 'staff' && guard.all.every(p => definition.permissions.includes(p));
            const before = mutation && !allowed ? await db.snapshot(sessionHash) : null;
            const key = `matrix_${randomUUID().replaceAll('-', '')}`;
            const response = await request.fetch(path, {
              method,
              headers: { 'x-csrf-token': token, origin: origin },
              ...(mutation && (refusal ? { data: {} } : { form: { action: 'create', key, permissions: '' } })),
              maxRedirects: 0,
            });
            const status = refusal?.status ?? (allowed ? (mutation ? 303 : 200) : (anonymous ? 303 : 403));
            expect(response.status(), refusal?.source ?? `${method} ${path}`).toBe(status);
            if (method === 'HEAD') expect((await response.body()).length).toBe(0);
            if (status === 303) expect(response.headers().location).toBe(allowed ? path : '/login');
            if (before) {
              const denialActions = guard.kind === 'staff' && definition.role ? ['admin.access.denied'] : [];
              assertRefusalState(before, await db.snapshot(sessionHash), denialActions);
            }
            if (allowed && mutation) {
              const created = await query('SELECT count(*)::int AS count FROM roles WHERE key=$1', [key]);
              expect(created[0].count, 'role creation persisted').toBe(1);
            }
            await response.dispose();
          });
        }
        await assertPublicReads(request, info.project.metadata.locale, anonymous);
        const harness = readHarness();
        await assertPrivateRoutes(request, harness);
        for (const route of listenerHealth) {
          await test.step(`${route.method} ${route.path} on ${route.listener}`, async () => {
            const base = route.listener === 'gateway' ? harness.baseUrls.gateway : harness.baseUrls.nativeAdmission;
            const target = new URL(route.path, base);
            expect(target.hostname === '127.0.0.1' && target.protocol === 'http:').toBe(true);
            const response = await request.fetch(target.href, { method: route.method, maxRedirects: 0 });
            expect(response.status(), route.registrations[0].source).toBe(200);
            if (route.method === 'HEAD') expect((await response.body()).length).toBe(0);
            await response.dispose();
          });
        }
      } finally {
        await db.close();
        await request.dispose();
      }
    });
  }
});

function readHarness() {
  return JSON.parse(readFileSync(process.env.PORTAL_E2E_HARNESS_REPORT, 'utf8'));
}

async function assertPrivateRoutes(request, harness) {
  const db = privateDatabase(process.env.PORTAL_E2E_HARNESS_REPORT, harness);
  try {
    for (const route of privateRoutes) {
      await test.step(`${route.method} ${route.path} on ${route.listener}`, async () => {
        const before = await db.snapshot();
        await assertServiceTokenRefused(request, harness, route);
        assertRefusalState(before, await db.snapshot());
      });
    }
  } finally {
    await db.close();
  }
}

function internalRequest(harness, route, client) {
  const path = route.path.replace(/\{[^{}]+\}/g, '019a541b-a000-7000-8000-000000000001');
  const target = new URL(path, harness.baseUrls.internal);
  if (target.protocol !== 'https:' || target.hostname !== '127.0.0.1') {
    throw new Error('The authorization matrix requires a loopback mTLS listener');
  }
  return new Promise((resolve, reject) => {
    const req = https.request(target, {
      method: route.method,
      servername: harness.internalServerName,
      ca: readFileSync(harness.caFile),
      ...(client && { cert: readFileSync(client.cert), key: readFileSync(client.key) }),
      minVersion: 'TLSv1.3',
      ALPNProtocols: ['http/1.1'],
      headers: ['GET', 'HEAD'].includes(route.method) ? {} : { 'content-type': 'application/json', 'content-length': 2 },
    }, response => {
      response.resume();
      response.once('end', () => resolve(response.statusCode));
      response.once('error', reject);
    });
    req.once('error', reject);
    req.setTimeout(5_000, () => req.destroy(new Error('mTLS request timed out')));
    req.end(['GET', 'HEAD'].includes(route.method) ? undefined : '{}');
  });
}

async function assertServiceTokenRefused(request, harness, route) {
  const target = new URL(route.path, harness.baseUrls.nativeAdmission);
  expect(target.hostname === '127.0.0.1' && target.protocol === 'http:').toBe(true);
  const response = await request.fetch(target.href, {
    method: route.method,
    data: {},
    maxRedirects: 0,
  });
  const { refusal } = route.registrations[0];
  expect(response.status(), refusal.source).toBe(refusal.status);
  await response.dispose();
}

const privateCases = JSON.parse(readFileSync(new URL('./private-http.json', import.meta.url), 'utf8'));

async function assertAdminControlPeers(harness, wrongPeer, db) {
  const directory = mkdtempSync(join(tmpdir(), 'portal-e2e-authz-agent-'));
  try {
    const key = join(directory, 'agent.key');
    const cert = join(directory, 'agent.pem');
    const request = join(directory, 'agent.csr');
    // Match the disposable agent binding in the browser launcher.
    execFileSync('openssl', ['req', '-new', '-newkey', 'ec', '-pkeyopt', 'ec_paramgen_curve:P-256',
      '-nodes', '-keyout', key, '-out', request, '-subj', '/CN=oteryn-admin-agent\\/e2e-local',
      '-addext', 'subjectAltName=DNS:spiffe://local.test/admin/e2e',
      '-addext', 'extendedKeyUsage=clientAuth'], { stdio: 'ignore' });
    execFileSync('openssl', ['x509', '-req', '-in', request, '-CA', harness.caFile,
      '-CAkey', join(dirname(harness.caFile), 'ca-key.pem'), '-days', '1',
      '-set_serial', `0x${randomUUID().replaceAll('-', '')}`,
      '-copy_extensions', 'copy', '-out', cert], { stdio: 'ignore' });
    for (const row of privateCases.filter(({ path }) => path.startsWith('/internal/v1/admin-control/'))) {
      await test.step(`${row.method} ${row.path} with a CA-valid agent peer`, async () => {
        const before = await db.snapshot();
        expect(await internalRequest(harness, row, wrongPeer), row.limitSource).toBe(403);
        assertRefusalState(before, await db.snapshot());
        const expected = row.path.endsWith('/{endpoint}') ? 404 : 400;
        expect(await internalRequest(harness, row, { key, cert }), row.source).toBe(expected);
      });
    }
  } finally {
    rmSync(directory, { recursive: true });
  }
}

test('private ingress reaches the route over a CA-valid connection', async () => {
  const harness = readHarness();
  const configured = routes.filter(route => route.listener === 'internal');
  const signature = ({ method, path }) => `${method} ${path}`;
  expect(privateCases.map(signature).sort()).toEqual(configured.map(signature).sort());

  // Transport admission is independent of the HTTP path or method.
  const first = configured.find(route => route.path.endsWith('/native-scope-assignments'));
  await expect(internalRequest(harness, first)).rejects.toMatchObject({
    code: 'ERR_SSL_TLSV13_ALERT_CERTIFICATE_REQUIRED',
  });

  const db = privateDatabase(process.env.PORTAL_E2E_HARNESS_REPORT, harness);
  const browserDb = database();
  try {
    for (const row of privateCases) {
      await test.step(`${signature(row)} on internal with a CA-valid peer`, async () => {
        const before = await db.snapshot();
        const peer = harness.clients[row.peer];
        expect(Boolean(peer), 'peer exists in the harness report').toBe(true);
        expect(await internalRequest(harness, row, peer), row.source).toBe(row.status);
        assertRefusalState(before, await db.snapshot());
        if (row.authorizedPeer) {
          const authorized = harness.clients[row.authorizedPeer];
          expect(Boolean(authorized), 'authorized peer exists in the harness report').toBe(true);
          expect(await internalRequest(harness, row, authorized), row.source).toBe(row.authorizedInvalidBodyStatus);
        }
      });
    }
    const caFile = process.env.SSL_CERT_FILE;
    const root = dirname(caFile);
    const browser = {
      baseUrls: { internal: process.env.PORTAL_E2E_BROWSER_PRIVATE_ORIGIN || 'https://127.0.0.1:18447' },
      internalServerName: 'localhost',
      caFile,
    };
    const portfolio = { cert: join(root, 'closure-portfolio.pem'), key: join(root, 'closure-portfolio-key.pem') };
    const character = { cert: join(root, 'closure-character.pem'), key: join(root, 'closure-character-key.pem') };
    const bazaar = { cert: join(root, 'bazaar-game/bazaar-cert.pem'), key: join(root, 'bazaar-game/bazaar-key.pem') };
    const league = { cert: join(root, 'league/league-game.pem'), key: join(root, 'league/league-game.key') };
    const expectedPeer = { bazaar, closure: character, league };

    await expect(internalRequest(browser, isolatedIngressRoutes[0])).rejects.toMatchObject({
      code: 'ERR_SSL_TLSV13_ALERT_CERTIFICATE_REQUIRED',
    });
    await assertAdminControlPeers(browser, league, browserDb);
    const browserBazaar = configured.filter(row => row.path.startsWith('/internal/v1/bazaar/'))
      .map(row => ({ ...row, listener: 'bazaar' }));
    for (const row of [...isolatedIngressRoutes, ...browserBazaar]) {
      await test.step(`${signature(row)} on ${row.listener} with a CA-valid peer`, async () => {
        const before = await browserDb.snapshot();
        expect(await internalRequest(browser, row, portfolio), row.registrations[0].source).toBe(401);
        assertRefusalState(before, await browserDb.snapshot());
        expect(await internalRequest(browser, row, expectedPeer[row.listener]), row.registrations[0].source)
          .toBe(400);
      });
    }
  } finally {
    await Promise.all([db.close(), browserDb.close()]);
  }
});
