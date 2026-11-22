import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';

const fixture = '019a541b-a000-7000-8000-000000000001';
const cases = [
  { method: 'GET', path: '/{lang}/collection/{character}' },
  { method: 'GET', path: '/{lang}/collection/{character}/cards/{id}/image' },
  { method: 'POST', path: '/{lang}/collection/{character}/decks', form: { name: 'Local authorization deck', first: fixture } },
  { method: 'POST', path: '/{lang}/collection/decks/{id}/battle', form: {} },
  { method: 'GET', path: '/{lang}/collection/battles/{id}' },
  { method: 'POST', path: '/{lang}/collection/battles/{id}', form: { expected: '0', action: 'guard' } },
  { method: 'POST', path: '/{lang}/collection/battles/{id}/forfeit', form: {}, cleanup: 'battle' },
  { method: 'POST', path: '/{lang}/collection/decks/{id}/delete', form: {}, cleanup: 'deck' },
];
export const collectionRoutes = cases.map(({ method, path }) => [method, path]);

async function resources(accountId) {
  if (!accountId) return { character: fixture, deck: fixture, battle: fixture };
  const [deck] = await query(`
    INSERT INTO collection_decks(id,account_id,character_id,name,cards)
    VALUES(uuidv7(),$1,uuidv7(),'Local authorization deck',ARRAY[$2::uuid])
    RETURNING id,character_id
  `, [accountId, fixture]);
  const fighters = [{ card: fixture, health: 30 }];
  const state = { ruleset: 1, turn: 0, player: fighters, enemy: fighters, outcome: 'active', seed: 1, last_action: null };
  const [battle] = await query(`
    INSERT INTO collection_battles(id,account_id,character_id,deck_id,state)
    VALUES(uuidv7(),$1,$2,$3,$4) RETURNING id
  `, [accountId, deck.character_id, deck.id, state]);
  return { character: deck.character_id, deck: deck.id, battle: battle.id };
}

export async function exerciseCollection(request, locale, token, accountId, routes, snapshot, verifyRefusal) {
  const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
  const { character, deck, battle } = await resources(accountId);
  for (const { method, path, form, cleanup } of cases) {
    const { guard } = routes.find(r => r.listener === 'public' && r.method === method && r.path === path)
      .registrations[0];
    expect(guard.kind, path).toBe('logged-in');
    const id = path.includes('/decks/{id}') ? deck : path.includes('/battles/{id}') ? battle : fixture;
    const target = path.replace('{lang}', locale).replace('{character}', character).replace('{id}', id);
    await test.step(`${method} ${path}`, async () => {
      const before = form && !accountId ? await snapshot() : null;
      const response = await request.fetch(target, {
        method, headers: { origin, 'x-csrf-token': token }, maxRedirects: 0,
        ...(form && { form: { ...form, _csrf: token } }),
      });
      try {
        // The configured Portal has no trusted T72 ownership provider.
        const source = accountId && !cleanup ? 'crates/minigames/src/collection/http.rs:120'
          : 'crates/minigames/src/collection/http.rs:145';
        expect(response.status(), source).toBe(accountId && !cleanup ? 503 : 303);
        if (before) await verifyRefusal(before);
        if (!accountId) expect(response.headers().location).toBe(`/${locale}/login`);
        if (accountId && cleanup === 'battle') {
          const [row] = await query("SELECT state->>'outcome' AS outcome FROM collection_battles WHERE id=$1", [battle]);
          expect(row.outcome).toBe('defeat');
        }
        if (accountId && cleanup === 'deck') {
          expect((await query('SELECT 1 FROM collection_decks WHERE id=$1', [deck])).length).toBe(0);
        }
      } finally {
        await response.dispose();
      }
    });
  }
}
