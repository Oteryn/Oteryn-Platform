import { mirrorHead } from './head.mjs';
import { randomBytes } from 'node:crypto';
import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';
import { seedBrowserAccount } from './actors.mjs';
import { closedOrder } from './native-checkout.mjs';

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

async function playerCommands({ request, token, definition, path, guard, source, origin, snapshot, verifyRefusal }) {
  const { accountId: target } = seedBrowserAccount('en');
  const { accountId: requester } = seedBrowserAccount('en');
  await query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='support'", [requester]);
  const [{ id: ticket }] = await query(`
    INSERT INTO tickets(account_id,request_key,category,subject) VALUES($1,uuidv7(),'account','Authorization fixture')
    RETURNING id
  `, [target]);
  const [{ id: evidence }] = await query(`
    INSERT INTO support_mfa_reset_evidence(account_id,ticket_id,verified_by) VALUES($1,$2,$3) RETURNING id
  `, [target, ticket, requester]);
  const reason = 'Local authorization matrix';
  const [{ id: reset }] = await query(`
    INSERT INTO player_mfa_reset_requests(account_id,requested_by,evidence,reason) VALUES($1,$2,$3,$4) RETURNING id
  `, [target, requester, evidence, reason]);
  const order = await closedOrder(target);
  const [{ hold_id: hold }] = await query(`
    INSERT INTO coin_purchase_holds(account_id,order_id,reason)
    VALUES($1,$2,'reversal_insufficient_balance') RETURNING hold_id
  `, [target, order]);
  const targetSession = randomBytes(32);
  await query(`
    INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token)
    VALUES($1,$2,clock_timestamp()+interval '10 minutes',clock_timestamp()+interval '20 minutes','Local authorization fixture',$3)
  `, [targetSession, target, randomBytes(32).toString('hex')]);
  const generation = async () => (await query(
    'SELECT native_security_generation::int AS generation FROM accounts WHERE id=$1', [target]))[0].generation;
  const cases = {
    note: [{ body: reason }, 'players.note.added'],
    logout: [{}, 'players.logout_all'],
    kick: [{}, 'players.kick'],
    'release-coin-hold': [{ hold }, 'players.coin_hold.released'],
    'verify-identity': [{ ticket }, 'players.mfa_reset.identity_verified'],
    'request-reset': [{ evidence }, 'players.mfa_reset.requested'],
    'approve-reset': [{ request: reset }, 'players.mfa_reset.completed'],
  };
  expect(Object.keys(cases).sort()).toEqual(Object.keys(guard.actions).sort());
  for (const [action, [fields, audit]] of Object.entries(cases)) {
    const permissions = [...guard.all, guard.actions[action], ...(guard.actionAll?.[action] ?? [])];
    const allowed = permissions.every(permission => definition.permissions.includes(permission));
    await test.step(`POST ${path} command ${action}`, async () => {
      const before = !allowed ? await snapshot() : null;
      const count = async () => (await query('SELECT count(*)::int AS n FROM audit_log WHERE action=$1 AND target=$2',
        [audit, target]))[0].n;
      const previous = allowed ? await count() : null;
      const beforeKick = allowed && action === 'kick' ? await generation() : null;
      const response = await request.post(path.replace('{id}', target), {
        form: { action, reason, ...fields }, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
      });
      try {
        const anonymous = definition.key === 'anonymous';
        expect(response.status(), `${source} ${action}`).toBe(allowed || anonymous ? 303 : 403);
        if (anonymous) expect(response.headers().location).toBe('/login');
        if (before) {
          await verifyRefusal(before, definition.role && !definition.permissions.includes('admin.access')
            ? ['admin.access.denied'] : []);
        } else {
          expect(await count(), `${action} committed its action-specific audit row`).toBe(previous + 1);
          if (action === 'note') {
            expect(await query('SELECT 1 FROM player_notes WHERE account_id=$1 AND body=$2',
              [target, reason])).toHaveLength(1);
          }
          if (action === 'kick') expect(await generation()).toBe(beforeKick + 1);
          if (action === 'approve-reset') {
            expect(await query('SELECT 1 FROM player_mfa_reset_requests WHERE id=$1 AND approved_by IS NOT NULL',
              [reset])).toHaveLength(1);
            expect(await query('SELECT 1 FROM support_mfa_reset_evidence WHERE id=$1 AND consumed_at IS NOT NULL',
              [evidence])).toHaveLength(1);
          }
          if (action === 'logout') expect(await query('SELECT 1 FROM web_sessions WHERE id_hash=$1', [targetSession])).toHaveLength(0);
          if (action === 'release-coin-hold') {
            expect(await query('SELECT 1 FROM coin_purchase_hold_releases WHERE hold_id=$1', [hold])).toHaveLength(1);
          }
        }
      } finally {
        await response.dispose();
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
  for (const [method, path] of staffActionRoutes) {
    const { guard, source } = routes.find(r => r.listener === 'public' && r.method === method && r.path === path)
      .registrations[0];
    const players = path.startsWith('/admin/players/');
    const mutation = method === 'POST';
    if (players && mutation) {
      await playerCommands({ request, token, definition, path, guard, source, origin, snapshot, verifyRefusal });
      continue;
    }
    const permitted = permission => definition.permissions.includes(permission);
    const allowed = guard.all.every(permitted) && (!guard.any || guard.any.some(permitted));
    const form = {
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
        if (!mutation) await mirrorHead(request, path, url, response);
        if (anonymous) expect(response.headers().location).toBe('/login');
        if (before) {
          const audit = definition.role && (!players || !permitted('admin.access'));
          await verifyRefusal(before, audit ? ['admin.access.denied'] : []);
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
