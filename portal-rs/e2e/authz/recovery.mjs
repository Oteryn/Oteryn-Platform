import { randomUUID } from 'node:crypto';
import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';
import { csrfPage } from './actors.mjs';

export const recoveryMethods = [
  ['POST', '', 200], ['POST', '/confirm', 200],
  ['POST', '/revoke', 302], ['DELETE', '', 303],
];
export const emailChangeRoutes = ['', '/pl', '/en'].map(prefix => ['POST', `${prefix}/account/security/email`]);

export async function exerciseRecovery(actor, prefix, locale, snapshot, verifyRefusal, openIndependentActor) {
  const { request, token, account, accountId } = actor;
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  const path = `${prefix}/account/recovery-key`;
  const form = { current_password: account?.password ?? 'unused' };
  let key = 'OTERYN-00000000-00000000-00000000-00000000-00000000';

  async function recover(method, suffix, status, { request, token, account, accountId } = actor) {
    return test.step(`${method} ${path}${suffix}`, async () => {
      const before = account ? null : await snapshot();
      const credentials = { current_password: account?.password ?? 'unused' };
      const response = await request.fetch(`${path}${suffix}`, {
        method, form: suffix === '/confirm' ? { ...credentials, recovery_key: key } : credentials,
        headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
      });
      try {
        expect(response.status(), 'crates/identity/src/recovery_key_http.rs:230').toBe(account ? status : 403);
        if (before) await verifyRefusal(before);
        if (account && method === 'POST' && !suffix) {
          const match = /<p class="op-code">(OTERYN(?:-[0-9A-F]{8}){5})<\/p>/.exec(await response.text());
          expect(Boolean(match), 'one-time recovery key rendered').toBe(true);
          key = match[1];
        } else if (account) {
          const [{ confirmed, revoked }] = await query(`
            SELECT confirmed_at IS NOT NULL AS confirmed, revoked_at IS NOT NULL AS revoked
            FROM account_recovery_keys WHERE account_id=$1
          `, [accountId]);
          if (suffix === '/confirm') expect(confirmed, 'recovery key confirmed').toBe(true);
          else {
            expect(response.headers().location).toMatch(/^\/(pl|en)\/account\/recovery-key$/);
            expect(revoked, 'recovery key revoked').toBe(true);
          }
        }
      } finally {
        await response.dispose();
      }
    });
  }

  for (const [method, suffix, status] of recoveryMethods) {
    if (account && method === 'DELETE') {
      // The account's five hourly management calls are shared across method aliases.
      const independent = await openIndependentActor();
      try {
        independent.token = await csrfPage(independent.request, `/${locale}/`);
        await recover('POST', '', 200, independent);
        await recover(method, suffix, status, independent);
      } finally {
        await independent.request.dispose();
      }
      continue;
    }
    await recover(method, suffix, status);
    if (suffix === '/confirm') {
      const change = `${prefix}/account/security/email`;
      const email = `matrix-change-${randomUUID()}@example.invalid`;
      await test.step(`POST ${change}`, async () => {
        const before = account ? null : await snapshot();
        const response = await request.post(change, {
          form: { ...form, email, email_confirmation: email, proof: key, recovery: 'true' },
          headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
        });
        try {
          expect(response.status(), 'crates/identity/src/email_change_http.rs:233').toBe(302);
          const destination = account ? /\/account\/security\/email$/ : /\/login$/;
          expect(response.headers().location).toMatch(destination);
          if (before) await verifyRefusal(before);
          else {
            const [row] = await query(`
              SELECT immediate FROM account_email_changes WHERE account_id=$1 AND new_email=$2
            `, [accountId, email]);
            expect(row?.immediate, 'current recovery proof created the pending change').toBe(true);
          }
        } finally {
          await response.dispose();
        }
      });
      if (account) await recover('POST', '', 200);
    }
  }
}
