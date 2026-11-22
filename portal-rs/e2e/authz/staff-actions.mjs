import { randomUUID } from 'node:crypto';
import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';

export const staffActionRoutes = [
  ['GET', '/admin/players/{id}'],
  ['POST', '/admin/players/{id}'],
  ['GET', '/admin/moderation/enforcement/{id}'],
  ['POST', '/admin/moderation/enforcement/{id}'],
];
export const publicationRoutes = [
  ['POST', '/admin/{lang}/bazaar'],
  ['POST', '/admin/world'],
];

export async function exercisePublication(request, locale, token, definition, routes, snapshot, verifyRefusal) {
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  const anonymous = definition.key === 'anonymous';
  const [{ id: fee }] = await query('SELECT uuidv7() AS id');
  const [version] = await query(`
    INSERT INTO world_versions(id,source_commit,status) VALUES(uuidv7(),repeat('a',40),'ready') RETURNING id
  `);
  const [previous] = await query('SELECT version_id,previous_version_id FROM world_active_version WHERE singleton');
  for (const [method, path] of publicationRoutes) {
    const { guard, source } = routes.find(r => r.listener === 'public' && r.method === method && r.path === path)
      .registrations[0];
    const world = path === '/admin/world';
    const allowed = guard.all.every(permission => definition.permissions.includes(permission))
      && (!world || definition.permissions.includes(guard.actions.publish));
    const form = world ? { action: 'publish', version: version.id } : {
      action: 'fees', id: fee, coin_bps: '1000', gold_bps: '250', listing_fee: '0',
      promotion_price: '1', reason: 'Disposable authorization fixture',
    };
    const target = path.replace('{lang}', locale);
    await test.step(`${method} ${path}`, async () => {
      const before = allowed ? null : await snapshot();
      const response = await request.post(target, {
        form: { ...form, _csrf: token }, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
      });
      try {
        expect(response.status(), source).toBe(allowed || anonymous ? 303 : 403);
        if (anonymous) expect(response.headers().location).toBe('/login');
        if (before) {
          const outer = world ? guard.all.slice(0, 2) : guard.all;
          const denial = definition.role && !outer.every(permission => definition.permissions.includes(permission));
          await verifyRefusal(before, denial ? ['admin.access.denied'] : []);
        }
        if (allowed && world) {
          const [active] = await query('SELECT version_id FROM world_active_version WHERE singleton');
          expect(active.version_id).toBe(version.id);
        }
        if (allowed && !world) {
          expect((await query('SELECT 1 FROM bazaar_fee_revisions WHERE id=$1', [fee])).length).toBe(1);
        }
      } finally {
        await response.dispose();
        if (world) {
          await query(`
            UPDATE world_active_version SET version_id=$1,previous_version_id=$2
            WHERE singleton AND version_id=$3
          `, [previous.version_id, previous.previous_version_id, version.id]);
        }
      }
    });
  }
}

export async function exerciseStaffActions(request, token, definition, routes, snapshot, verifyRefusal) {
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  const anonymous = definition.key === 'anonymous';
  const [{ id: target }] = await query("SELECT id FROM accounts WHERE email='bazaar-escrow@local.test'");
  const effective = Math.floor(Date.now() / 1000) - 300;
  const expires = effective + 900;
  const [sanction] = await query(`
    INSERT INTO sanctions(account_id,category,status,public_reason,effective_at,expires_at,created_by)
    VALUES($1,'warning','active','Local authorization fixture',to_timestamp($2),to_timestamp($3),$1)
    RETURNING id,lock_version
  `, [target, effective, expires]);
  const note = `Local authorization note ${randomUUID()}`;
  for (const [method, path] of staffActionRoutes) {
    const { guard, source } = routes.find(r => r.listener === 'public' && r.method === method && r.path === path)
      .registrations[0];
    const players = path.startsWith('/admin/players/');
    const mutation = method === 'POST';
    const permitted = permission => definition.permissions.includes(permission);
    const allowed = guard.all.every(permitted) && (!guard.any || guard.any.some(permitted))
      && (!guard.actions || permitted(guard.actions.note));
    const form = players ? { action: 'note', body: note } : {
      action: 'sanction', category: 'warning', target, status: 'revoked',
      public_reason: 'Local authorization fixture', moderator_notes: '',
      effective_at: String(effective), expires_at: String(expires), lock_version: String(sanction.lock_version),
    };
    const url = path.replace('{id}', players ? target : sanction.id);
    await test.step(`${method} ${path}`, async () => {
      const before = mutation && !allowed ? await snapshot() : null;
      const response = await request.fetch(url, {
        method, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
        ...(mutation && { form }),
      });
      try {
        expect(response.status(), source).toBe(allowed ? mutation ? 303 : 200 : anonymous ? 303 : 403);
        if (anonymous) expect(response.headers().location).toBe('/login');
        if (before) {
          const audit = definition.role && (!players || !permitted('admin.access'));
          await verifyRefusal(before, audit ? ['admin.access.denied'] : []);
        }
        if (allowed && mutation && players) {
          expect((await query('SELECT 1 FROM player_notes WHERE account_id=$1 AND body=$2', [target, note])).length).toBe(1);
        }
        if (allowed && mutation && !players) {
          const [revoked] = await query('SELECT status FROM sanctions WHERE id=$1', [sanction.id]);
          expect(revoked.status).toBe('revoked');
        }
      } finally {
        await response.dispose();
      }
    });
  }
}
