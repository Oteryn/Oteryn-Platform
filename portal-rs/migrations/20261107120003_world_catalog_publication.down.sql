-- Remove only this catalogue's derived versions while provenance still identifies
-- them. Keep legacy versions, approvals and any legacy pointer value untouched.
UPDATE world_active_version SET version_id = NULL
WHERE version_id IN (SELECT version_id FROM world_catalog_versions);
UPDATE world_active_version SET previous_version_id = NULL
WHERE previous_version_id IN (SELECT version_id FROM world_catalog_versions);
DELETE FROM world_entities e USING world_catalog_versions c
WHERE e.version_id = c.version_id;
-- Catalogue-dependent puzzle rows must not retain otherwise-orphaned versions.
-- The DDL lock/transaction preserves the immutable guard for all legacy puzzles.
ALTER TABLE puzzles_daily DISABLE TRIGGER puzzles_daily_immutable;
DELETE FROM puzzles_answers a USING puzzles_daily p, world_catalog_versions c
WHERE a.puzzle_id = p.id AND p.world_version = c.version_id;
DELETE FROM puzzles_daily p USING world_catalog_versions c
WHERE p.world_version = c.version_id;
ALTER TABLE puzzles_daily ENABLE TRIGGER puzzles_daily_immutable;
DELETE FROM world_catalog_publication;
WITH removed AS (DELETE FROM world_catalog_versions RETURNING version_id)
DELETE FROM world_versions WHERE id IN (SELECT version_id FROM removed);
DROP TABLE world_catalog_publication;
DROP TABLE world_catalog_versions;
