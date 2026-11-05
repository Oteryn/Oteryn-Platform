-- Evidence is immutable; canonical response bytes are retained without JSON reserialization.
CREATE TABLE native_evidence (
    namespace_hash text NOT NULL CHECK (namespace_hash ~ '^[0-9a-f]{64}$'),
    source_revision bigint NOT NULL CHECK (source_revision > 0),
    response_json text NOT NULL CHECK (octet_length(response_json) <= 8192),
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (namespace_hash, source_revision)
);
CREATE TABLE native_evidence_witness_stores (
    id smallint PRIMARY KEY CHECK (id = 1),
    store_id text NOT NULL CHECK (store_id ~ '^[0-9a-f]{64}$'),
    created_at timestamptz NOT NULL DEFAULT now()
);
-- Only the latest publication per canonical scope is kept. Assignment writers belong to T20.
CREATE TABLE native_runtime_status_reports (
    world_id uuid NOT NULL,
    channel_id uuid NOT NULL,
    assignment_epoch numeric(20, 0) NOT NULL CHECK (assignment_epoch BETWEEN 1 AND 18446744073709551615),
    scope_ownership_generation numeric(20, 0) NOT NULL CHECK (scope_ownership_generation BETWEEN 1 AND 18446744073709551615),
    source_revision numeric(20, 0) NOT NULL CHECK (source_revision BETWEEN 1 AND 18446744073709551615),
    node_identity text NOT NULL,
    content_json text NOT NULL CHECK (octet_length(content_json) <= 2048),
    observed_at bigint NOT NULL CHECK (observed_at >= 0),
    invalid boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (world_id, channel_id)
);
