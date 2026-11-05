-- Additive remembered-device history. Used hashes are retained for replay detection.
CREATE TABLE device_sessions (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES accounts(id),
    client_id uuid NOT NULL,
    enrollment_token_id text NOT NULL UNIQUE,
    game_generation bigint NOT NULL CHECK (game_generation >= 0),
    native_generation bigint NOT NULL CHECK (native_generation > 0),
    sequence bigint NOT NULL DEFAULT 0 CHECK (sequence >= 0),
    absolute_expires_at bigint NOT NULL,
    idle_expires_at bigint NOT NULL CHECK (idle_expires_at <= absolute_expires_at),
    revoked_at bigint,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX device_sessions_owner ON device_sessions(account_id);
CREATE TABLE device_session_credentials (
    secret_hash bytea PRIMARY KEY CHECK (octet_length(secret_hash) = 32),
    family_id uuid NOT NULL REFERENCES device_sessions(id),
    sequence bigint NOT NULL CHECK (sequence >= 0),
    consumed_at bigint,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (family_id, sequence)
);
CREATE TABLE device_session_source_limits (
    source_hash bytea PRIMARY KEY CHECK (octet_length(source_hash) = 32),
    window_start bigint NOT NULL,
    attempts integer NOT NULL CHECK (attempts > 0)
);
