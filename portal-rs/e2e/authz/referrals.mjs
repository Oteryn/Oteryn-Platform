import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';

export const referralRoutes = [
  ['POST', '/{lang}/account/referrals'],
  ['GET', '/{lang}/referrals/{code}'],
  ['POST', '/{lang}/referrals/{code}'],
];

export async function exerciseReferrals(request, locale, token, accountId, restricted, routes, snapshot, verifyRefusal) {
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  const [referral] = await query(`
    INSERT INTO referral_links(account_id,created_at)
    SELECT id,now()-interval '1 hour' FROM accounts WHERE email='bazaar-escrow@local.test'
    ON CONFLICT (account_id) DO UPDATE SET account_id=EXCLUDED.account_id
    RETURNING account_id,code
  `);
  const entry = `/${locale}/account/referrals`;
  for (const [method, path] of referralRoutes) {
    const creation = path.endsWith('/account/referrals');
    const { guard, source } = routes.find(r => r.listener === 'public' && r.method === method && r.path === path)
      .registrations[0];
    expect(guard.kind).toBe(creation ? 'logged-in' : 'anonymous');
    const status = method === 'GET' ? 200 : restricted ? 403 : 303;
    const refused = status === 403 || (creation && !accountId);
    await test.step(`${method} ${path}`, async () => {
      const before = refused ? await snapshot() : null;
      const response = await request.fetch(path.replace('{lang}', locale).replace('{code}', referral.code), {
        method, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
        ...(method === 'POST' && { form: creation ? { kind: 'link', _csrf: token } : { _csrf: token } }),
      });
      try {
        expect(response.status(), source).toBe(status);
        if (before) await verifyRefusal(before);
        if (status === 303) {
          expect(response.headers().location).toBe(accountId ? entry : `/${locale}/${creation ? 'login' : 'register'}`);
        }
        if (creation && !refused) {
          expect((await query('SELECT code FROM referral_links WHERE account_id=$1', [accountId])).length).toBe(1);
        }
        if (!creation && method === 'POST' && accountId && !refused) {
          const [binding] = await query('SELECT inviter_id FROM referral_campaign_invitations WHERE invitee_id=$1', [accountId]);
          expect(binding?.inviter_id).toBe(referral.account_id);
        }
      } finally {
        await response.dispose();
      }
    });
  }
}
