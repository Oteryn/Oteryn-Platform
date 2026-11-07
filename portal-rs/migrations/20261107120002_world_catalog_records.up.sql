-- Rebuildable, inactive evidence, scoped by the immutable verified snapshot.
-- No public entity/version or visibility grant is created by this cache.
CREATE TABLE world_catalog_records (
    snapshot_id text NOT NULL REFERENCES world_catalog_candidates(snapshot_id),
    record_class text NOT NULL CHECK (record_class IN ('entity','relation','tombstone')),
    record_key text NOT NULL CHECK (octet_length(record_key) BETWEEN 1 AND 2048),
    capability_id text NOT NULL CHECK (capability_id IN ('item','creature','creature_loot','npc','npc_shop','spell','quest','achievement')),
    record bytea NOT NULL CHECK (octet_length(record) BETWEEN 1 AND 268435456),
    PRIMARY KEY (snapshot_id, record_class, record_key)
);
CREATE INDEX world_catalog_records_capability ON world_catalog_records(snapshot_id, capability_id, record_class);
