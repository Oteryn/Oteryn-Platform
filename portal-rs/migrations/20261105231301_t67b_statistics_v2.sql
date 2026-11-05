-- Owner-only payloads deliberately never enter public daily statistics or snapshots.
-- Shared public_projection_state retains the public feed's monotonic epoch floor.
CREATE TABLE stats_v2_subjects (
    source_authority text NOT NULL,
    family text NOT NULL CHECK (family IN ('character_online_sessions','world_daily_kills')),
    subject text NOT NULL,
    epoch numeric(20,0) NOT NULL CHECK (epoch > 0),
    revision numeric(20,0) NOT NULL CHECK (revision > 0),
    observed_at bigint NOT NULL CHECK (observed_at > 0),
    content_digest bytea NOT NULL CHECK (octet_length(content_digest) = 32),
    payload jsonb,
    expires_at bigint NOT NULL CHECK (expires_at > 0),
    -- Payload erasure is irreversible for this subject for its retained fence.
    erased boolean NOT NULL DEFAULT false,
    open_unresolved boolean NOT NULL DEFAULT false,
    closed boolean NOT NULL DEFAULT false,
    PRIMARY KEY (source_authority, family, subject),
    CHECK (NOT erased OR payload IS NULL)
);
CREATE INDEX stats_v2_retention ON stats_v2_subjects (expires_at) WHERE payload IS NOT NULL;
CREATE TABLE stats_v2_watermarks (
    source_authority text NOT NULL,
    family text NOT NULL CHECK (family IN ('character_online_sessions','world_daily_kills')),
    epoch numeric(20,0) NOT NULL CHECK (epoch > 0),
    complete_through bigint NOT NULL CHECK (complete_through >= 0),
    observed_at bigint NOT NULL CHECK (observed_at >= complete_through),
    PRIMARY KEY (source_authority, family)
);
