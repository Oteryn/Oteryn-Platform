CREATE TABLE account_recovery_keys (
    account_id uuid PRIMARY KEY REFERENCES accounts(id),
    key_hash text NOT NULL CHECK (key_hash LIKE '$argon2id$%'),
    confirmed_at timestamptz,
    used_at timestamptz,
    revoked_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE account_recovery_key_events (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    account_id uuid NOT NULL REFERENCES accounts(id),
    kind text NOT NULL CHECK (kind IN ('generated', 'confirmed', 'revoked', 'used')),
    created_at timestamptz NOT NULL DEFAULT now()
);
