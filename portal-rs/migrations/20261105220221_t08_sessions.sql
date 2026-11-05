-- Public identifiers are separate from bearer hashes and survive credential rotation.
ALTER TABLE web_sessions ADD COLUMN session_id uuid NOT NULL DEFAULT uuidv7();
CREATE UNIQUE INDEX web_sessions_public_id ON web_sessions(session_id);
CREATE INDEX web_sessions_account_activity ON web_sessions(account_id, last_seen_at DESC);

CREATE TABLE web_known_devices (
    account_id uuid NOT NULL REFERENCES accounts(id),
    fingerprint bytea NOT NULL CHECK (octet_length(fingerprint)=32),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, fingerprint)
);

-- No session foreign key: history must remain after logout and expiry cleanup.
CREATE TABLE security_events (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    account_id uuid NOT NULL REFERENCES accounts(id),
    session_id uuid,
    kind text NOT NULL CHECK (kind IN ('login_succeeded','session_revoked','other_sessions_revoked','all_sessions_revoked')),
    device varchar(256) NOT NULL DEFAULT '',
    ip inet,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX security_events_account_history ON security_events(account_id, created_at DESC, id DESC);
CREATE UNIQUE INDEX security_events_login_once ON security_events(session_id) WHERE kind='login_succeeded';
