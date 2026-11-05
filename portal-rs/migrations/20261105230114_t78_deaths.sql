-- Private, rebuildable G-11 projections. No game database access or mutations.
CREATE TABLE pd_t78_streams (
    source_authority text NOT NULL,
    account_id uuid NOT NULL,
    projection_epoch numeric(20,0) NOT NULL CHECK (projection_epoch > 0),
    recovery_generation numeric(20,0) NOT NULL CHECK (recovery_generation > 0),
    family text NOT NULL CHECK (family IN ('death_recap','pvp','skull')),
    accepted_through numeric(20,0) NOT NULL DEFAULT 0 CHECK (accepted_through >= 0),
    complete_through numeric(20,0) NOT NULL DEFAULT 0 CHECK (complete_through >= 0),
    observed_at_ms bigint,
    baseline_complete boolean NOT NULL DEFAULT false,
    PRIMARY KEY (source_authority, account_id, projection_epoch, recovery_generation, family)
);
CREATE TABLE pd_t78_records (
    source_authority text NOT NULL,
    account_id uuid NOT NULL,
    projection_epoch numeric(20,0) NOT NULL,
    recovery_generation numeric(20,0) NOT NULL,
    family text NOT NULL,
    source_revision numeric(20,0) NOT NULL CHECK (source_revision > 0),
    record_id uuid NOT NULL,
    character_id uuid NOT NULL,
    world_id uuid NOT NULL,
    entity_key text NOT NULL,
    digest bytea NOT NULL CHECK (octet_length(digest) = 32),
    occurred_at_ms bigint NOT NULL,
    payload jsonb,
    PRIMARY KEY (source_authority, record_id),
    UNIQUE (source_authority, account_id, projection_epoch, recovery_generation, family, source_revision),
    FOREIGN KEY (source_authority, account_id, projection_epoch, recovery_generation, family)
        REFERENCES pd_t78_streams ON DELETE CASCADE
);
CREATE INDEX pd_t78_subject ON pd_t78_records (account_id, character_id, family, source_revision DESC);
