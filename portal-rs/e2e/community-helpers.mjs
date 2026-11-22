import { spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { readFileSync } from 'node:fs';
import { expect } from './fixtures.mjs';
import { fixtureDatabaseEnv, password, pool, waitForWikiBudget } from './helpers.mjs';

export function strings(directory, lang) {
  const file = new URL(`../${directory}/${lang}.ftl`, import.meta.url);
  return Object.fromEntries(
    [...readFileSync(file, 'utf8').matchAll(/^([\w-]+) = (.+)$/gm)].map(([, key, value]) => [key, value])
  );
}

export async function open(page, path, status = 200) {
  await waitForWikiBudget(path);
  const response = await page.goto(path);
  expect(response.status(), path).toBe(status);
  if (status === 503) expect(response.headers()['cache-control'], path).toContain('no-store');
  return response;
}

export async function follow(page, control, path, status = 200, query = {}) {
  await waitForWikiBudget(path);
  const navigation = page.waitForResponse(response => {
    const url = new URL(response.url());
    return response.request().isNavigationRequest() && url.pathname === path &&
      Object.entries(query).every(([key, value]) => url.searchParams.get(key) === value);
  });
  await control.click();
  const response = await navigation;
  expect(response.status(), response.url()).toBe(status);
  if (status === 503) expect(response.headers()['cache-control']).toContain('no-store');
  await page.waitForURL(url => url.href === response.url());
  return response;
}

// Seed only portal-owned editorial data in the harness's disposable database.
export async function withEvents(action) {
  const db = pool();
  const connection = await db.connect();
  let actor;
  try {
    await connection.query('BEGIN');
    actor = (await connection.query('SELECT uuidv7() AS id')).rows[0].id;
    await connection.query(`
      WITH fixtures AS (
        INSERT INTO events (id,status,starts_at,ends_at,created_by,updated_by,published_by)
        SELECT uuidv7(), CASE WHEN i = 202 THEN 'draft' ELSE 'published' END,
               now() + i * interval '1 minute', now() + interval '2 days',
               $1, $1, CASE WHEN i < 202 THEN $1::uuid END
        FROM generate_series(1,202) AS i RETURNING id, starts_at
      ), numbered AS (
        SELECT id, row_number() OVER (ORDER BY starts_at) AS n FROM fixtures
      )
      INSERT INTO event_translations(event_id,locale,title,slug,summary,body)
      SELECT id, locale, title || ' ' || n, 'community-e2e-' || n, summary, body
      FROM numbered CROSS JOIN (VALUES
        ('en','Community gathering','Meet at the plaza.','Bring a lantern.'),
        ('pl','Spotkanie społeczności','Spotkajmy się na placu.','Przynieś latarnię.')
      ) AS translations(locale,title,summary,body)
    `, [actor]);
    await connection.query('COMMIT');
    await action(async n => {
      await connection.query(
        "DELETE FROM events USING event_translations t WHERE events.id=t.event_id AND created_by=$1 AND t.slug=$2",
        [actor, `community-e2e-${n}`]
      );
    });
  } finally {
    try {
      await connection.query('ROLLBACK');
      if (actor) {
        await connection.query('DELETE FROM events WHERE created_by=$1', [actor]);
      }
    } finally {
      connection.release();
      await db.end();
    }
  }
}

export async function withEditorial(action) {
  const db = pool();
  let guide;
  try {
    guide = (await db.query(`
      WITH guide AS (
        INSERT INTO wiki_guides(id,status,published_at)
        VALUES(uuidv7(),'published',now()) RETURNING id
      )
      INSERT INTO wiki_revisions(id,guide_id,version,locale,title,slug,summary,markdown,editor,change_note)
      SELECT uuidv7(),id,1,locale,title,'community-e2e-guide',summary,markdown,id,'Community E2E'
      FROM guide CROSS JOIN (VALUES
        ('en','Lantern workshop','A guide to lanterns.','## Assembly\n\nFit the wick.'),
        ('pl','Warsztat latarni','Poradnik o latarniach.','## Montaż\n\nZałóż knot.')
      ) AS translations(locale,title,summary,markdown) RETURNING guide_id
    `)).rows[0].guide_id;
    await db.query(`
      WITH page AS (
        INSERT INTO pages(id,slug) VALUES(uuidv7(),'community-e2e-page') RETURNING id
      ), source AS (
        INSERT INTO editorial_pages(page_id,locale,title,body,published_at)
        SELECT id,'en','Plaza directions','Follow the eastern road.',now() FROM page
        RETURNING page_id,revision
      )
      INSERT INTO editorial_pages(page_id,locale,title,body,published_at,source_revision)
      SELECT page_id,'pl','Droga na plac','Idź drogą na wschód.',now(),revision FROM source
    `);
    await db.query(`
      INSERT INTO news(slug,title,body,published_at,title_pl,body_pl,translation_published_at,translation_source_updated_at)
      VALUES('community-e2e-news','Lantern festival','The gates open at dusk.',now(),
        'Festiwal latarni','Bramy otworzą się o zmierzchu.',now(),now())
    `);
    await action();
  } finally {
    try {
      await db.query("DELETE FROM news WHERE slug='community-e2e-news'");
      await db.query("DELETE FROM pages WHERE slug='community-e2e-page'");
      if (guide) {
        await db.query("UPDATE wiki_guides SET status='archived', published_at=NULL WHERE id=$1", [guide]);
      }
    } finally {
      await db.end();
    }
  }
}

export async function seedPlayer(locale) {
  const env = fixtureDatabaseEnv();
  const binary = process.env.PORTAL_E2E_BINARY ||
    fileURLToPath(new URL('../target/debug/oteryn-portal', import.meta.url));
  const account = {email: `community-${randomUUID()}@example.invalid`, password: password()};
  const seed = spawnSync(binary, ['seed', 'account', JSON.stringify({...account, locale})], {
    stdio: 'ignore',
    env,
    timeout: 30_000,
  });
  expect(seed.status, 'seed active player in disposable harness').toBe(0);
  return account;
}
