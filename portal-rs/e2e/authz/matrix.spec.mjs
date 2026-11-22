import { randomUUID } from 'node:crypto';
import { readFileSync } from 'node:fs';
import https from 'node:https';
import { expect, test } from '@playwright/test';
import { actorDefinitions, csrfPage, openActor, sessionHash as hashSession } from './actors.mjs';
import { assertRefusalState, database } from './database.mjs';
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
  && ['internal', 'admission'].includes(route.listener));
const isolatedIngressRoutes = routes.filter(({ listener }) => ['bazaar', 'closure', 'league'].includes(listener));
const listenerHealth = routes.filter(({ listener, method }) =>
  ['gateway', 'admission'].includes(listener) && method === 'GET');
const browserRoutes = [
  ...[
    '/admin', '/admin/roles', '/admin/announcements', '/admin/announcements/create',
    '/admin/console/access', '/admin/console/approvals', '/admin/events', '/admin/events/create',
    '/admin/media', '/admin/moderation/enforcement', '/admin/moderation/enforcement/create',
    '/admin/moderation/reports', '/admin/news', '/admin/news/create', '/admin/pages',
    '/admin/players', '/admin/support/tickets', '/admin/wiki', '/admin/wiki/articles',
    '/admin/wiki/articles/create',
  ].map(path => registration(path, 'GET')),
  registration('/__test/payments/webhook', 'POST'),
  registration('/admin/roles', 'POST'),
];

const publicReads = JSON.parse(readFileSync(new URL('./public-reads.json', import.meta.url), 'utf8'));
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
        for (const route of [...privateRoutes, ...isolatedIngressRoutes]) {
          await test.step(`${route.method} ${route.path} on ${route.listener}`, async () => {
            const refusal = route.registrations[0].refusal;
            if (route.listener !== 'admission') {
              const internal = route.listener === 'internal';
              const ingress = internal ? harness : {
                baseUrls: { internal: process.env.PORTAL_E2E_BROWSER_PRIVATE_ORIGIN || 'https://127.0.0.1:18447' },
                internalServerName: 'localhost',
                caFile: process.env.SSL_CERT_FILE,
              };
              if (internal) expect(refusal.tlsAlert, refusal.source).toBe('certificate_required');
              await expect(internalRequest(ingress, route)).rejects.toMatchObject({
                code: 'ERR_SSL_TLSV13_ALERT_CERTIFICATE_REQUIRED',
              });
              if (internal && refusal.wrongIdentity) {
                expect(await internalRequest(ingress, route, harness.clients['bootstrap-intents']),
                  refusal.wrongIdentity.source).toBe(refusal.wrongIdentity.status);
              }
            } else {
              await assertServiceTokenRefused(request, harness, route);
            }
          });
        }
        for (const route of listenerHealth) {
          await test.step(`${route.method} ${route.path} on ${route.listener}`, async () => {
            const base = route.listener === 'gateway' ? harness.baseUrls.gateway : harness.baseUrls.nativeAdmission;
            const target = new URL(route.path, base);
            expect(target.hostname === '127.0.0.1' && target.protocol === 'http:').toBe(true);
            const response = await request.get(target.href, { maxRedirects: 0 });
            expect(response.status(), route.registrations[0].source).toBe(200);
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
      headers: route.method === 'GET' ? {} : { 'content-type': 'application/json', 'content-length': 2 },
    }, response => {
      response.resume();
      response.once('end', () => resolve(response.statusCode));
      response.once('error', reject);
    });
    req.once('error', reject);
    req.setTimeout(5_000, () => req.destroy(new Error('mTLS request timed out')));
    req.end(route.method === 'GET' ? undefined : '{}');
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

test('private ingress refuses missing credentials', async ({ request }) => {
  const harness = readHarness();
  const route = routes.find(row =>
    row.listener === 'internal' && row.path.endsWith('/native-scope-assignments') && row.method === 'POST');
  expect(route.registrations[0].guard.kind).toBe('mtls');
  await expect(internalRequest(harness, route)).rejects.toMatchObject({
    code: 'ERR_SSL_TLSV13_ALERT_CERTIFICATE_REQUIRED',
  });
  const peerRefusal = route.registrations[0].refusal.wrongIdentity;
  expect(await internalRequest(harness, route, harness.clients['bootstrap-intents']), peerRefusal.source)
    .toBe(peerRefusal.status);

  const admission = routes.find(row =>
    row.listener === 'admission' && row.path.endsWith('/native-admissions') && row.method === 'POST');
  expect(admission.registrations[0].guard.kind).toBe('service-token');
  await assertServiceTokenRefused(request, harness, admission);
});
