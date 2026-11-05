CREATE TABLE account_passkeys (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    account_id uuid NOT NULL REFERENCES accounts(id),
    credential_id bytea NOT NULL UNIQUE,
    credential jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX account_passkeys_account ON account_passkeys(account_id);
-- Ceremony state is private server-side data; only random token digests are persisted.
CREATE TABLE passkey_ceremonies (
    token_hash bytea PRIMARY KEY CHECK (octet_length(token_hash) = 32),
    session_hash bytea NOT NULL CHECK (octet_length(session_hash) = 32),
    account_id uuid REFERENCES accounts(id),
    purpose text NOT NULL CHECK (purpose IN ('register', 'authenticate')),
    state jsonb NOT NULL,
    expires_at timestamptz NOT NULL DEFAULT now() + interval '5 minutes',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (session_hash, purpose)
);
CREATE INDEX passkey_ceremonies_expiry ON passkey_ceremonies(expires_at);
