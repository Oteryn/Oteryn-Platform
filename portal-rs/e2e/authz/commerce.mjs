import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';
import { seedBrowserAccount } from './actors.mjs';

export const commerceRoutes = [
  ['POST', '/admin/commerce/{action}'],
  ['POST', '/admin/commerce/approvals/{id}'],
  ['POST', '/admin/commerce/cancellations/{id}'],
];

export async function exerciseCommerce(request, locale, token, definition, routes, snapshot, verifyRefusal) {
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  const anonymous = definition.key === 'anonymous';
  const { accountId: proposer } = seedBrowserAccount(locale);
  await query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'", [proposer]);
  const [{ id: target }] = await query("SELECT id FROM accounts WHERE email='bazaar-escrow@local.test'");
  const approvals = await query(`
    INSERT INTO commerce_operations(id,actor_id,target_id,command,reason_sha256,requires_approval)
    SELECT uuidv7(),$1,$2,'{"kind":"grant","days":7}',encode(sha256('Local authorization fixture'::bytea),'hex'),true
    FROM generate_series(1,2) RETURNING id
  `, [proposer, target]);
  const [{ id: proposal }] = await query('SELECT uuidv7() AS id');
  for (const [index, [method, path]] of commerceRoutes.entries()) {
    const { guard, source } = routes.find(r => r.listener === 'public' && r.method === method && r.path === path)
      .registrations[0];
    const permitted = permission => definition.permissions.includes(permission);
    const hasPanelAccess = guard.any.some(permitted);
    const allowed = guard.all.every(permitted) && hasPanelAccess && permitted(guard.actions.grant);
    const id = index === 0 ? proposal : approvals[index - 1].id;
    const targetPath = path.replace('{action}', 'grant').replace('{id}', id);
    const reason = ['Local authorization fixture', 'value_verified', 'value_withdrawn'][index];
    await test.step(`${method} ${path}`, async () => {
      const before = allowed ? null : await snapshot();
      const response = await request.post(targetPath, {
        form: { _csrf: token, reason, ...(index === 0 && { request_id: id, target, days: '7' }) },
        headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
      });
      try {
        expect(response.status(), source).toBe(allowed || anonymous ? 303 : 403);
        if (anonymous) expect(response.headers().location).toBe('/login');
        if (before) {
          const outerDenial = definition.role && !guard.all.every(permitted);
          // A denied command logs its refusal after the panel admits the actor.
          const commandDenial = index === 0 && hasPanelAccess;
          const audit = outerDenial ? ['admin.access.denied'] : commandDenial ? ['commerce.denied'] : [];
          await verifyRefusal(before, audit);
        }
        if (allowed) {
          const [row] = await query('SELECT requires_approval,approved_by,result FROM commerce_operations WHERE id=$1', [id]);
          expect(row.requires_approval).toBe(true);
          if (index === 0) expect(row.approved_by).toBeNull();
          if (index === 1) {
            expect((await query('SELECT 1 FROM premium_ledger WHERE request_id=$1 AND account_id=$2', [id, target])).length)
              .toBe(1);
          }
          if (index === 2) expect(row.result).toEqual({ status: 'cancelled' });
        }
      } finally {
        await response.dispose();
      }
    });
  }
}
