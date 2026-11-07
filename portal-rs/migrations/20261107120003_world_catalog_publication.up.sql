-- A dedicated wiki pointer: catalogue publication must not imply atlas/map readiness.
CREATE TABLE world_catalog_versions (
    snapshot_id text PRIMARY KEY REFERENCES world_catalog_candidates(snapshot_id),
    version_id uuid NOT NULL UNIQUE REFERENCES world_versions(id),
    capabilities text[] NOT NULL CHECK (cardinality(capabilities) BETWEEN 1 AND 7 AND
        capabilities <@ ARRAY['achievement','creature','creature_loot','item','npc','quest','spell']::text[])
);
CREATE TABLE world_catalog_publication (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    snapshot_id text REFERENCES world_catalog_versions(snapshot_id)
);
INSERT INTO world_catalog_publication(singleton) VALUES (true);
