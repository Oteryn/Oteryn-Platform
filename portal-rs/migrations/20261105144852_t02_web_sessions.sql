CREATE TABLE web_sessions (
    id_hash bytea PRIMARY KEY CHECK (octet_length(id_hash) = 32),
    account_id uuid,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    last_seen_at timestamptz NOT NULL DEFAULT now(),
    idle_expires_at timestamptz NOT NULL,
    absolute_expires_at timestamptz NOT NULL,
    ip inet,
    user_agent_summary varchar(256) NOT NULL,
    data jsonb NOT NULL DEFAULT '{}' CHECK (jsonb_typeof(data) = 'object'),
    csrf_token text NOT NULL CHECK (length(csrf_token) = 64),
    sudo_until timestamptz
);
CREATE INDEX web_sessions_idle_expiry ON web_sessions (idle_expires_at);
CREATE INDEX web_sessions_absolute_expiry ON web_sessions (absolute_expires_at);
