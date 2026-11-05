-- Private disposable projections. Authority evidence is supplied by the owning
-- projection/restore adapters, never inferred from these recipient partitions.
CREATE TABLE pd_streams (
    stream_key text PRIMARY KEY,
    authority text NOT NULL,
    account_id text NOT NULL,
    family text NOT NULL,
    progress boolean NOT NULL,
    retention_floor numeric(20,0) NOT NULL DEFAULT 0,
    last_record_id text,
    erased_ms bigint,
    highest_seen numeric(20,0) NOT NULL DEFAULT 0,
    stored_through numeric(20,0) NOT NULL DEFAULT 0,
    applied_through numeric(20,0) NOT NULL DEFAULT 0,
    published_through numeric(20,0) NOT NULL DEFAULT 0,
    observed_ms bigint NOT NULL DEFAULT 0,
    baseline_complete boolean NOT NULL DEFAULT false,
    watermark_digest bytea,
    erased boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    CHECK (0 <= applied_through AND applied_through <= stored_through AND stored_through <= highest_seen)
);
CREATE TABLE pd_records (
    stream_key text NOT NULL REFERENCES pd_streams(stream_key),
    source_revision numeric(20,0) NOT NULL,
    authority text NOT NULL,
    recovery_generation text NOT NULL,
    record_id text NOT NULL,
    entity_key text NOT NULL,
    character_id text,
    scope_key text NOT NULL,
    kind text NOT NULL,
    occurred_ms bigint NOT NULL,
    coverage_ms bigint NOT NULL,
    digest bytea NOT NULL,
    payload bytea,
    expires_ms bigint NOT NULL,
    PRIMARY KEY (stream_key, source_revision),
    UNIQUE (authority, recovery_generation, record_id)
);
CREATE INDEX pd_records_expiry ON pd_records(expires_ms) WHERE payload IS NOT NULL;
CREATE INDEX pd_records_entity ON pd_records(stream_key, entity_key, source_revision DESC);
-- Encrypted raw request archives are deliberately not retained. Canonical
-- payloads expire independently from the minimal replay digest/sequence fence.
CREATE TABLE pd_item_heads (
    stream_key text PRIMARY KEY REFERENCES pd_streams(stream_key),
    snapshot_id text NOT NULL,
    begin_revision numeric(20,0) NOT NULL,
    end_revision numeric(20,0) NOT NULL,
    occurred_ms bigint NOT NULL
);

-- Bounded, payload-free quarantine references for immutable binding conflicts.
CREATE TABLE pd_conflicts (
    reference bytea PRIMARY KEY,
    reason text NOT NULL CHECK (reason = 'IMMUTABLE_BINDING'),
    expires_ms bigint NOT NULL
);
