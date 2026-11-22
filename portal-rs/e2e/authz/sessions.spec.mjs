import { randomBytes } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';
import { actorDefinitions, csrfPage, openActor, sessionHash as hashSession } from './actors.mjs';
import { calendarRoutes, exerciseCalendar, exercisePublishedContent } from './calendar.mjs';
import { characterFamilies, exerciseCharacterFamilies } from './character-families.mjs';
import { consoleRoutes, exerciseConsole } from './console.mjs';
import { assertRefusalState, database } from './database.mjs';
import { exerciseProtocol, exerciseUnavailableAuthorities, protocolRoutes } from './protocol.mjs';
import { exerciseRecovery, recoveryMethods } from './recovery.mjs';
import { exerciseResourceReads, resourceReads } from './resource-reads.mjs';
import { exerciseSupport, supportRoutes } from './support.mjs';

const { routes, roles } = JSON.parse(readFileSync(new URL('./routes.json', import.meta.url), 'utf8'));
const signedOutFixture = '019a541b-a000-7000-8000-000000000001';
const variants = [
  { method: 'DELETE', suffix: '', selection: 'others' },
  { method: 'POST', suffix: '/others', selection: 'others' },
  { method: 'POST', suffix: '/{id}', selection: 'one' },
  { method: 'DELETE', suffix: '/{id}', selection: 'one' },
  { method: 'POST', suffix: '/all', selection: 'all' },
];
const aliases = ['', '/pl', '/en'];
for (const prefix of aliases) {
  for (const { method, suffix } of variants) {
    const path = `${prefix}/account/sessions${suffix}`;
    if (!routes.some(route => route.listener === 'public' && route.method === method && route.path === path)) {
      throw new Error(`No session revocation registration for ${method} ${path}`);
    }
  }
  const logout = routes.find(route => route.listener === 'public' && route.method === 'POST'
    && route.path === `${prefix}/logout`);
  if (logout?.registrations[0].guard.kind !== 'logged-in') {
    throw new Error(`No authenticated POST registration for ${prefix}/logout`);
  }
  for (const [method, suffix] of recoveryMethods) {
    const path = `${prefix}/account/recovery-key${suffix}`;
    const route = routes.find(r => r.listener === 'public' && r.method === method && r.path === path);
    if (route?.registrations[0].guard.kind !== 'logged-in') {
      throw new Error(`No authenticated registration for ${method} ${path}`);
    }
  }
}

for (const [method, path] of [
  ...supportRoutes, ...calendarRoutes, ...protocolRoutes, ...consoleRoutes, ...characterFamilies, ...resourceReads,
]) {
  if (!routes.some(route => route.listener === 'public' && route.method === method && route.path === path)) {
    throw new Error(`No workflow registration for ${method} ${path}`);
  }
}

async function secondSession(sessionHash) {
  const rows = await query(`
    INSERT INTO web_sessions(id_hash, account_id, authorization_generation, idle_expires_at,
                             absolute_expires_at, user_agent_summary, csrf_token)
    SELECT $1, account_id, authorization_generation, clock_timestamp() + interval '10 minutes',
           clock_timestamp() + interval '20 minutes', 'Local revocation target', $2
    FROM web_sessions WHERE id_hash=$3 AND account_id IS NOT NULL
    RETURNING session_id
  `, [randomBytes(32), randomBytes(32).toString('hex'), sessionHash]);
  expect(rows.length, 'secondary session belongs to the authenticated actor').toBe(1);
  return rows[0].session_id;
}

test.describe('session, support and content authorization', () => {
  for (const definition of actorDefinitions(roles)) {
    test(definition.key, async ({}, info) => {
      const { locale } = info.project.metadata;
      const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
      const anonymous = definition.key === 'anonymous';
      const issuer = definition.restricted
        ? (await query("SELECT id FROM accounts WHERE email='bazaar-escrow@local.test'"))[0].id
        : undefined;
      const db = database();
      try {
        for (const prefix of aliases) {
          const { request, account, accountId } = await openActor(info.project.metadata.locale, definition, issuer);
          try {
            const token = await csrfPage(request, anonymous ? `/${locale}/login` : `/${locale}/`);
            const sessionHash = await hashSession(request);
            const snapshot = () => db.snapshot(sessionHash);
            const verify = async (before, actions) => assertRefusalState(before, await snapshot(), actions);
            if (!prefix) {
              await exerciseSupport(request, locale, token, definition, routes, snapshot, verify);
              await exerciseCalendar(request, token, definition, routes, snapshot, verify);
              await exerciseConsole(request, locale, token, definition, routes, snapshot, verify);
              await exerciseProtocol(request, token, routes, snapshot, verify);
              await exerciseUnavailableAuthorities(request, locale, anonymous, token, routes);
              await exercisePublishedContent(request, locale, routes);
              await exerciseCharacterFamilies(request, accountId, routes);
              await exerciseResourceReads(request, accountId, locale, token, routes);
            }
            await exerciseRecovery(request, prefix, token, account, accountId, snapshot, verify);
            for (const { method, suffix, selection } of variants) {
              const pattern = `${prefix}/account/sessions${suffix}`;
              await test.step(`${method} ${pattern}`, async () => {
                const id = anonymous ? signedOutFixture : await secondSession(sessionHash);
                const target = pattern.replace('{id}', id);
                const before = anonymous ? await db.snapshot(sessionHash) : null;
                const response = await request.fetch(target, {
                  method, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
                });
                try {
                  expect(response.status(), 'session actor gate').toBe(anonymous ? 403 : 302);
                  if (anonymous) {
                    assertRefusalState(before, await db.snapshot(sessionHash));
                  } else {
                    const destination = selection === 'all' ? 'login' : 'account/sessions';
                    expect(response.headers().location).toMatch(new RegExp(`^/(pl|en)/${destination}$`));
                    const [{ targetExists, currentExists }] = await query(`
                      SELECT EXISTS(SELECT 1 FROM web_sessions WHERE session_id=$1) AS "targetExists",
                             EXISTS(SELECT 1 FROM web_sessions WHERE id_hash=$2) AS "currentExists"
                    `, [id, sessionHash]);
                    expect(targetExists, 'owned target session revoked').toBe(false);
                    expect(currentExists, 'requesting session state').toBe(selection !== 'all');
                  }
                } finally {
                  await response.dispose();
                }
              });
            }
          } finally {
            await request.dispose();
          }

          const actor = await openActor(info.project.metadata.locale, definition, issuer);
          try {
            await test.step(`POST ${prefix}/logout`, async () => {
              const token = await csrfPage(actor.request, anonymous ? `/${locale}/login` : `/${locale}/`);
              const sessionHash = await hashSession(actor.request);
              const before = anonymous ? await db.snapshot(sessionHash) : null;
              const response = await actor.request.post(`${prefix}/logout`, {
                headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
              });
              try {
                expect(response.status(), 'crates/identity/src/login_http.rs:222').toBe(302);
                expect(response.headers().location).toMatch(anonymous ? /^\/(pl|en)\/login$/ : /^\/(pl|en)\/$/);
                if (before) assertRefusalState(before, await db.snapshot(sessionHash));
                else expect((await query('SELECT 1 FROM web_sessions WHERE id_hash=$1', [sessionHash])).length).toBe(0);
              } finally {
                await response.dispose();
              }
            });
          } finally {
            await actor.request.dispose();
          }
        }
      } finally {
        await db.close();
      }
    });
  }
});
