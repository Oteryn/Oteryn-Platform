-- Offline v1 artifacts are evidence only. No activation or public projection exists
-- until Game capability payload and authenticated publication gates are accepted.
CREATE TABLE world_catalog_candidates (
    snapshot_id text PRIMARY KEY CHECK (snapshot_id ~ '^sha256:[0-9a-f]{64}$'),
    artifact_sha256 text NOT NULL CHECK (artifact_sha256 ~ '^[0-9a-f]{64}$'),
    source_revision text NOT NULL CHECK (source_revision ~ '^[0-9a-f]{40}$'),
    authority_epoch text NOT NULL,
    ruleset_id text NOT NULL,
    content_profile_id text NOT NULL,
    status text NOT NULL DEFAULT 'inactive' CHECK (status = 'inactive'),
    -- bytea retains the exact canonical export, including legal escaped NULs
    -- which PostgreSQL JSONB cannot represent. Only the verifier admits rows.
    artifact bytea NOT NULL CHECK (octet_length(artifact) BETWEEN 1 AND 268435456),
    received_at timestamptz NOT NULL DEFAULT now()
);
