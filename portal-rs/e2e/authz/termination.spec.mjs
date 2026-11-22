import { readFileSync } from 'node:fs';
import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';
import { actorDefinitions, csrfPage, openActor, seedBrowserAccount, sessionHash } from './actors.mjs';
import { assertRefusalState, database } from './database.mjs';

const { routes, roles } = JSON.parse(readFileSync(new URL('./routes.json', import.meta.url), 'utf8'));
const pattern = '/{lang}/account/privacy/delete';
const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';

async function sudo(request, locale, password) {
  const token = await csrfPage(request, `/${locale}/account/sudo`);
  const response = await request.post(`/${locale}/account/sudo`, {
    form: { current_password: password, _csrf: token },
    headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
  });
  try {
    expect(response.status(), 'current password establishes recent authorization').toBe(302);
  } finally {
    await response.dispose();
  }
  return csrfPage(request, `/${locale}/account/privacy/delete`);
}

test.describe('account termination authorization', () => {
  for (const definition of actorDefinitions(roles)) {
    test(definition.key, async ({}, { project }) => {
      const { locale } = project.metadata;
      const path = `/${locale}/account/privacy/delete`;
      const anonymous = definition.key === 'anonymous';
      const issuer = definition.restricted
        ? (await query("SELECT id FROM accounts WHERE email='bazaar-escrow@local.test'"))[0].id
        : undefined;
      const { request, account, accountId, signInAgain } = await openActor(locale, definition, issuer);
      const db = database();
      try {
        if (!anonymous) {
          // This account is deliberately never the last holder of a protected role.
          const { accountId: survivor } = seedBrowserAccount(locale);
          await query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'",
            [survivor]);
        }
        const pending = async () => (await query(`
          SELECT id FROM account_terminations WHERE account_id=$1 AND cancelled_at IS NULL AND finalized_at IS NULL
        `, [accountId])).length;
        for (const suffix of ['', '/cancel']) {
          const registration = routes.find(row => row.listener === 'public' && row.method === 'POST'
            && row.path === pattern + suffix).registrations[0];
          expect(registration.guard.kind).toBe('logged-in');
          if (suffix && !anonymous) await signInAgain();
          await test.step(`POST ${pattern}${suffix}`, async () => {
            const token = anonymous ? await csrfPage(request, `/${locale}/login`)
              : await sudo(request, locale, account.password);
            const hash = await sessionHash(request);
            const before = anonymous ? await db.snapshot(hash) : null;
            const response = await request.post(path + suffix, {
              form: { _csrf: token, current_password: account?.password ?? 'unused', confirmation: 'TERMINATE' },
              headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
            });
            try {
              expect(response.status(), registration.source).toBe(anonymous ? 403 : 303);
              if (before) {
                assertRefusalState(before, await db.snapshot(hash));
              } else {
                expect(response.headers().location).toBe(suffix ? path : `/${locale}/login`);
                expect(await pending(), 'termination lifecycle state').toBe(suffix ? 0 : 1);
                if (!suffix) expect((await query('SELECT 1 FROM web_sessions WHERE id_hash=$1', [hash])).length).toBe(0);
              }
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
