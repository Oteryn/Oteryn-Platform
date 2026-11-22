import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';
import { actorDefinitions, csrfPage, openActor } from './actors.mjs';
import { assertRefusalState, database } from './database.mjs';
import { editorialRoutes, exerciseEditorial } from './editorial.mjs';

const { routes, roles } = JSON.parse(readFileSync(new URL('./routes.json', import.meta.url), 'utf8'));
const fixtures = JSON.parse(readFileSync(new URL('./protected-reads.json', import.meta.url), 'utf8'));
const publicRoutes = new Map(routes.filter(r => r.listener === 'public').map(r => [`${r.method} ${r.path}`, r]));
const reads = fixtures.map(fixture => {
  const route = publicRoutes.get(`GET ${fixture.path}`);
  if (!route) throw new Error(`No public registration for GET ${fixture.path}`);
  return { ...fixture, guard: route.registrations[0].guard };
});

function fixturePath(target, locale) {
  return target.replaceAll(/\{(?:language|lang|locale)\}/g, locale);
}

function expectedRead(fixture, definition) {
  if (definition.key === 'anonymous') return fixture.anonymousStatus;
  if (definition.role && fixture.mfaStatus) return fixture.mfaStatus;
  const { guard } = fixture;
  if (guard.kind !== 'staff') return fixture.status;
  const allowed = guard.all.every(permission => definition.permissions.includes(permission))
    && (!guard.any || guard.any.some(permission => definition.permissions.includes(permission)));
  return allowed ? fixture.status : fixture.deniedStatus;
}

test.describe('protected reads and identity writes', () => {
  for (const definition of actorDefinitions(roles)) {
    test(definition.key, async ({ browser }, info) => {
      const { locale } = info.project.metadata;
      const anonymous = definition.key === 'anonymous';
      const issuer = definition.restricted
        ? (await query("SELECT id FROM accounts WHERE email='bazaar-escrow@local.test'"))[0].id
        : undefined;
      const { context, request, account, accountId } = await openActor(browser, info, definition, issuer);
      try {
        for (const fixture of reads) {
          const target = fixturePath(fixture.target, locale);
          await test.step(`GET ${fixture.path}`, async () => {
            const response = await request.get(target, { maxRedirects: 0 });
            try {
              const status = expectedRead(fixture, definition);
              expect.soft(response.status(), `GET ${target} (${fixture.source})`).toBe(status);
              if (status === 302 || status === 303) {
                const location = response.headers().location;
                if (anonymous) expect.soft(location, target).toMatch(/^\/(?:pl\/|en\/)?login$/);
                else if (fixture.signedInLocationSuffix) {
                  expect.soft(location?.endsWith(fixture.signedInLocationSuffix), target).toBe(true);
                }
              }
            } finally {
              await response.dispose();
            }
          });
        }
        await assertIdentityWrites(context, request, locale, account);
        await assertExport(request, locale, accountId);
        await exerciseEditorial(context, request, locale, definition, publicRoutes);
      } finally {
        await context.close();
      }
    });
  }
});

const idFixture = '019a541b-a000-7000-8000-000000000001';
const atlasBody = { x: 1, y: 1, floor: 7, text: 'Local authorization map fixture' };
const identityWrites = [
  { method: 'POST', path: '/{lang}/account/consents', status: 303, refusal: 303,
    source: 'crates/identity/src/consents_http.rs:64',
    form: () => ({ kind: 'play_session', version: '2026-11-11.1', granted: 'true' }) },
  { method: 'POST', path: '/{lang}/atlas/places/participation', status: 303, refusal: 403,
    source: 'crates/playerdata/src/places/http.rs:187',
    form: () => ({ participating: 'true' }) },
  { method: 'POST', path: '/atlas/private-markers', status: 201, refusal: 401,
    source: 'crates/atlas/src/spawns.rs:234', json: atlasBody, captureMarker: true },
  { method: 'DELETE', path: '/atlas/private-markers/{id}', status: 204, refusal: 401,
    source: 'crates/atlas/src/spawns.rs:270', json: {} },
  { method: 'POST', path: '/atlas/map-reports', status: 201, refusal: 401,
    source: 'crates/atlas/src/spawns.rs:288', json: atlasBody },
  ...['POST', 'DELETE'].map(method => ({
    method, path: '/api/v1/stats/watches', refusal: 401, bearer: true,
    source: 'crates/stats/src/watch/http.rs:70', json: {},
  })),
  { method: 'POST', path: '/{lang}/account/privacy', status: 303, refusal: 403,
    source: 'crates/identity/src/privacy_web.rs:187',
    form: ({ locale }) => ({ locale, public_account_association: '1', public_status_visible: '0' }) },
  { method: 'PUT', path: '/{lang}/account/privacy', status: 303, refusal: 403,
    source: 'crates/identity/src/privacy_web.rs:187',
    form: ({ locale }) => ({ locale, public_account_association: '0', public_status_visible: '1' }) },
  { method: 'POST', path: '/{lang}/account/notifications', status: 303, refusal: 403,
    source: 'crates/identity/src/account_lifecycle_http.rs:343',
    form: () => ({ news_portal: 'true', support_portal: 'true' }) },
  ...['', '/en', '/pl'].map(prefix => ({
    method: 'POST', path: `${prefix}/account/sudo`, status: 302, refusal: 401,
    source: 'crates/identity/src/credentials_http.rs:323',
    form: ({ account }) => ({ current_password: account?.password ?? 'unused' }),
  })),
];
for (const { method, path } of [...identityWrites, ...editorialRoutes.map(([method, path]) => ({ method, path }))]) {
  if (!publicRoutes.has(`${method} ${path}`)) throw new Error(`No public registration for ${method} ${path}`);
}

async function assertIdentityWrites(context, request, locale, account) {
  const db = database();
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  const entry = account ? `/${locale}/` : `/${locale}/login`;
  let markerId = idFixture;
  try {
    for (const fixture of identityWrites) {
      const url = fixturePath(fixture.path, locale).replace('{id}', markerId);
      const allowed = Boolean(account) && !fixture.bearer;
      await test.step(`${fixture.method} ${fixture.path}`, async () => {
        const token = await csrfPage(request, entry);
        const cookie = (await context.cookies(origin)).find(({ name }) => name === '__Host-oteryn_session');
        expect(Boolean(cookie), 'session for mutation').toBe(true);
        const sessionHash = createHash('sha256').update(cookie.value).digest();
        const before = allowed ? null : await db.snapshot(sessionHash);
        const response = await request.fetch(url, {
          method: fixture.method,
          ...(fixture.json ? { data: fixture.json } : { form: { _csrf: token, ...fixture.form({ account, locale }) } }),
          headers: { 'x-csrf-token': token, origin },
          maxRedirects: 0,
        });
        try {
          expect(response.status(), `${fixture.method} ${url} (${fixture.source})`)
            .toBe(allowed ? fixture.status : fixture.refusal);
          if (allowed && fixture.captureMarker) markerId = (await response.json()).id;
          if (!allowed && fixture.refusal === 303) expect(response.headers().location).toBe(`/${locale}/login`);
          if (before) assertRefusalState(before, await db.snapshot(sessionHash));
        } finally {
          await response.dispose();
        }
      });
    }
  } finally {
    await db.close();
  }
}


async function assertExport(request, locale, accountId) {
  for (const lang of ['pl', 'en']) {
    const path = `/${lang}/account/privacy/export`;
    await test.step(`GET ${path} with the actor's sudo state`, async () => {
      if (!accountId) await csrfPage(request, `/${locale}/login`);
      const response = await request.get(path, { maxRedirects: 0 });
      try {
        expect(response.status(), 'owner-only export gate').toBe(accountId ? 200 : 303);
        if (!accountId) expect(response.headers().location).toMatch(/^\/(?:pl\/|en\/)?login$/);
        if (accountId) expect((await response.json()).account.id).toBe(accountId);
      } finally {
        await response.dispose();
      }
    });
  }
}
