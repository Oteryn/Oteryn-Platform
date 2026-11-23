CREATE TABLE account_mfa_replacements (
    session_id uuid PRIMARY KEY REFERENCES web_sessions(session_id) ON DELETE CASCADE,
    factor_generation uuid NOT NULL,
    authorization_generation bigint NOT NULL,
    secret bytea NOT NULL,
    expires_at timestamptz NOT NULL DEFAULT (clock_timestamp() + interval '5 minutes')
);
CREATE INDEX account_mfa_replacements_expiry ON account_mfa_replacements(expires_at);
