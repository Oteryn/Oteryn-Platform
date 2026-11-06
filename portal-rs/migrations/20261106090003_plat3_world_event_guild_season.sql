-- Additive T94 projection. Roll back by disabling flags; existing data is untouched.
-- NUMERIC preserves the contract's complete uint64 counter domain.
CREATE TABLE league_subjects (
    authority text NOT NULL,
    family text NOT NULL CHECK (family IN ('world_event_outcome','guild_season')),
    world_id uuid NOT NULL,
    subject_id uuid NOT NULL,
    epoch numeric(20,0) NOT NULL CHECK (epoch > 0),
    revision numeric(20,0) NOT NULL CHECK (revision > 0),
    recovery_generation numeric(20,0) NOT NULL CHECK (recovery_generation > 0),
    wire bytea NOT NULL,
    content_hash text NOT NULL CHECK (length(content_hash)=64),
    record_hash text NOT NULL CHECK (length(record_hash)=64),
    final_state boolean NOT NULL DEFAULT false,
    finalized_at bigint,
    quarantined boolean NOT NULL DEFAULT false,
    PRIMARY KEY(authority,family,world_id,subject_id)
);
CREATE TABLE league_receipts (
    envelope_id uuid PRIMARY KEY,
    transaction_id uuid NOT NULL,
    ordinal integer NOT NULL CHECK (ordinal > 0),
    event_count integer NOT NULL CHECK (event_count >= ordinal),
    fingerprint text NOT NULL CHECK (length(fingerprint)=64),
    UNIQUE(transaction_id,ordinal)
);
CREATE TABLE league_audit (
    envelope_id uuid PRIMARY KEY REFERENCES league_receipts(envelope_id),
    authority text NOT NULL,
    family text NOT NULL,
    world_id uuid NOT NULL,
    subject_id uuid NOT NULL,
    epoch numeric(20,0) NOT NULL,
    revision numeric(20,0) NOT NULL,
    recovery_generation numeric(20,0) NOT NULL,
    wire bytea NOT NULL,
    payload bytea NOT NULL,
    signature bytea NOT NULL,
    received_at bigint NOT NULL,
    occurred_at_unix_ms bigint NOT NULL CHECK (occurred_at_unix_ms > 0),
    UNIQUE(authority,family,world_id,subject_id,epoch,revision)
);
-- Receipts/final-state controls intentionally outlive P90D payload retention.
CREATE INDEX league_audit_retention ON league_audit(occurred_at_unix_ms);
CREATE TABLE league_occurrences (
    authority text NOT NULL,
    world_id uuid NOT NULL,
    occurrence_id uuid NOT NULL,
    subject_id uuid NOT NULL,
    PRIMARY KEY(authority,world_id,occurrence_id)
);
