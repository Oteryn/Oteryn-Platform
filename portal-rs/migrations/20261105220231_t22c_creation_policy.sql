-- Platform policy only; Game remains the authoritative name registry.
CREATE TABLE reserved_names (
    name text PRIMARY KEY CHECK (name = lower(name) AND length(name) BETWEEN 3 AND 29),
    expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

-- One bounded bucket per authenticated account and surface, shared across processes.
CREATE TABLE character_create_web_limits (
    account_id uuid NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    surface text NOT NULL CHECK (surface IN ('create', 'read')),
    window_start timestamptz NOT NULL,
    requests integer NOT NULL CHECK (requests > 0),
    PRIMARY KEY (account_id, surface)
);
