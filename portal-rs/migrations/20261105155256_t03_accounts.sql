CREATE TABLE accounts (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    email text NOT NULL CHECK (length(email) BETWEEN 3 AND 254),
    email_verified_at timestamptz,
    password_hash text NOT NULL,
    status text NOT NULL DEFAULT 'pending_verification' CHECK (status IN ('pending_verification', 'active', 'suspended', 'closed')),
    locale text NOT NULL CHECK (locale IN ('pl', 'en')),
    public_account_association boolean NOT NULL DEFAULT false,
    public_status_visible boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX accounts_email_unique ON accounts (lower(email));
CREATE TABLE account_email_tokens (
    token_hash bytea PRIMARY KEY CHECK (octet_length(token_hash) = 32),
    account_id uuid NOT NULL REFERENCES accounts(id),
    purpose text NOT NULL CHECK (purpose = 'verify_email'),
    expires_at timestamptz NOT NULL,
    used_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX account_email_tokens_account ON account_email_tokens(account_id);
CREATE TABLE account_consents (
    account_id uuid NOT NULL REFERENCES accounts(id),
    document_kind text NOT NULL CHECK (document_kind IN ('terms', 'privacy')),
    document_version text NOT NULL CHECK (length(document_version) BETWEEN 1 AND 128),
    accepted_at timestamptz NOT NULL DEFAULT now(),
    ip inet,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, document_kind, document_version)
);
