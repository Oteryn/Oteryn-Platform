import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';

const unusedId = '019a541b-a000-7000-8000-000000000001';
export const resourceReads = [
  ['GET', '/{lang}/account/hunts/{id}'],
  ['GET', '/{lang}/account/hunts/compare'],
  ['GET', '/{lang}/account/hunts/{id}/share'],
  ['POST', '/{lang}/account/hunts/{id}/share'],
  ['GET', '/{locale}/bazaar/{listing}'],
  ['GET', '/{locale}/bazaar/items/{listing}'],
  ['GET', '/{locale}/bazaar/items/new'],
];

async function seedHunts(accountId) {
  const results = await query(`
    WITH facts AS (
      SELECT uuidv7() AS id, uuidv7() AS character, uuidv7() AS generation,
             extract(epoch FROM now())::bigint - 180 + (60 * fixture) AS started
      FROM generate_series(0,1) AS fixture
    ), inserted AS (
      INSERT INTO pd_hunt_sessions
        (account_id,session_id,stream_generation,character_revision,character_id,
         started_at,ended_at,expires_at,has_party,facts)
      SELECT $1,id,generation,1,character,started,started+60,started+3600,false,
             jsonb_build_object(
                'session_id',id,'character_id',character,'started_at',started,'ended_at',started+60,
                'duration_seconds',60,'places','{"fixture":60}'::jsonb,
                'experience_gained',100,'experience_lost',0,
                'loot','{}'::jsonb,'supplies','{}'::jsonb,'damage_dealt','[]'::jsonb,'damage_taken','[]'::jsonb,
                'healing_done',0,'healing_received',0,'monsters_killed','{}'::jsonb,'party',NULL
             )
      FROM facts RETURNING account_id,session_id,stream_generation,character_revision
    )
    INSERT INTO pd_hunt_access
      (account_id,session_id,stream_generation,character_revision,complete,allowed)
    SELECT account_id,session_id,stream_generation,character_revision,true,true FROM inserted
    RETURNING session_id
  `, [accountId]);
  expect(results.length, 'two owned hunt fixtures are available').toBe(2);
  return results.map(row => row.session_id);
}

export async function exerciseResourceReads(request, accountId, locale, token, routes) {
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  const ids = accountId ? await seedHunts(accountId) : [unusedId, unusedId];
  for (const [method, path] of resourceReads) {
    const { guard, source } = routes.find(r => r.listener === 'public' && r.method === method && r.path === path)
      .registrations[0];
    const comparison = path.endsWith('/compare');
    const share = path.endsWith('/share');
    const hunts = path.includes('/account/hunts');
    const newListing = path.endsWith('/items/new');
    const expected = share ? 503 : guard.kind === 'logged-in' && !accountId ? newListing ? 403 : 303 : 200;
    const target = path.replace(/\{(?:lang|locale)\}/, locale).replace('{id}', ids[0]).replace('{listing}', unusedId);
    await test.step(`${method} ${path}`, async () => {
      const response = await request.fetch(comparison ? `${target}?ids=${ids.join(',')}` : target, {
        method, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
      });
      try {
        expect(response.status(), source).toBe(expected);
        if (expected === 303) expect(response.headers().location).toBe(`/${locale}/login`);
        if (hunts && expected === 200) {
          const html = await response.text();
          for (const id of comparison ? ids : [ids[0]]) expect(html.includes(id), 'owned hunt rendered').toBe(true);
        }
      } finally {
        await response.dispose();
      }
    });
  }
}
