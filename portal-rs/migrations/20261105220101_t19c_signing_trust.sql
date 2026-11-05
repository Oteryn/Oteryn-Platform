-- Public trust only. Private seeds and encrypted custody files never enter SQL.
CREATE TABLE native_game_signing_trust_profiles (
    id uuid PRIMARY KEY,
    issuer text NOT NULL,
    profile text NOT NULL,
    key_purpose text NOT NULL CHECK (length(key_purpose) BETWEEN 1 AND 256),
    profile_version bigint NOT NULL CHECK (profile_version > 0),
    issuer_revision bigint NOT NULL CHECK (issuer_revision > 0),
    revoked_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    UNIQUE (issuer, profile, key_purpose, profile_version)
);
CREATE TABLE native_game_signing_trust_key_versions (
    profile_id uuid NOT NULL REFERENCES native_game_signing_trust_profiles(id),
    key_id text NOT NULL CHECK (key_id ~ '^[A-Za-z0-9._-]{1,64}$'),
    key_revision bigint NOT NULL CHECK (key_revision > 0),
    public_key text NOT NULL CHECK (public_key ~ '^[A-Za-z0-9_-]{43}$'),
    trusted boolean NOT NULL,
    trusted_at timestamptz NOT NULL,
    revoked_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (profile_id, key_id, key_revision),
    CHECK ((trusted AND revoked_at IS NULL) OR (NOT trusted AND revoked_at IS NOT NULL))
);
