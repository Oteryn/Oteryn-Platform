CREATE TABLE staff_invites (
    id uuid PRIMARY KEY,
    email text NOT NULL CHECK (length(email) BETWEEN 1 AND 254 AND email = lower(email)),
    role_ids uuid[] NOT NULL CHECK (cardinality(role_ids) BETWEEN 1 AND 32),
    invited_by uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    token_hash bytea NOT NULL UNIQUE CHECK (octet_length(token_hash) = 32),
    locale text NOT NULL CHECK (locale IN ('pl', 'en')),
    created_at timestamptz NOT NULL,
    expires_at timestamptz NOT NULL CHECK (expires_at > created_at),
    accepted_at timestamptz,
    accepted_by uuid REFERENCES accounts(id) ON DELETE CASCADE,
    revoked_at timestamptz,
    CHECK ((accepted_at IS NULL) = (accepted_by IS NULL)),
    CHECK (accepted_at IS NULL OR revoked_at IS NULL)
);

CREATE INDEX staff_invites_pending_idx ON staff_invites (id DESC)
    WHERE accepted_at IS NULL AND revoked_at IS NULL;
