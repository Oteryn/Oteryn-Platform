-- Rebuildable private G-10 projections. No game database or balance authority.
CREATE TABLE pd_t76_streams (
    stream_key bytea PRIMARY KEY CHECK (octet_length(stream_key)=32),
    source_authority text NOT NULL,
    account_id uuid NOT NULL,
    world_id uuid NOT NULL,
    character_id uuid,
    family text NOT NULL CHECK (family IN ('ITEMS','BANK','MARKET','PLAYER_TRADE')),
    generation numeric(20,0) NOT NULL CHECK (generation>0),
    fenced boolean NOT NULL DEFAULT false,
    cursor numeric(20,0) NOT NULL DEFAULT 0 CHECK (cursor>=0),
    retention_floor numeric(20,0) NOT NULL DEFAULT 0,
    coverage_start bigint NOT NULL,
    staging_start numeric(20,0),
    staging_at bigint,
    snapshot bytea,
    snapshot_at bigint,
    last_bank_entry text,
    last_bank_balance text,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX pd_t76_owner ON pd_t76_streams(account_id,world_id,character_id,family);
CREATE TABLE pd_t76_records (
    stream_key bytea NOT NULL REFERENCES pd_t76_streams(stream_key) ON DELETE CASCADE,
    source_revision numeric(20,0) NOT NULL,
    source_authority text NOT NULL,
    generation numeric(20,0) NOT NULL,
    record_id uuid NOT NULL,
    digest bytea NOT NULL CHECK (octet_length(digest)=32),
    canonical_bytes bytea,
    occurred_at bigint NOT NULL,
    PRIMARY KEY(stream_key,source_revision),
    UNIQUE(source_authority,generation,record_id)
);
CREATE TABLE pd_t76_events (
    stream_key bytea NOT NULL REFERENCES pd_t76_streams(stream_key) ON DELETE CASCADE,
    event_id uuid NOT NULL,
    business_id uuid NOT NULL,
    payload_digest bytea NOT NULL,
    canonical_bytes bytea,
    occurred_at bigint NOT NULL,
    PRIMARY KEY(stream_key,event_id),
    UNIQUE(stream_key,business_id)
);
