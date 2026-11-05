-- Opaque OAuth credentials are stored only as one-way digests.
CREATE TABLE oauth_authorization_codes (
    code_hash bytea PRIMARY KEY CHECK (octet_length(code_hash)=32),
    account_id uuid NOT NULL REFERENCES accounts(id),
    client_id text NOT NULL,
    redirect_uri text NOT NULL,
    challenge text NOT NULL,
    credential_hash bytea NOT NULL,
    expires_at timestamptz NOT NULL DEFAULT now()+interval '5 minutes',
    used_at timestamptz,
    revoked_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX oauth_codes_account ON oauth_authorization_codes(account_id);
CREATE TABLE oauth_token_families (
    id uuid PRIMARY KEY,
    account_id uuid NOT NULL REFERENCES accounts(id),
    client_id text NOT NULL,
    credential_hash bytea NOT NULL,
    expires_at timestamptz NOT NULL DEFAULT now()+interval '1 hour',
    revoked_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX oauth_families_account ON oauth_token_families(account_id);
CREATE TABLE oauth_tokens (
    id uuid PRIMARY KEY,
    family_id uuid NOT NULL REFERENCES oauth_token_families(id),
    access_hash bytea NOT NULL UNIQUE CHECK (octet_length(access_hash)=32),
    refresh_hash bytea NOT NULL UNIQUE CHECK (octet_length(refresh_hash)=32),
    access_expires_at timestamptz NOT NULL DEFAULT now()+interval '30 minutes',
    used_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX oauth_tokens_family ON oauth_tokens(family_id);
CREATE TABLE oauth_security_events (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    account_id uuid REFERENCES accounts(id),
    event text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);
