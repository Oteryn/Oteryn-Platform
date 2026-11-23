import { createHash, randomUUID } from 'node:crypto';
import { mkdir, rm, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { test, expect } from '../fixtures.mjs';
import { pool } from '../helpers.mjs';
import { messages } from '../public-depth-helpers.mjs';
import { seededStaff } from '../staff-rbac-helpers.mjs';
import { expectUnpublishedCatalog } from '../catalog-setup.mjs';

const digest = bytes => createHash('sha256').update(bytes).digest('hex');
// Synthetic one-pixel lossless WebP for the existing tile and sprite readers.
const pixel = Buffer.from('UklGRh4AAABXRUJQVlA4TBEAAAAvAAAAAAdQrVLUqP+BiOh/AAA=', 'base64');

test('a public quest item resolves to its NPC offer, drop and Atlas location', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const t = messages('crates/atlas/src/delivery', lang);
  const version = randomUUID();
  const commit = digest(version).slice(0, 40);
  const key = name => `oteryn:${name}.${version}`;
  const item = key('item');
  const creature = key('creature');
  const hidden = key('hidden');
  const npc = key('npc');
  const currency = `${item}.currency`;
  const loot = key('loot');
  const delivery = `/${lang}/atlas/delivery`;
  const tileRoot = path.join(process.env.OTERYN_ATLAS_DIRECTORY, commit);
  const spriteRoot = path.join(process.env.OTERYN_STATIC_DIR, 'world-sprites', commit);
  const sha = digest(pixel);
  const file = `item-7-0-0-0-${sha}.webp`;
  const spriteManifest = JSON.stringify({
    format: 1, game_commit: commit, sources: {},
    thumbnails: [{ request: { kind: 'item', appearance_id: 7, pattern: null },
      pattern: [0, 0, 0], file, sha256: sha, width: 1, height: 1 }],
  });
  const manifestHash = digest(spriteManifest);
  const offer = { direction: 'npc_sell_offer', npc, item, currency, unit_price: 27, item_subtype: -1 };
  const records = [
    ['Item', item, {}, true], ['Item', currency, {}, true],
    ['Quest', key('quest'), { stages: [{ key: 'collect', kind: 'collect', item, count: 2 }] }, true],
    ['Creature', creature, { loot, level: 2 }, true],
    ['Creature', hidden, { loot, level: 2 }, false],
    ['Loot', loot, { algorithm: 'IndependentBernoulliPpm',
      entries: [{ item, probability_ppm: 250000, min_count: 1, max_count: 1 }] }, true],
    ['Spawn.Source', key('spawn'), { points: [{ creature, cell: { x: 10, y: 20, floor: 7 },
      respawn_ms: 1000 }] }, true],
    ['NPC', npc, {}, true], ['NPC', hidden, {}, false],
    ['NPC.ShopOffer', key('offer'), offer, true],
    ['NPC.ShopOffer', `${key('offer')}.hidden`, { ...offer, npc: hidden, unit_price: 918273 }, true],
    ['Item.Sprite', item, { appearance_id: 7, manifest_sha256: manifestHash }, true],
  ];
  const db = pool();
  const client = await db.connect();
  let previous;
  try {
    await mkdir(path.join(tileRoot, '7/0/0'), { recursive: true });
    await writeFile(path.join(tileRoot, '7/0/0/0.webp'), pixel);
    await writeFile(path.join(tileRoot, '7/manifest.json'), JSON.stringify({
      format_version: 1, world_version: commit, floor: 7, tile_size: 256,
      bounds: { min_x: 0, min_y: 0, max_x: 256, max_y: 256 },
      levels: [{ z: 0, width: 256, height: 256, columns: 1, rows: 1 }],
      tiles: [{ path: '0/0/0.webp', sha256: sha }],
    }));
    await mkdir(path.join(spriteRoot, manifestHash), { recursive: true });
    await writeFile(path.join(spriteRoot, manifestHash, file), pixel);
    await writeFile(path.join(spriteRoot, manifestHash, 'manifest.json'), spriteManifest);

    await client.query('BEGIN');
    previous = (await client.query('SELECT version_id FROM world_active_version WHERE singleton FOR UPDATE')).rows[0].version_id;
    await client.query("INSERT INTO world_versions(id,source_commit,status,document_count) VALUES ($1,$2,'ready',$3)",
      [version, commit, records.length]);
    for (const [kind, key, body, allowed] of records) {
      await client.query('INSERT INTO world_entities(version_id,kind,key,data,publishable) VALUES ($1,$2,$3,$4,true)',
        [version, kind, key, body]);
      await client.query('INSERT INTO world_visibility(kind,key,allowed) VALUES ($1,$2,$3)', [kind, key, allowed]);
    }
    await client.query('UPDATE world_active_version SET version_id=$1 WHERE singleton', [version]);
    await client.query('COMMIT');

    await page.goto(`/${lang}/news`);
    if (info.project.use.viewport.width <= 960) await page.locator('details.om-fallback-menu > summary').click();
    const nav = page.locator('#portal-navigation');
    const entry = nav.locator(`a[href="${delivery}"]`);
    await nav.locator('details.om-section').filter({ has: page.locator(`a[href="${delivery}"]`) }).locator('summary').click();
    await entry.click();
    await page.getByRole('link', { name: item, exact: true }).click();
    await expect(page).toHaveURL(url => url.searchParams.get('version') === version && url.searchParams.get('count') === '2');
    await expect(page.getByRole('heading', { name: item, exact: true })).toBeVisible();
    const shops = page.locator('main section').filter({ has: page.getByRole('heading', { name: t.shops, exact: true }) });
    await expect(shops).toContainText(npc);
    await expect(shops).toContainText(`27 ${currency}`);
    await expect(page.getByRole('link', { name: creature, exact: true })).toBeVisible();
    await expect(page.locator('main')).not.toContainText(hidden);
    await expect(shops).not.toContainText('918273');
    await expect(page.getByRole('img', { name: t.sprite })).toHaveJSProperty('naturalWidth', 1);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
    const map = page.getByRole('link', { name: `${t.map} (10, 20, 7)`, exact: true });
    await map.click();
    await expect(page.locator('canvas')).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
    await expect(page).toHaveURL(url => url.searchParams.get('version') === version && url.searchParams.get('x') === '10');
    expect((await page.request.get(`/atlas/tiles/${commit}/7/0/0/0.webp`)).status()).toBe(200);
    expect((await page.request.get(`${delivery}?item=oteryn%3Aitem%2Fbad`)).status()).toBe(400);
    expect((await page.request.get(`${delivery}?version=${randomUUID()}`)).status()).toBe(404);
  } finally {
    try {
      // Also releases the pointer lock when setup failed inside its transaction.
      await client.query('ROLLBACK');
      await client.query('BEGIN');
      await client.query('UPDATE world_active_version SET version_id=$1 WHERE version_id=$2', [previous ?? null, version]);
      await client.query('DELETE FROM world_entities WHERE version_id=$1', [version]);
      await client.query('DELETE FROM world_versions WHERE id=$1', [version]);
      for (const [kind, key] of records) {
        await client.query('DELETE FROM world_visibility WHERE kind=$1 AND key=$2', [kind, key]);
      }
      await client.query('COMMIT');
    } finally {
      client.release();
      await db.end();
      await Promise.all([tileRoot, spriteRoot].map(dir => rm(dir, { recursive: true, force: true })));
    }
  }
});

test('staff can inspect the pinned native catalog status from the staff home', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const labels = messages('templates/admin/world', lang);
  const state = page.locator('section[aria-labelledby="native-catalog"]');
  await expectUnpublishedCatalog();
  await seededStaff(page, lang, 'platform_admin');
  expect((await page.goto('/admin')).status()).toBe(200);
  await page.locator('main nav a[href="/admin/world"]').click();
  await expect(page).toHaveURL(/\/admin\/world$/);
  await expect(state.getByRole('heading')).toHaveText(labels.native_catalog);
  await expect(state).toContainText(labels.native_pending_contract);
  if (process.env.PORTAL_E2E_CATALOG_EXPECT_UNAVAILABLE !== '1') {
    await expect(state).toContainText(labels.native_shop_records);
  }
  await expect(state).toContainText(labels.native_unavailable);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
});
