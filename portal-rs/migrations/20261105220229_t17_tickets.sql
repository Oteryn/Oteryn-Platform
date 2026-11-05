-- OAuth and identity remain authoritative; only digests of bearer secrets persist.
CREATE TABLE game_login_tickets (
    ticket_hash bytea PRIMARY KEY CHECK (octet_length(ticket_hash) = 32),
    account_id uuid NOT NULL,
    game_generation text NOT NULL CHECK (game_generation ~ '^[0-9]+$'),
    native_generation text NOT NULL CHECK (native_generation ~ '^[1-9][0-9]*$'),
    audience text NOT NULL CHECK (audience = 'oteryn-native-game-gateway'),
    expires_at timestamptz NOT NULL,
    used_at timestamptz,
    attempt_ref uuid,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE TABLE game_ticket_budgets (
    credential_hash bytea PRIMARY KEY CHECK (octet_length(credential_hash) = 32),
    window_start timestamptz NOT NULL,
    requests bigint NOT NULL CHECK (requests > 0)
);
