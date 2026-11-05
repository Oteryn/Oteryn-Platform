-- Account identity is owned by identity tasks; deliberately no accounts foreign key.
CREATE TABLE native_admissions (
    attempt_ref uuid PRIMARY KEY,
    ticket_hash bytea NOT NULL UNIQUE CHECK (octet_length(ticket_hash) = 32),
    request_digest bytea NOT NULL CHECK (octet_length(request_digest) = 32),
    account_id uuid NOT NULL,
    response_body bytea NOT NULL,
    expires_at_ms bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
CREATE INDEX native_admissions_account_created ON native_admissions (account_id, created_at);

-- Shared across issuer processes; stores only credential digests, never credentials.
CREATE TABLE native_admission_budgets (
    credential_hash bytea PRIMARY KEY CHECK (octet_length(credential_hash) = 32),
    window_start timestamptz NOT NULL,
    requests bigint NOT NULL CHECK (requests > 0)
);
