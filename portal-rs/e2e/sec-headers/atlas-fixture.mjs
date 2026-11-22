import { createHash, randomBytes } from 'node:crypto';
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { pool } from '../helpers.mjs';

// The Atlas viewer accepts a ready WorldStore version and an on-disk tile
// manifest. Use the same fixture contract as crates/atlas/src/web_tests.rs.
export async function withAtlasFixture(visit) {
  const db = pool();
  const version = randomBytes(20).toString('hex');
  const directory = join(process.env.OTERYN_ATLAS_DIRECTORY, version);
  let id;
  let previous;
  try {
    const tile = await readFile(new URL('../../static/thais/atlas.webp', import.meta.url));
    const relative = '0/0/0.webp';
    await mkdir(join(directory, '7/0/0'), { recursive: true, mode: 0o700 });
    await writeFile(join(directory, '7', relative), tile);
    await writeFile(join(directory, '7/manifest.json'), JSON.stringify({
      format_version: 1,
      world_version: version,
      floor: 7,
      tile_size: 256,
      bounds: { min_x: 100, min_y: 200, max_x: 356, max_y: 456 },
      levels: [{ z: 0, width: 256, height: 256, columns: 1, rows: 1 }],
      tiles: [{ path: relative, sha256: createHash('sha256').update(tile).digest('hex') }],
    }));
    ({
      rows: [{ version_id: previous }],
    } = await db.query('SELECT version_id FROM world_active_version WHERE singleton'));
    ({
      rows: [{ id }],
    } = await db.query("INSERT INTO world_versions(id,source_commit,status) VALUES (uuidv7(),$1,'ready') RETURNING id", [version]));
    const activated = await db.query(
      'UPDATE world_active_version SET version_id=$1 WHERE singleton AND version_id IS NOT DISTINCT FROM $2',
      [id, previous],
    );
    if (activated.rowCount !== 1) throw new Error('World version changed while preparing the Atlas fixture');
    await visit(version);
  } finally {
    try {
      if (id) {
        await db.query('UPDATE world_active_version SET version_id=$1 WHERE singleton AND version_id=$2', [previous, id]);
        await db.query('DELETE FROM world_versions WHERE id=$1', [id]);
      }
    } finally {
      await db.end();
      await rm(directory, { recursive: true, force: true });
    }
  }
}
