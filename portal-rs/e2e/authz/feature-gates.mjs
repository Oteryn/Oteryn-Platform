import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';

const fixture = '019a541b-a000-7000-8000-000000000001';
const cases = [
  { method: 'POST', path: '/{lang}/pass/{season}/buy', anonymous: 401, signed: 404,
    source: 'crates/minigames/src/pass/web.rs:136', form: { operation: fixture } },
  { method: 'POST', path: '/{lang}/pass/{season}/claim', anonymous: 401, signed: 404,
    source: 'crates/minigames/src/pass/web.rs:136', form: { operation: fixture, reward: fixture } },
  { method: 'POST', path: '/{lang}/collection-pvp', anonymous: 403, signed: 503,
    source: 'crates/minigames/src/collection_pvp/mod.rs:127', form: { op: 'create', capacity: '2' } },
  { method: 'GET', path: '/{lang}/bazaar/sellers/{seller}', anonymous: 404, signed: 404,
    source: 'crates/bazaar/src/moderation_web.rs:60' },
  { method: 'GET', path: '/{lang}/account/bazaar/{listing}/promote', anonymous: 303, signed: 200,
    source: 'crates/bazaar/src/moderation_web.rs:162' },
  { method: 'POST', path: '/{lang}/account/bazaar/{listing}/promote', anonymous: 403, signed: 403,
    source: 'crates/bazaar/src/moderation_web.rs:60', promotion: true },
  { method: 'POST', path: '/{lang}/account/bazaar/{listing}/report', anonymous: 403, signed: 403,
    source: 'crates/bazaar/src/moderation_web.rs:60',
    form: { id: fixture, category: 'other', detail: 'Local authorization fixture' } },
  { method: 'GET', path: '/{lang}/puzzles/{id}/image', anonymous: 303, signed: 503,
    source: 'crates/minigames/src/puzzles/store.rs:39' },
  { method: 'POST', path: '/{lang}/puzzles/{id}/answer', anonymous: 303, signed: 503,
    source: 'crates/minigames/src/puzzles/store.rs:166', form: { choice: '0' } },
  { method: 'GET', path: '/{lang}/account/characters/requests/{operation}', anonymous: 303, signed: 200,
    source: 'crates/characters/src/create_web/service.rs:269' },
];
export const featureGateRoutes = cases.map(({ method, path }) => [method, path]);

async function unpublishedPuzzle() {
  const today = "(clock_timestamp() AT TIME ZONE 'UTC')::date";
  const [existing] = await query(`SELECT id FROM puzzles_daily WHERE day=${today} AND kind='monster'`);
  if (existing) return existing.id;
  const [world] = await query("INSERT INTO world_versions(id,source_commit,status) VALUES(uuidv7(),repeat('b',40),'ready') RETURNING id");
  const label = text => ({ pl: text, en: text });
  const definition = {
    kind: 'monster', world_version: world.id, question: label('Choose a monster'), clue: label('Local puzzle clue'),
    choices: ['first', 'second'].map(key => ({ label: label(key), source: { kind: 'monster', key } })),
    correct_choice: 0,
  };
  const image = Buffer.from('UklGRiIAAABXRUJQVlA4TAYAAAAvAAAAAAfQ//73v/+BiOh/AAA=', 'base64');
  const [row] = await query(`
    INSERT INTO puzzles_daily(day,kind,world_version,definition,image)
    VALUES(${today},'monster',$1,$2,$3) RETURNING id
  `, [world.id, definition, image]);
  return row.id;
}

export async function exerciseFeatureGates(request, locale, token, accountId, routes, snapshot, verifyRefusal) {
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  const [{ id: seller }] = await query("SELECT id FROM accounts WHERE email='bazaar-escrow@local.test'");
  const [fee] = await query(`
    INSERT INTO bazaar_fee_revisions(id,actor,coin_bps,gold_bps,listing_fee,promotion_price,reason)
    VALUES(uuidv7(),$1,1000,250,0,1,'Local authorization fixture') RETURNING id
  `, [seller]);
  const puzzle = await unpublishedPuzzle();
  const [operation] = await query(`
    INSERT INTO character_commands(id, issuer_decision_id, request_key, command, account_id, binding_json, intent_json)
    SELECT id, uuidv7(), 'authz-status:' || id, 'CreateCharacter', $1, '{}',
           jsonb_build_object('account_id', $1::uuid)::text
    FROM (SELECT uuidv7() AS id) fixture
    RETURNING id
  `, [accountId || seller]);
  for (const row of cases) {
    const { method, path } = row;
    const { guard } = routes.find(r => r.listener === 'public' && r.method === method && r.path === path)
      .registrations[0];
    const status = accountId ? row.signed : row.anonymous;
    const form = row.promotion ? { id: fixture, fee_revision: fee.id, amount: '1' } : row.form;
    const target = path.replace('{lang}', locale).replace('{season}', fixture)
      .replace('{seller}', seller).replace('{listing}', fixture).replace('{id}', puzzle).replace('{operation}', operation.id);
    expect(guard.kind, path).toBe(path.includes('/sellers/') ? 'anonymous' : 'logged-in');
    await test.step(`${method} ${path}`, async () => {
      const before = method === 'POST' && status !== 503 ? await snapshot() : null;
      const response = await request.fetch(target, {
        method, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
        ...(form && { form: { ...form, _csrf: token } }),
      });
      try {
        expect(response.status(), row.source).toBe(status);
        if (status === 303) expect(response.headers().location).toBe(`/${locale}/login`);
        if (before) await verifyRefusal(before);
      } finally {
        await response.dispose();
      }
    });
  }
}
