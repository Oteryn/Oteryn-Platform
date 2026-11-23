CREATE TABLE stats_watch_access (
    session_id uuid PRIMARY KEY REFERENCES web_sessions(session_id) ON DELETE CASCADE,
    session_hash bytea NOT NULL CHECK (octet_length(session_hash) = 32),
    token_hash bytea NOT NULL UNIQUE CHECK (octet_length(token_hash) = 32),
    expires_at timestamptz NOT NULL
);
