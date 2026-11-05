-- Local projection only: account identity remains owned by the identity module.
CREATE TABLE account_character_feed (
    id smallint PRIMARY KEY CHECK (id = 1),
    highest_epoch text CHECK (highest_epoch ~ '^[1-9][0-9]{0,19}$' AND highest_epoch::numeric <= 18446744073709551615),
    watermark_epoch text,
    source_authority text,
    complete_through bigint CHECK (complete_through >= 0),
    observed_at bigint CHECK (observed_at >= complete_through),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK ((watermark_epoch IS NULL AND source_authority IS NULL AND complete_through IS NULL AND observed_at IS NULL)
        OR (watermark_epoch = highest_epoch AND source_authority IS NOT NULL AND complete_through IS NOT NULL AND observed_at IS NOT NULL))
);
INSERT INTO account_character_feed (id) VALUES (1);

CREATE TABLE account_character_snapshots (
    account_id uuid PRIMARY KEY,
    source_authority text NOT NULL,
    projection_epoch text NOT NULL CHECK (projection_epoch ~ '^[1-9][0-9]{0,19}$' AND projection_epoch::numeric <= 18446744073709551615),
    projection_revision text NOT NULL CHECK (projection_revision ~ '^[1-9][0-9]{0,19}$' AND projection_revision::numeric <= 18446744073709551615),
    source_observed_at bigint NOT NULL CHECK (source_observed_at >= 0),
    content_digest bytea NOT NULL CHECK (octet_length(content_digest) = 32),
    invalid boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE account_characters (
    account_id uuid NOT NULL,
    character_id uuid NOT NULL,
    world_id uuid NOT NULL,
    name text NOT NULL CHECK (octet_length(name) BETWEEN 1 AND 64),
    availability text NOT NULL CHECK (availability IN ('AVAILABLE', 'UNAVAILABLE')),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, character_id)
);

-- Shared across serving processes, one bounded bucket per trusted publisher.
CREATE TABLE account_character_publisher_limits (
    peer_key bytea PRIMARY KEY CHECK (octet_length(peer_key) = 32),
    reset_at bigint NOT NULL,
    requests integer NOT NULL CHECK (requests > 0)
);
