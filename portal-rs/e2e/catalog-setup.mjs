import { expect } from '@playwright/test';
import { pool } from './helpers.mjs';

// The launcher's pinned CATALOG_EXPORT_V1 negative artifact. See
// crates/world/tests/fixtures/catalog-669d5af5/SOURCE.md for its provenance.
const unsupportedArtifact = '4fe8ce4488c21396b588dc6b04179cde93c7a9b22b87427b6e4af3cda6e60d7d';

export async function expectUnpublishedCatalog() {
  const db = pool();
  try {
    if (process.env.PORTAL_E2E_CATALOG_EXPECT_UNAVAILABLE === '1') {
      const { rows } = await db.query(
        `SELECT (SELECT count(*)::int FROM world_catalog_candidates) AS candidate_count,
                snapshot_id
         FROM world_catalog_publication WHERE singleton`,
      );
      expect(rows, 'failed catalog transport leaves no candidate or public snapshot')
        .toEqual([{ candidate_count: 0, snapshot_id: null }]);
      return;
    }

    await expect.poll(async () => {
      const { rows } = await db.query(
        `SELECT candidate.status, publication.snapshot_id
         FROM world_catalog_candidates candidate
         JOIN world_catalog_publication publication
           ON publication.configured_snapshot_id = candidate.snapshot_id
         WHERE candidate.artifact_sha256 = $1 AND publication.singleton`,
        [unsupportedArtifact],
      );
      return rows;
    }, { message: 'pinned native candidate is retained without a public snapshot' })
      .toEqual([{ status: 'inactive', snapshot_id: null }]);
  } finally {
    await db.end();
  }
}
