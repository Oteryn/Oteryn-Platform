import { mirrorHead } from './head.mjs';
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
  { method: 'GET', path: '/{lang}/puzzles/{id}/image', anonymous: 303, signed: 200,
    source: 'crates/minigames/src/puzzles/store.rs:152', puzzleImage: true },
  { method: 'POST', path: '/{lang}/puzzles/{id}/answer', anonymous: 303, signed: 303,
    source: 'crates/minigames/src/puzzles/store.rs:161', puzzleAnswer: true, form: { choice: '0' } },
  { method: 'POST', path: '/{lang}/account/characters/new', anonymous: 403, signed: 302,
    source: 'crates/characters/src/create_web/http.rs:124', characterCreation: true },
  { method: 'GET', path: '/{lang}/account/characters/requests/{operation}', anonymous: 303, signed: 200,
    source: 'crates/characters/src/create_web/service.rs:269' },
];
export const featureGateRoutes = cases.map(({ method, path }) => [method, path]);

const puzzleImage = Buffer.from('UklGRiIAAABXRUJQVlA4TAYAAAAvAAAAAAfQ//73v/+BiOh/AAA=', 'base64');

async function publishedPuzzle() {
  const today = "(clock_timestamp() AT TIME ZONE 'UTC')::date";
  const [existing] = await query(`SELECT id,world_version FROM puzzles_daily WHERE day=${today} AND kind='monster'`);
  if (existing) return existing;
  const [world] = await query(`
    INSERT INTO world_versions(id,source_commit,status)
    VALUES(uuidv7(),repeat('b',40),'ready') RETURNING id
  `);
  const keys = ['authz-puzzle-first', 'authz-puzzle-second'];
  await query(`
    INSERT INTO world_entities(version_id,kind,key,data,publishable)
    SELECT $1,'Creature',key,'{}'::jsonb,true FROM unnest($2::text[]) fixture(key)
  `, [world.id, keys]);
  await query(`
    INSERT INTO world_visibility(kind,key,allowed)
    SELECT 'Creature',key,true FROM unnest($1::text[]) fixture(key)
    ON CONFLICT(kind,key) DO UPDATE SET allowed=true
  `, [keys]);
  const label = text => ({ pl: text, en: text });
  const definition = {
    kind: 'monster', world_version: world.id, question: label('Choose a monster'), clue: label('Local puzzle clue'),
    choices: keys.map(key => ({ label: label(key), source: { kind: 'Creature', key } })),
    correct_choice: 0,
  };
  const [row] = await query(`
    INSERT INTO puzzles_daily(day,kind,world_version,definition,image)
    VALUES(${today},'monster',$1,$2,$3) RETURNING id,world_version
  `, [world.id, definition, puzzleImage]);
  return row;
}

export async function exerciseFeatureGates(request, locale, token, accountId, routes, snapshot, verifyRefusal) {
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  const [{ id: seller }] = await query("SELECT id FROM accounts WHERE email='bazaar-escrow@local.test'");
  const [fee] = await query(`
    INSERT INTO bazaar_fee_revisions(id,actor,coin_bps,gold_bps,listing_fee,promotion_price,reason)
    VALUES(uuidv7(),$1,1000,250,0,1,'Local authorization fixture') RETURNING id
  `, [seller]);
  const [{ submission, world }] = await query(`
    SELECT uuidv7() AS submission, id AS world FROM world_status ORDER BY id LIMIT 1
  `);
  let operation = fixture;
  const puzzle = await publishedPuzzle();
  const [previous] = await query('SELECT version_id,previous_version_id FROM world_active_version WHERE singleton');
  await query(`
    UPDATE world_active_version SET version_id=$1,previous_version_id=NULL WHERE singleton
  `, [puzzle.world_version]);
  try {
    for (const row of cases) {
      const { method, path } = row;
      const { guard } = routes.find(r => r.listener === 'public' && r.method === method && r.path === path)
        .registrations[0];
      const status = accountId ? row.signed : row.anonymous;
      const form = row.characterCreation
        ? { submission, world, name: 'Aldric Stone', sex: 'Female', vocation: 'Druid' }
        : row.promotion ? { id: fixture, fee_revision: fee.id, amount: '1' } : row.form;
      const target = path.replace('{lang}', locale).replace('{season}', fixture)
        .replace('{seller}', seller).replace('{listing}', fixture).replace('{id}', puzzle.id).replace('{operation}', operation);
      expect(guard.kind, path).toBe(path.includes('/sellers/') ? 'anonymous' : 'logged-in');
      await test.step(`${method} ${path}`, async () => {
        const refused = !accountId || (status >= 400 && status !== 503);
        const before = method === 'POST' && refused ? await snapshot() : null;
        const response = await request.fetch(target, {
          method, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
          ...(form && { form: { ...form, _csrf: token } }),
        });
        try {
          expect(response.status(), row.source).toBe(status);
          if (method === 'GET') await mirrorHead(request, path, target, response);
          if (status === 303) expect(response.headers().location).toBe(`/${locale}/${accountId ? 'puzzles' : 'login'}`);
          if (row.puzzleImage && accountId) {
            expect(response.headers()['content-type']).toBe('image/webp');
            expect((await response.body()).equals(puzzleImage), 'published puzzle image rendered').toBe(true);
          }
          if (row.puzzleAnswer && accountId) {
            const [answer] = await query("SELECT correct FROM puzzles_answers WHERE account_id=$1 AND puzzle_id=$2",
              [accountId, puzzle.id]);
            expect(answer?.correct, 'correct answer recorded for the actor').toBe(true);
          }
          if (row.characterCreation && accountId) {
            const [created] = await query(
              "SELECT id FROM character_commands WHERE account_id=$1 AND request_key=$2 AND command='CreateCharacter'",
              [accountId, `web-create:${accountId}:${submission}`],
            );
            expect(Boolean(created), 'character request was durably queued for the actor').toBe(true);
            operation = created.id;
            expect(response.headers().location).toBe(`/${locale}/account/characters/requests/${operation}`);
          }
          if (before) await verifyRefusal(before);
        } finally {
          await response.dispose();
        }
      });
    }
  } finally {
    const restored = await query(`
      UPDATE world_active_version SET version_id=$1,previous_version_id=$2
      WHERE singleton AND version_id=$3 RETURNING version_id,previous_version_id
    `, [previous.version_id, previous.previous_version_id, puzzle.world_version]);
    expect(restored, 'puzzle fixture restored the world pointers').toEqual([previous]);
  }
}
