import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';

export const partyRoutes = [
  ['POST', '/{lang}/party'],
  ['POST', '/{lang}/party/{id}/report'],
  ['POST', '/{lang}/party/{id}/remove'],
  ['POST', '/admin/{lang}/party/{id}/moderate'],
];

async function postFixture(owner) {
  const [row] = await query(`
    INSERT INTO pd_party_posts
      (account_id,character_id,character_name,world_id,purpose,min_level,max_level,profession,note,starts_at,expires_at)
    SELECT $1,uuidv7(),'Local Matrix Character',id,'hunt',1,100,'any','Local authorization fixture',
           now()+interval '10 minutes',now()+interval '1 hour'
    FROM world_status ORDER BY id LIMIT 1 RETURNING id,character_id
  `, [owner]);
  return row;
}

export async function exerciseParty(request, locale, token, accountId, definition, routes, snapshot, verifyRefusal) {
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  const [{ id: escrow }] = await query("SELECT id FROM accounts WHERE email='bazaar-escrow@local.test'");
  const other = await postFixture(escrow);
  const own = accountId ? await postFixture(accountId) : other;
  const starts = Math.floor(Date.now() / 1000) + 600;
  const viewer = Boolean(accountId) && !definition.restricted;
  const [{ id: initialReport }] = await query(`
    INSERT INTO pd_party_reports(post_id,account_id,reason)
    VALUES($1,$2,'Local authorization fixture') RETURNING id
  `, [other.id, escrow]);
  const forms = [
    { character: own.character_id, purpose: 'hunt', min_level: '1', max_level: '100', profession: 'any',
      starts_at: String(starts), expires_at: String(starts + 600), note: 'Local authorization fixture' },
    { reason: 'Local authorization report' },
    {},
    { reason: 'Local authorization review', action: 'remove', report_ids: initialReport },
  ];
  for (const [index, [method, pattern]] of partyRoutes.entries()) {
    const { guard, source } = routes.find(r => r.listener === 'public' && r.method === method && r.path === pattern)
      .registrations[0];
    const staff = guard.kind === 'staff';
    // Posting needs an authoritative character cut. The fixture record is only a party entry.
    const allowed = staff ? guard.all.every(p => definition.permissions.includes(p)) : index > 0 && viewer;
    const status = allowed || (staff && !accountId) ? 303 : 403;
    const id = index === 2 ? own.id : other.id;
    const target = pattern.replace('{lang}', locale).replace('{id}', id);
    await test.step(`${method} ${pattern}`, async () => {
      const before = allowed ? null : await snapshot();
      const response = await request.post(target, {
        form: { ...forms[index], _csrf: token }, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
      });
      try {
        const policy = index === 0 ? 'crates/community/src/party/party_store.rs:59' : source;
        expect(response.status(), policy).toBe(status);
        if (before) await verifyRefusal(before, staff && definition.role ? ['admin.access.denied'] : []);
        if (staff && !accountId) expect(response.headers().location).toBe('/login');
        if (allowed && index === 1) {
          expect((await query('SELECT 1 FROM pd_party_reports WHERE post_id=$1 AND account_id=$2',
            [other.id, accountId])).length).toBe(1);
        }
        if (allowed && (index === 2 || staff)) {
          expect((await query('SELECT status FROM pd_party_posts WHERE id=$1', [id]))[0].status).toBe('removed');
        }
      } finally {
        await response.dispose();
      }
    });
  }
}
