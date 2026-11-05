-- Non-payload ordering fences survive retention erasure.
CREATE TABLE stats_world_kills_v2 (
    source_authority text NOT NULL,
    world_id uuid NOT NULL,
    day_start bigint NOT NULL CHECK (day_start > 0 AND day_start % 86400 = 0),
    epoch numeric(20,0) NOT NULL CHECK (epoch > 0),
    revision numeric(20,0) NOT NULL CHECK (revision > 0),
    content_digest bytea NOT NULL CHECK (octet_length(content_digest) = 32),
    source_observed_at bigint NOT NULL,
    payload jsonb,
    retired boolean NOT NULL DEFAULT false,
    PRIMARY KEY (source_authority, world_id, day_start),
    CHECK (NOT retired OR payload IS NULL)
);
CREATE INDEX stats_world_kills_v2_retention ON stats_world_kills_v2(day_start)
    WHERE payload IS NOT NULL;
