import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';

export const recoveryMethods = [
  ['POST', '', 200], ['POST', '/confirm', 200],
  ['POST', '/revoke', 302], ['DELETE', '', 302],
];

export async function exerciseRecovery(request, prefix, token, account, accountId, snapshot, verifyRefusal) {
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  const path = `${prefix}/account/recovery-key`;
  const form = { current_password: account?.password ?? 'unused' };
  let key = 'OTERYN-00000000-00000000-00000000-00000000-00000000';

  async function recover(method, suffix, status) {
    return test.step(`${method} ${path}${suffix}`, async () => {
      const before = account ? null : await snapshot();
      const response = await request.fetch(`${path}${suffix}`, {
        method, form: suffix === '/confirm' ? { ...form, recovery_key: key } : form,
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
    if (account && method === 'DELETE') await recover('POST', '', 200);
    await recover(method, suffix, status);
  }
}
