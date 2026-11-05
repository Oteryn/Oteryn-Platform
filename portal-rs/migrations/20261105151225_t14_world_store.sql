-- Versions are immutable after publication; failed attempts keep their diagnostics.
CREATE TABLE world_versions (
    id uuid PRIMARY KEY,
    source_commit text NOT NULL CHECK (source_commit ~ '^([0-9a-f]{40}|[0-9a-f]{64})$'),
    status text NOT NULL DEFAULT 'importing' CHECK (status IN ('importing', 'ready', 'failed')),
    document_count bigint NOT NULL DEFAULT 0 CHECK (document_count >= 0),
    errors jsonb NOT NULL DEFAULT '[]'::jsonb CHECK (jsonb_typeof(errors) = 'array'),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE world_entities (
    version_id uuid NOT NULL REFERENCES world_versions(id),
    kind text NOT NULL CHECK (octet_length(kind) BETWEEN 1 AND 512),
    key text NOT NULL CHECK (octet_length(key) BETWEEN 1 AND 512),
    data jsonb NOT NULL,
    publishable boolean NOT NULL DEFAULT false,
    PRIMARY KEY (version_id, kind, key)
);

-- This permanent singleton is also the lock for switches and rollback.
CREATE TABLE world_active_version (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    version_id uuid REFERENCES world_versions(id),
    previous_version_id uuid REFERENCES world_versions(id),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (version_id IS DISTINCT FROM previous_version_id OR version_id IS NULL)
);
INSERT INTO world_active_version (singleton) VALUES (true);

-- Absence means hidden. Visibility is independent of versions and import data.
CREATE TABLE world_visibility (
    kind text NOT NULL CHECK (octet_length(kind) BETWEEN 1 AND 512),
    key text NOT NULL CHECK (octet_length(key) BETWEEN 1 AND 512),
    allowed boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (kind, key)
);
