CREATE TABLE credential_security_events (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    account_id uuid NOT NULL REFERENCES accounts(id),
    kind text NOT NULL CHECK (kind IN ('password_changed', 'sudo_confirmed')),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX credential_security_events_account ON credential_security_events(account_id, created_at);
