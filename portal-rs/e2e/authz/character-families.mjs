import { mirrorHead } from './head.mjs';
import { expect, test } from '@playwright/test';
import { query } from '../staff-rbac-helpers.mjs';

const name = 'Matrix Fixture';
const families = ['achievements', 'equipment', 'outfit'];
export const characterFamilies = ['', '/pl', '/en'].flatMap(prefix => families.map(family =>
  ['GET', `${prefix}/account/characters/{name}/${family}`]));

export async function exerciseCharacterFamilies(request, accountId, routes) {
  if (accountId) {
    const inserted = await query(`
      INSERT INTO account_characters(account_id,character_id,world_id,name,availability)
      SELECT $1,uuidv7(),id,$2,'AVAILABLE' FROM world_status ORDER BY id LIMIT 1
      RETURNING character_id
    `, [accountId, name]);
    expect(inserted.length, 'local character lookup candidate').toBe(1);
  }
  for (const [method, path] of characterFamilies) {
    const guard = routes.find(r => r.listener === 'public' && r.method === method && r.path === path)
      .registrations[0].guard;
    expect(guard.kind, path).toBe('logged-in');
    await test.step(`${method} ${path}`, async () => {
      const found = path.replace('{name}', encodeURIComponent(name));
      const response = await request.get(found, { maxRedirects: 0 });
      try {
        // The signed-in actor reaches ownership proof, whose fresh game feed is absent locally.
        const source = accountId ? 'crates/community/src/families/web.rs:428' : 'crates/web/src/chrome.rs:35';
        expect(response.status(), source).toBe(accountId ? 503 : 303);
        if (!accountId) expect(response.headers().location).toMatch(/^\/(pl|en)\/login$/);
        await mirrorHead(request, path, found, response);
      } finally {
        await response.dispose();
      }
    });
  }
}
