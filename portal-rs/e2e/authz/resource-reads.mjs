import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';

const unusedId = '019a541b-a000-7000-8000-000000000001';
export const resourceReads = [
  ['GET', '/{lang}/account/hunts/{id}'],
  ['GET', '/{lang}/account/hunts/compare'],
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
                'duration_seconds',60,'places','{"place:fixture":60}'::jsonb,
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

async function privateListing(accountId) {
  const [listing] = await query(`
    WITH owner AS (
      SELECT COALESCE($1::uuid,(SELECT id FROM accounts WHERE email='bazaar-escrow@local.test')) AS id
    ), listing AS (
      INSERT INTO bazaar_listings(id,owner,kind,form,currency,price,accept_offers,state,ends_at)
      SELECT uuidv7(),id,'item','auction','coins',1,false,'cancelled',now()+interval '1 day' FROM owner
      RETURNING id
    )
    INSERT INTO bazaar_authority_assets
      (listing_id,request_id,world_id,seller_character_id,asset_id,expected_revision,session_hash,
       quantity,publication_consent,input_binding)
    SELECT listing.id,uuidv7(),world.id,uuidv7(),uuidv7(),1,sha256('local fixture'::bytea),1,true,'{}'
      FROM listing CROSS JOIN (SELECT id FROM world_status ORDER BY id LIMIT 1) world
    RETURNING listing_id
  `, [accountId ?? null]);
  return listing.listing_id;
}

export async function exerciseResourceReads(request, accountId, locale, token, routes) {
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  const ids = accountId ? await seedHunts(accountId) : [unusedId, unusedId];
  const listing = await privateListing(accountId);
  for (const [method, path] of resourceReads) {
    const { guard, source } = routes.find(r => r.listener === 'public' && r.method === method && r.path === path)
      .registrations[0];
    const comparison = path.endsWith('/compare');
    const share = path.endsWith('/share');
    const hunts = path.includes('/account/hunts');
    const newListing = path.endsWith('/items/new');
    let expected = 200;
    if (share) expected = 503;
    else if (!accountId && hunts) expected = 303;
    else if (!accountId) expected = newListing ? 403 : 404;
    const target = path.replace(/\{(?:lang|locale)\}/, locale).replace('{id}', ids[0]).replace('{listing}', listing);
    await test.step(`${method} ${path}`, async () => {
      const response = await request.fetch(comparison ? `${target}?ids=${ids.join(',')}` : target, {
        method, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
        ...(method === 'POST' && { form: { _csrf: token } }),
      });
      try {
        expect(guard.kind).toBe(hunts && !share || newListing ? 'logged-in' : 'anonymous');
        const policy = expected === 404 ? 'crates/bazaar/src/game/runtime/browser.rs:339' : source;
        expect(response.status(), policy).toBe(expected);
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
