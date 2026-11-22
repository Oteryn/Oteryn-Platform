import { mirrorHead } from './head.mjs';
import { readFileSync } from 'node:fs';
import https from 'node:https';
import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';
import { actorDefinitions, csrfPage, openActor, sessionHash } from './actors.mjs';
import { assertRefusalState, database } from './database.mjs';

const { routes, roles } = JSON.parse(readFileSync(new URL('./routes.json', import.meta.url), 'utf8'));
const methods = routes.filter(row => row.path === '/native-account/coin-checkout/{order_id}')
  .flatMap(row => row.registrations.filter(registration => registration.profile === 'unavailable')
    .map(registration => ({ method: row.method, ...registration })));
const path = '/native-account/coin-checkout/019a541b-a000-7000-8000-000000000001';
const bazaarCases = [
  ['POST', '/{locale}/bazaar/items'],
  ['GET', '/{locale}/bazaar/items/{id}'],
  ['POST', '/{locale}/bazaar/items/{id}/{action}'],
];

async function connectStatus(request, token, origin) {
  // The browser launcher HTTP proxy reserves CONNECT for tunneling. The native
  // TLS listener serves the same public application directly.
  const native = new URL(path, process.env.PORTAL_E2E_BROWSER_NATIVE_ORIGIN || 'https://127.0.0.1:18444');
  expect(native.protocol === 'https:' && native.hostname === '127.0.0.1').toBe(true);
  const { cookies } = await request.storageState();
  const session = cookies.find(({ name }) => name === '__Host-oteryn_session');
  expect(Boolean(session), 'checkout actor session exists').toBe(true);
  return new Promise((resolve, reject) => {
    const req = https.request(native, {
      method: 'CONNECT', servername: 'localhost', ca: readFileSync(process.env.SSL_CERT_FILE),
      minVersion: 'TLSv1.3', headers: { origin, cookie: `${session.name}=${session.value}`, 'x-csrf-token': token },
    });
    req.on('connect', (response, socket) => {
      socket.destroy();
      resolve(response.statusCode);
    });
    req.on('response', response => {
      response.resume();
      response.once('end', () => resolve(response.statusCode));
      response.once('error', reject);
    });
    req.once('error', reject);
    req.end();
  });
}

test.describe('unconfigured checkout and marketplace authority registrations', () => {
  for (const definition of actorDefinitions(roles)) {
    test(definition.key, async ({}, info) => {
      expect(methods.map(({ method }) => method).sort()).toEqual(
        ['CONNECT', 'DELETE', 'GET', 'HEAD', 'OPTIONS', 'PATCH', 'POST', 'PUT', 'TRACE']);
      const { locale } = info.project.metadata;
      const issuer = definition.restricted
        ? (await query("SELECT id FROM accounts WHERE email='bazaar-escrow@local.test'"))[0].id
        : undefined;
      const { request, accountId } = await openActor(locale, definition, issuer);
      const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
      const db = database();
      try {
        const token = await csrfPage(request, accountId ? `/${locale}/` : `/${locale}/login`);
        const hash = await sessionHash(request);
        for (const { method, guard, source } of methods) {
          expect(guard.kind).toBe('anonymous');
          await test.step(`${method} /native-account/coin-checkout/{order_id}`, async () => {
            const before = await db.snapshot(hash);
            if (method === 'CONNECT') {
              expect(await connectStatus(request, token, origin), source).toBe(503);
            } else {
              const response = await request.fetch(path, {
                method, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
                ...(method === 'POST' && { form: { action: 'fail', _csrf: token } }),
              });
              try {
                expect(response.status(), source).toBe(503);
                if (method === 'HEAD') expect((await response.body()).length).toBe(0);
              } finally {
                await response.dispose();
              }
            }
            assertRefusalState(before, await db.snapshot(hash));
          });
        }
        for (const [method, pattern] of bazaarCases) {
          const route = routes.find(r => r.listener === 'public' && r.method === method && r.path === pattern);
          const { guard, source } = route.registrations[0];
          expect(guard.kind, source).toBe('anonymous');
          const target = pattern.replace('{locale}', locale).replace('{id}', path.split('/').at(-1))
            .replace('{action}', 'buy');
          await test.step(`${method} ${pattern}`, async () => {
            const response = await request.fetch(target, {
              method, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
              ...(method === 'POST' && { form: { _csrf: token } }),
            });
            try {
              expect(response.status(), 'crates/bazaar/src/items/web.rs:83').toBe(503);
              if (method === 'GET') await mirrorHead(request, pattern, target, response);
            } finally {
              await response.dispose();
            }
          });
        }
      } finally {
        await db.close();
        await request.dispose();
      }
    });
  }
});
