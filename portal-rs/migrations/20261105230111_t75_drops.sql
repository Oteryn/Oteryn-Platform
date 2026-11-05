-- Disposable owner-partitioned projections of complete, validated journal sessions.
-- Raw G-9 ingest, watermarks and authoritative ownership remain with the journal core.
CREATE TABLE pd_drop_sessions (
    account_id uuid NOT NULL,
    character_id uuid NOT NULL,
    session_id uuid NOT NULL,
    character_revision bigint NOT NULL CHECK (character_revision > 0),
    projection_revision bigint NOT NULL CHECK (projection_revision > 0),
    projection_epoch bigint NOT NULL CHECK (projection_epoch > 0),
    recovery_generation bigint NOT NULL CHECK (recovery_generation > 0),
    payload jsonb NOT NULL CHECK (jsonb_typeof(payload) = 'object'),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id, character_id, session_id)
);
CREATE INDEX pd_drop_sessions_owner ON pd_drop_sessions (account_id, character_id);
