-- Immutable season definitions are published only through the authorized service.
CREATE TABLE pass_seasons (
    id uuid PRIMARY KEY,
    starts_at timestamptz NOT NULL,
    ends_at timestamptz NOT NULL,
    price bigint NOT NULL CHECK (price > 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (ends_at - starts_at BETWEEN interval '42 days' AND interval '56 days'),
    EXCLUDE USING gist (tstzrange(starts_at, ends_at, '[)') WITH &&)
);
CREATE TABLE pass_challenges (
    id uuid PRIMARY KEY,
    season_id uuid NOT NULL REFERENCES pass_seasons(id),
    source text NOT NULL CHECK (source IN ('journal','portal')),
    metric text NOT NULL CHECK (length(metric) BETWEEN 1 AND 64),
    target bigint NOT NULL CHECK (target > 0),
    xp bigint NOT NULL CHECK (xp > 0),
    UNIQUE (season_id, id)
);
CREATE TABLE pass_rewards (
    id uuid PRIMARY KEY,
    season_id uuid NOT NULL REFERENCES pass_seasons(id),
    track text NOT NULL CHECK (track IN ('free','paid')),
    xp bigint NOT NULL CHECK (xp > 0),
    grant_key text NOT NULL CHECK (grant_key ~ '^[a-z0-9][a-z0-9._:-]{0,63}$'),
    grant_revision text NOT NULL CHECK (grant_revision ~ '^[1-9][0-9]{0,19}$' AND grant_revision::numeric <= 18446744073709551615),
    UNIQUE (season_id, id)
);
CREATE TABLE pass_progress (
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    season_id uuid NOT NULL REFERENCES pass_seasons(id),
    xp bigint NOT NULL DEFAULT 0 CHECK (xp >= 0),
    paid boolean NOT NULL DEFAULT false,
    purchase_key uuid UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, season_id)
);
CREATE TABLE pass_challenge_progress (
    account_id uuid NOT NULL,
    season_id uuid NOT NULL,
    challenge_id uuid NOT NULL,
    amount bigint NOT NULL CHECK (amount > 0),
    PRIMARY KEY (account_id, season_id, challenge_id),
    FOREIGN KEY (account_id, season_id) REFERENCES pass_progress ON DELETE CASCADE,
    FOREIGN KEY (season_id, challenge_id) REFERENCES pass_challenges(season_id,id)
);
CREATE TABLE pass_events (
    account_id uuid NOT NULL,
    season_id uuid NOT NULL,
    challenge_id uuid NOT NULL,
    event_id uuid NOT NULL,
    amount bigint NOT NULL CHECK (amount > 0),
    occurred_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, challenge_id, event_id),
    FOREIGN KEY (account_id, season_id) REFERENCES pass_progress ON DELETE CASCADE,
    FOREIGN KEY (season_id, challenge_id) REFERENCES pass_challenges(season_id,id)
);
CREATE TABLE pass_claims (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL,
    season_id uuid NOT NULL,
    reward_id uuid NOT NULL,
    -- This is an enqueued entitlement operation, never proof of Game delivery.
    delivery_id uuid NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (account_id, season_id, reward_id),
    FOREIGN KEY (account_id, season_id) REFERENCES pass_progress ON DELETE CASCADE,
    FOREIGN KEY (season_id, reward_id) REFERENCES pass_rewards(season_id,id)
);
