import { readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';
import { actorDefinitions, csrfPage, openActor, sessionHash } from './actors.mjs';
import { assertRefusalState, database, rateKey } from './database.mjs';

const { routes, roles } = JSON.parse(readFileSync(new URL('./routes.json', import.meta.url), 'utf8'));
const issue = '/{lang}/admin/store/codes';
const redeem = '/{lang}/shop/codes/redeem';
const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
const actors = actorDefinitions(roles);
const registration = (method, path) => routes.find(row => row.listener === 'public' && row.method === method && row.path === path).registrations[0];

async function issueCode(request, locale, csrf) {
  const [{ id }] = await query('SELECT uuidv7() AS id');
  const now = Math.floor(Date.now() / 1000);
  return request.post(issue.replace('{lang}', locale), {
    headers: { origin, 'x-csrf-token': csrf },
    data: { request_id: id, kind: 'coins', amount: 1, count: 1, max_per_code: 1,
      max_per_account: 1, valid_from: now - 60, valid_until: now + 3600, note: 'Local authorization matrix' },
    maxRedirects: 0,
  });
}

let codes;
test.beforeAll(async ({}, { project }) => {
  const locale = project.metadata.locale;
  const staff = await openActor(locale, actors.find(actor => actor.key === 'platform_admin'));
  try {
    const csrf = await csrfPage(staff.request, `/${locale}/`);
    codes = new Map();
    for (const { key } of actors) {
      const response = await issueCode(staff.request, locale, csrf);
      try {
        expect(response.status()).toBe(200);
        const batch = await response.json();
        expect(batch.codes.length).toBe(1);
        codes.set(key, batch.codes[0]);
      } finally {
        await response.dispose();
      }
    }
  } finally {
    await staff.request.dispose();
  }
});

for (const definition of actors) {
  test(`store-code issuance and redemption ${definition.key}`, async ({}, { project }) => {
    const locale = project.metadata.locale;
    const [issuer] = await query("SELECT id FROM accounts WHERE email='bazaar-escrow@local.test'");
    const { request, accountId } = await openActor(locale, definition, issuer.id);
    const db = database();
    try {
      const csrf = await csrfPage(request, accountId ? `/${locale}/` : `/${locale}/login`);
      const hash = await sessionHash(request);
      for (const method of ['GET', 'HEAD', 'POST']) {
        const { guard, source } = registration(method, issue);
        expect(guard).toEqual({ kind: 'staff', all: ['admin.access', 'store.codes.generate'] });
        const allowed = guard.all.every(permission => definition.permissions.includes(permission));
        await test.step(`${method} ${issue}`, async () => {
          const before = method === 'POST' && !allowed ? await db.snapshot(hash) : null;
          const response = method === 'POST' ? await issueCode(request, locale, csrf)
            : await request.fetch(issue.replace('{lang}', locale), { method, maxRedirects: 0 });
          try {
            expect(response.status(), source).toBe(allowed ? 200 : accountId ? 403 : 303);
            if (!accountId) expect(response.headers().location).toBe('/login');
            if (allowed && method === 'POST') expect((await response.json()).codes.length).toBe(1);
            if (method === 'HEAD') expect((await response.body()).length).toBe(0);
            if (before) assertRefusalState(before, await db.snapshot(hash), definition.role ? ['admin.access.denied'] : []);
          } finally {
            await response.dispose();
          }
        });
      }
      await test.step(`POST ${redeem}`, async () => {
        const { guard, source } = registration('POST', redeem);
        expect(guard.kind).toBe('logged-in');
        const policy = 'store-code-redeem';
        const rate = definition.restricted ? { policy, key: rateKey(policy, accountId) } : undefined;
        const refused = !accountId || definition.restricted;
        const [{ id: operation }] = await query('SELECT uuidv7() AS id');
        const before = refused ? await db.snapshot(hash, rate) : null;
        const response = await request.post(redeem.replace('{lang}', locale), {
          headers: { origin, 'x-csrf-token': csrf }, data: { code: codes.get(definition.key), operation }, maxRedirects: 0,
        });
        try {
          expect(response.status(), source).toBe(!accountId ? 401 : definition.restricted ? 403 : 200);
          if (before) assertRefusalState(before, await db.snapshot(hash, rate));
          else expect(await query('SELECT 1 FROM code_redemptions WHERE idempotency_key=$1 AND account_id=$2',
            [operation, accountId])).toHaveLength(1);
        } finally {
          await response.dispose();
        }
      });
    } finally {
      await request.dispose();
      await db.close();
    }
  });
}
