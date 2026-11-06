-- Only identifiers and cutover bookkeeping belong here, never source payloads.
CREATE TABLE legacy_import_map (
    source_table text NOT NULL CHECK (source_table ~ '^[a-z][a-z0-9_]*$'),
    legacy_id text NOT NULL CHECK (length(legacy_id) BETWEEN 1 AND 256),
    target_id uuid NOT NULL,
    area text NOT NULL CHECK (area IN ('identity','content','commerce','support','characters')),
    imported_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (source_table, legacy_id)
);
