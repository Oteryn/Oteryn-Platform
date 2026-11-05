-- Latest pushed snapshots belong to the authenticated agent and ownership epoch.
CREATE TABLE analytics_cache (
    agent_id text NOT NULL REFERENCES admin_control_agents(agent_id),
    cache_key text NOT NULL CHECK (octet_length(cache_key) BETWEEN 1 AND 256),
    owner_epoch bigint NOT NULL CHECK (owner_epoch > 0),
    payload jsonb NOT NULL CHECK (jsonb_typeof(payload) = 'object'),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (agent_id, cache_key)
);
CREATE TABLE admin_control_health (
    agent_id text PRIMARY KEY REFERENCES admin_control_agents(agent_id),
    owner_epoch bigint NOT NULL CHECK (owner_epoch > 0),
    payload jsonb NOT NULL CHECK (jsonb_typeof(payload) = 'object'),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE admin_control_stream_chunks (
    request_id uuid NOT NULL REFERENCES admin_control_requests(request_id),
    chunk_index integer NOT NULL CHECK (chunk_index BETWEEN 0 AND 63),
    agent_id text NOT NULL REFERENCES admin_control_agents(agent_id),
    payload bytea NOT NULL CHECK (octet_length(payload) <= 1048576),
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (request_id, chunk_index)
);
